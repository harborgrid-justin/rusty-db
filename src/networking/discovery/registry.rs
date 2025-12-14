// Service Discovery Registry
//
// Central registry that aggregates multiple discovery backends and provides
// unified node discovery with change notifications.

use super::{
    DiscoveryConfig, DiscoveryEvent, DiscoveryState, HealthStatus, Node, ServiceDiscovery,
    SharedDiscoveryState,
};
use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;

/// Central discovery registry that aggregates multiple backends
pub struct Registry {
    /// Configuration
    config: DiscoveryConfig,

    /// Active discovery backends
    backends: Vec<Box<dyn ServiceDiscovery>>,

    /// Shared discovery state
    state: SharedDiscoveryState,

    /// Background refresh task handle
    refresh_handle: Option<tokio::task::JoinHandle<()>>,

    /// Event channel sender
    event_tx: Option<tokio::sync::mpsc::UnboundedSender<DiscoveryEvent>>,

    /// Event channel receiver
    event_rx: Option<tokio::sync::mpsc::UnboundedReceiver<DiscoveryEvent>>,

    /// Whether the registry is running
    running: Arc<RwLock<bool>>,
}

impl Registry {
    /// Creates a new discovery registry
    pub fn new(config: DiscoveryConfig) -> Self {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            config,
            backends: Vec::new(),
            state: Arc::new(RwLock::new(DiscoveryState::new())),
            refresh_handle: None,
            event_tx: Some(event_tx),
            event_rx: Some(event_rx),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initializes all configured discovery backends
    async fn initialize_backends(&mut self) -> Result<()> {
        tracing::info!("Initializing discovery backends");

        // DNS discovery
        if self.config.enable_dns {
            if let Some(dns_config) = self.config.dns_config.clone() {
                let mut backend = Box::new(super::dns::DnsDiscovery::new(dns_config));
                backend.initialize().await?;
                tracing::info!("Initialized DNS discovery backend");
                self.backends.push(backend);
            }
        }

        // Static list discovery
        if self.config.enable_static {
            if let Some(static_config) = self.config.static_config.clone() {
                let mut backend =
                    Box::new(super::static_list::StaticListDiscovery::new(static_config));
                backend.initialize().await?;
                tracing::info!("Initialized static list discovery backend");
                self.backends.push(backend);
            }
        }

        // Kubernetes discovery
        if self.config.enable_kubernetes {
            if let Some(k8s_config) = self.config.kubernetes_config.clone() {
                let mut backend = Box::new(super::kubernetes::KubernetesDiscovery::new(k8s_config));
                backend.initialize().await?;
                tracing::info!("Initialized Kubernetes discovery backend");
                self.backends.push(backend);
            }
        }

        // Consul discovery
        if self.config.enable_consul {
            if let Some(consul_config) = self.config.consul_config.clone() {
                let mut backend = Box::new(super::consul::ConsulDiscovery::new(consul_config));
                backend.initialize().await?;
                tracing::info!("Initialized Consul discovery backend");
                self.backends.push(backend);
            }
        }

        // etcd discovery
        if self.config.enable_etcd {
            if let Some(etcd_config) = self.config.etcd_config.clone() {
                let mut backend = Box::new(super::etcd::EtcdDiscovery::new(etcd_config));
                backend.initialize().await?;
                tracing::info!("Initialized etcd discovery backend");
                self.backends.push(backend);
            }
        }

        // Cloud discovery
        if self.config.enable_cloud {
            if let Some(cloud_config) = self.config.cloud_config.clone() {
                let mut backend = Box::new(super::cloud::CloudDiscovery::new(cloud_config)?);
                backend.initialize().await?;
                tracing::info!("Initialized cloud discovery backend");
                self.backends.push(backend);
            }
        }

        if self.backends.is_empty() {
            return Err(DbError::Configuration(
                "No discovery backends configured".to_string(),
            ));
        }

        tracing::info!("Initialized {} discovery backends", self.backends.len());
        Ok(())
    }

    /// Starts the discovery registry
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting service discovery registry");

        // Initialize backends
        self.initialize_backends().await?;

        // Register event listener
        if let Some(event_tx) = &self.event_tx {
            self.state.write().await.add_listener(event_tx.clone());
        }

        // Perform initial discovery
        self.refresh_nodes().await?;

        // Start background refresh task
        self.start_refresh_task();

        *self.running.write().await = true;

        tracing::info!("Service discovery registry started");
        Ok(())
    }

    /// Stops the discovery registry
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping service discovery registry");

        *self.running.write().await = false;

        // Cancel refresh task
        if let Some(handle) = self.refresh_handle.take() {
            handle.abort();
        }

        // Shutdown all backends
        for backend in &mut self.backends {
            backend.shutdown().await?;
        }

        self.backends.clear();

        tracing::info!("Service discovery registry stopped");
        Ok(())
    }

    /// Discovers nodes from all backends
    pub async fn discover_nodes(&self) -> Result<Vec<Node>> {
        let state = self.state.read().await;
        Ok(state.nodes.values().cloned().collect())
    }

    /// Gets healthy nodes only
    pub async fn get_healthy_nodes(&self) -> Result<Vec<Node>> {
        let state = self.state.read().await;
        Ok(state
            .nodes
            .values()
            .filter(|n| n.is_healthy())
            .cloned()
            .collect())
    }

    /// Gets a specific node by ID
    pub async fn get_node(&self, node_id: &str) -> Option<Node> {
        let state = self.state.read().await;
        state.nodes.get(node_id).cloned()
    }

    /// Registers a node with all backends that support registration
    pub async fn register_node(&self, node: &Node) -> Result<()> {
        tracing::info!("Registering node: {}", node.id);

        let mut errors = Vec::new();

        for backend in &self.backends {
            match backend.register_node(node).await {
                Ok(()) => {
                    tracing::debug!(
                        "Registered node {} with backend: {}",
                        node.id,
                        backend.name()
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to register node {} with backend {}: {}",
                        node.id,
                        backend.name(),
                        e
                    );
                    errors.push((backend.name().to_string(), e));
                }
            }
        }

        // Update local state
        self.state.write().await.add_node(node.clone());

        if !errors.is_empty() && errors.len() == self.backends.len() {
            return Err(DbError::Network(format!(
                "Failed to register node with any backend: {:?}",
                errors
            )));
        }

        Ok(())
    }

    /// Deregisters a node from all backends
    pub async fn deregister_node(&self, node_id: &str) -> Result<()> {
        tracing::info!("Deregistering node: {}", node_id);

        for backend in &self.backends {
            if let Err(e) = backend.deregister_node(node_id).await {
                tracing::warn!(
                    "Failed to deregister node {} from backend {}: {}",
                    node_id,
                    backend.name(),
                    e
                );
            }
        }

        // Remove from local state
        self.state.write().await.remove_node(node_id);

        Ok(())
    }

    /// Updates a node in all backends
    pub async fn update_node(&self, node: &Node) -> Result<()> {
        tracing::debug!("Updating node: {}", node.id);

        for backend in &self.backends {
            if let Err(e) = backend.update_node(node).await {
                tracing::warn!(
                    "Failed to update node {} in backend {}: {}",
                    node.id,
                    backend.name(),
                    e
                );
            }
        }

        // Update local state
        self.state.write().await.add_node(node.clone());

        Ok(())
    }

    /// Checks health of a specific node
    pub async fn health_check(&self, node_id: &str) -> Result<HealthStatus> {
        // Try to get health from any backend
        for backend in &self.backends {
            match backend.health_check(node_id).await {
                Ok(status) if status != HealthStatus::Unknown => return Ok(status),
                _ => continue,
            }
        }

        Ok(HealthStatus::Unknown)
    }

    /// Refreshes nodes from all backends
    async fn refresh_nodes(&self) -> Result<()> {
        tracing::debug!("Refreshing nodes from all backends");

        let mut all_nodes = HashMap::new();

        // Collect nodes from all backends
        for backend in &self.backends {
            match backend.discover_nodes().await {
                Ok(nodes) => {
                    tracing::debug!(
                        "Backend {} discovered {} nodes",
                        backend.name(),
                        nodes.len()
                    );

                    for node in nodes {
                        // Merge nodes with same ID (prefer newer data)
                        all_nodes
                            .entry(node.id.clone())
                            .and_modify(|existing: &mut Node| {
                                if node.last_seen > existing.last_seen {
                                    *existing = node.clone();
                                }
                            })
                            .or_insert(node);
                    }
                }
                Err(e) => {
                    tracing::warn!("Backend {} discovery failed: {}", backend.name(), e);
                }
            }
        }

        // Update state with discovered nodes
        let mut state = self.state.write().await;

        // Remove stale nodes
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let node_ttl_secs = self.config.node_ttl.as_secs();
        let stale_nodes: Vec<String> = state
            .nodes
            .iter()
            .filter(|(_, node)| now - node.last_seen > node_ttl_secs)
            .map(|(id, _)| id.clone())
            .collect();

        for node_id in stale_nodes {
            tracing::info!("Removing stale node: {}", node_id);
            state.remove_node(&node_id);
        }

        // Add or update discovered nodes
        for (_, node) in all_nodes {
            state.add_node(node);
        }

        tracing::info!(
            "Discovery refresh completed. Total nodes: {}",
            state.nodes.len()
        );
        Ok(())
    }

    /// Starts background refresh task
    fn start_refresh_task(&mut self) {
        let state = Arc::clone(&self.state);
        let running = Arc::clone(&self.running);
        let refresh_interval = self.config.refresh_interval;
        let backends = self.backends.len();
        let node_ttl = self.config.node_ttl;

        // Create a minimal context for the refresh task
        let _backend_names: Vec<String> =
            self.backends.iter().map(|b| b.name().to_string()).collect();

        let handle = tokio::spawn(async move {
            let mut ticker = time::interval(refresh_interval);

            loop {
                ticker.tick().await;

                // Check if still running
                if !*running.read().await {
                    break;
                }

                // The actual refresh is done by the main registry
                // This task just triggers periodic cleanup
                tracing::debug!("Background refresh tick (backends: {})", backends);

                // Clean up stale nodes
                let mut state_guard = state.write().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let stale_nodes: Vec<String> = state_guard
                    .nodes
                    .iter()
                    .filter(|(_, node)| now - node.last_seen > node_ttl.as_secs())
                    .map(|(id, _)| id.clone())
                    .collect();

                for node_id in stale_nodes {
                    tracing::debug!("Background cleanup: removing stale node {}", node_id);
                    state_guard.remove_node(&node_id);
                }
            }

            tracing::debug!("Background refresh task stopped");
        });

        self.refresh_handle = Some(handle);
    }

    /// Gets an event receiver for discovery events
    pub fn event_receiver(
        &mut self,
    ) -> Option<tokio::sync::mpsc::UnboundedReceiver<DiscoveryEvent>> {
        self.event_rx.take()
    }

    /// Returns the number of active backends
    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }

    /// Returns the names of active backends
    pub fn backend_names(&self) -> Vec<String> {
        self.backends.iter().map(|b| b.name().to_string()).collect()
    }

    /// Checks if the registry is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[tokio::test]
    async fn test_registry_creation() {
        let config = DiscoveryConfig::default();
        let registry = Registry::new(config);

        assert_eq!(registry.backend_count(), 0);
        assert!(!registry.is_running().await);
    }

    #[tokio::test]
    async fn test_registry_start_stop() {
        let config = DiscoveryConfig::default();
        let mut registry = Registry::new(config);

        let result = registry.start().await;
        assert!(result.is_ok());
        assert!(registry.is_running().await);
        assert!(registry.backend_count() > 0);

        let stop_result = registry.stop().await;
        assert!(stop_result.is_ok());
        assert!(!registry.is_running().await);
    }

    #[tokio::test]
    async fn test_node_registration() {
        let config = DiscoveryConfig::default();
        let mut registry = Registry::new(config);

        registry.start().await.expect("Failed to start registry");

        let node = Node::new(
            "test-node-1".to_string(),
            "127.0.0.1".parse::<IpAddr>().unwrap(),
            5432,
        );

        let result = registry.register_node(&node).await;
        assert!(result.is_ok());

        let discovered = registry.get_node("test-node-1").await;
        assert!(discovered.is_some());

        registry.stop().await.expect("Failed to stop registry");
    }

    #[tokio::test]
    async fn test_healthy_nodes_filter() {
        let config = DiscoveryConfig::default();
        let mut registry = Registry::new(config);

        registry.start().await.expect("Failed to start registry");

        let mut healthy_node = Node::new(
            "healthy-node".to_string(),
            "127.0.0.1".parse::<IpAddr>().unwrap(),
            5432,
        );
        healthy_node.health = HealthStatus::Healthy;

        let mut unhealthy_node = Node::new(
            "unhealthy-node".to_string(),
            "127.0.0.2".parse::<IpAddr>().unwrap(),
            5432,
        );
        unhealthy_node.health = HealthStatus::Unhealthy;

        registry.register_node(&healthy_node).await.unwrap();
        registry.register_node(&unhealthy_node).await.unwrap();

        let healthy_nodes = registry.get_healthy_nodes().await.unwrap();
        assert_eq!(healthy_nodes.len(), 1);
        assert_eq!(healthy_nodes[0].id, "healthy-node");

        registry.stop().await.expect("Failed to stop registry");
    }
}
