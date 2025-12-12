// Static Seed List Discovery
//
// Provides discovery based on a pre-configured list of seed nodes.
// Supports file-based configuration with hot reload capabilities.

use super::{HealthStatus, Node, ServiceDiscovery, StaticConfig};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time;

/// Static seed list discovery
pub struct StaticListDiscovery {
    /// Configuration
    config: StaticConfig,

    /// Parsed nodes from seed list
    nodes: Arc<RwLock<Vec<Node>>>,

    /// Last modification time of seed file
    last_modified: Arc<RwLock<Option<SystemTime>>>,

    /// Background reload task handle
    reload_handle: Option<tokio::task::JoinHandle<()>>,
}

impl StaticListDiscovery {
    /// Creates a new static list discovery instance
    pub fn new(config: StaticConfig) -> Self {
        Self {
            config,
            nodes: Arc::new(RwLock::new(Vec::new())),
            last_modified: Arc::new(RwLock::new(None)),
            reload_handle: None,
        }
    }

    /// Parses a seed node string (format: "host:port")
    fn parse_seed_node(&self, seed: &str) -> Result<Vec<Node>> {
        let addr_str = seed.trim();

        // Parse socket address
        let socket_addrs: Vec<SocketAddr> = addr_str
            .to_socket_addrs()
            .map_err(|e| {
                DbError::Configuration(format!("Invalid seed address '{}': {}", addr_str, e))
            })?
            .collect();

        if socket_addrs.is_empty() {
            return Err(DbError::Configuration(format!(
                "Could not resolve seed address: {}",
                addr_str
            )));
        }

        let mut nodes = Vec::new();
        for socket_addr in socket_addrs {
            let node = Node {
                id: socket_addr.to_string(),
                address: socket_addr.ip(),
                port: socket_addr.port(),
                datacenter: None,
                rack: None,
                health: HealthStatus::Unknown,
                metadata: HashMap::new(),
                last_seen: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Loads nodes from configuration
    async fn load_nodes(&self) -> Result<Vec<Node>> {
        let mut all_nodes = Vec::new();

        // Load from configured seed nodes
        for seed in &self.config.seed_nodes {
            match self.parse_seed_node(seed) {
                Ok(mut nodes) => all_nodes.append(&mut nodes),
                Err(e) => {
                    tracing::warn!("Failed to parse seed node '{}': {}", seed, e);
                }
            }
        }

        // Load from seed file if configured
        if let Some(ref seed_file) = self.config.seed_file {
            match self.load_from_file(seed_file).await {
                Ok(mut file_nodes) => all_nodes.append(&mut file_nodes),
                Err(e) => {
                    tracing::warn!("Failed to load seed file '{}': {}", seed_file, e);
                }
            }
        }

        if all_nodes.is_empty() {
            return Err(DbError::Configuration(
                "No valid seed nodes configured".to_string(),
            ));
        }

        Ok(all_nodes)
    }

    /// Loads nodes from a file
    async fn load_from_file(&self, path: &str) -> Result<Vec<Node>> {
        let file_path = PathBuf::from(path);

        // Check if file exists
        if !file_path.exists() {
            return Err(DbError::NotFound(format!(
                "Seed file not found: {}",
                path
            )));
        }

        // Read file contents
        let contents = fs::read_to_string(&file_path)
            .await
            .map_err(|e| DbError::Io(e))?;

        let mut nodes = Vec::new();

        // Parse each line as a seed node
        for line in contents.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match self.parse_seed_node(line) {
                Ok(mut parsed_nodes) => nodes.append(&mut parsed_nodes),
                Err(e) => {
                    tracing::warn!("Invalid seed node in file '{}': {}", line, e);
                }
            }
        }

        // Update last modified time
        if let Ok(metadata) = fs::metadata(&file_path).await {
            if let Ok(modified) = metadata.modified() {
                *self.last_modified.write().await = Some(modified);
            }
        }

        Ok(nodes)
    }

    /// Checks if seed file has been modified
    async fn check_file_modified(&self) -> bool {
        if let Some(ref seed_file) = self.config.seed_file {
            if let Ok(metadata) = fs::metadata(seed_file).await {
                if let Ok(modified) = metadata.modified() {
                    let last_mod = self.last_modified.read().await;
                    if let Some(last) = *last_mod {
                        return modified > last;
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Starts background hot reload task
    fn start_hot_reload(&mut self) {
        if !self.config.hot_reload || self.config.seed_file.is_none() {
            return;
        }

        let config = self.config.clone();
        let nodes = Arc::clone(&self.nodes);
        let last_modified = Arc::clone(&self.last_modified);
        let interval = self.config.reload_interval;

        let handle = tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            let discovery = StaticListDiscovery {
                config,
                nodes: Arc::clone(&nodes),
                last_modified,
                reload_handle: None,
            };

            loop {
                ticker.tick().await;

                // Check if file has been modified
                if discovery.check_file_modified().await {
                    tracing::info!("Seed file modified, reloading nodes");

                    match discovery.load_nodes().await {
                        Ok(new_nodes) => {
                            *nodes.write().await = new_nodes;
                            tracing::info!("Successfully reloaded seed nodes");
                        }
                        Err(e) => {
                            tracing::error!("Failed to reload seed nodes: {}", e);
                        }
                    }
                }
            }
        });

        self.reload_handle = Some(handle);
    }
}

#[async_trait]
impl ServiceDiscovery for StaticListDiscovery {
    async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing static seed list discovery");

        // Validate configuration
        if self.config.seed_nodes.is_empty() && self.config.seed_file.is_none() {
            return Err(DbError::Configuration(
                "No seed nodes or seed file configured".to_string(),
            ));
        }

        // Load initial nodes
        let nodes = self.load_nodes().await?;
        *self.nodes.write().await = nodes;

        tracing::info!(
            "Loaded {} seed nodes",
            self.nodes.read().await.len()
        );

        // Start hot reload if enabled
        self.start_hot_reload();

        Ok(())
    }

    async fn discover_nodes(&self) -> Result<Vec<Node>> {
        let nodes = self.nodes.read().await;

        if nodes.is_empty() {
            return Err(DbError::NotFound("No seed nodes available".to_string()));
        }

        // Clone and update last seen timestamp
        let mut discovered_nodes: Vec<Node> = nodes.clone();
        for node in &mut discovered_nodes {
            node.touch();
        }

        tracing::debug!("Returning {} static seed nodes", discovered_nodes.len());
        Ok(discovered_nodes)
    }

    async fn register_node(&self, _node: &Node) -> Result<()> {
        // Static discovery doesn't support dynamic registration
        tracing::debug!("Static discovery does not support node registration");
        Ok(())
    }

    async fn deregister_node(&self, _node_id: &str) -> Result<()> {
        // Static discovery doesn't support dynamic deregistration
        tracing::debug!("Static discovery does not support node deregistration");
        Ok(())
    }

    async fn update_node(&self, _node: &Node) -> Result<()> {
        // Static discovery doesn't support dynamic updates
        tracing::debug!("Static discovery does not support node updates");
        Ok(())
    }

    async fn health_check(&self, _node_id: &str) -> Result<HealthStatus> {
        // Static discovery doesn't provide health information
        Ok(HealthStatus::Unknown)
    }

    async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down static seed list discovery");

        // Cancel hot reload task
        if let Some(handle) = self.reload_handle.take() {
            handle.abort();
        }

        // Clear nodes
        self.nodes.write().await.clear();

        Ok(())
    }

    fn name(&self) -> &str {
        "static"
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;

    #[tokio::test]
    async fn test_static_discovery_with_seeds() {
        let config = StaticConfig {
            seed_nodes: vec!["127.0.0.1:5432".to_string(), "localhost:5433".to_string()],
            seed_file: None,
            hot_reload: false,
            reload_interval: Duration::from_secs(60),
        };

        let mut discovery = StaticListDiscovery::new(config);
        assert!(discovery.initialize().await.is_ok());

        let nodes = discovery.discover_nodes().await;
        assert!(nodes.is_ok());
        let nodes = nodes.unwrap();
        assert!(!nodes.is_empty());

        assert!(discovery.shutdown().await.is_ok());
    }

    #[test]
    fn test_parse_seed_node() {
        let config = StaticConfig::default();
        let discovery = StaticListDiscovery::new(config);

        let result = discovery.parse_seed_node("127.0.0.1:5432");
        assert!(result.is_ok());

        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].port, 5432);
    }

    #[tokio::test]
    async fn test_empty_configuration_fails() {
        let config = StaticConfig {
            seed_nodes: vec![],
            seed_file: None,
            hot_reload: false,
            reload_interval: Duration::from_secs(60),
        };

        let mut discovery = StaticListDiscovery::new(config);
        let result = discovery.initialize().await;
        assert!(result.is_err());
    }
}
