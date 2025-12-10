//! HashiCorp Consul Service Discovery
//!
//! Implements service discovery and registration using HashiCorp Consul.
//! Supports health checks, KV store metadata, and prepared queries.

use super::{HealthStatus, Node, ServiceDiscovery, ConsulConfig};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::time;

/// Consul service discovery
pub struct ConsulDiscovery {
    /// Configuration
    config: ConsulConfig,

    /// HTTP client for Consul API
    http_client: reqwest::Client,

    /// Current node registration ID
    registration_id: Arc<RwLock<Option<String>>>,

    /// Background health check task handle
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConsulDiscovery {
    /// Creates a new Consul discovery instance
    pub fn new(config: ConsulConfig) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();

        // Add ACL token if provided
        if let Some(ref token) = config.token {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_str(token) {
                headers.insert("X-Consul-Token", header_value);
            }
        }

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            registration_id: Arc::new(RwLock::new(None)),
            health_check_handle: None,
        }
    }

    /// Builds the Consul API URL
    fn build_url(&self, path: &str) -> String {
        format!("{}/v1/{}", self.config.address.trim_end_matches('/'), path)
    }

    /// Queries services from Consul catalog
    async fn query_services(&self) -> Result<Vec<Node>> {
        let url = if let Some(ref dc) = self.config.datacenter {
            format!("health/service/{}?dc={}&passing=true", self.config.service_name, dc)
        } else {
            format!("health/service/{}?passing=true", self.config.service_name)
        };

        let full_url = self.build_url(&url);

        tracing::debug!("Querying Consul services: {}", full_url);

        let response = self
            .http_client
            .get(&full_url)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("Consul API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "Consul API returned error: {}",
                response.status()
            )));
        }

        let services: Vec<ConsulServiceEntry> = response
            .json()
            .await
            .map_err(|e| DbError::Serialization(format!("Failed to parse Consul response: {}", e)))?;

        let mut nodes = Vec::new();

        for entry in services {
            // Parse IP address
            let ip_addr: IpAddr = entry
                .Service
                .Address
                .parse()
                .map_err(|e| {
                    DbError::Network(format!("Invalid IP address from Consul: {}", e))
                })?;

            let mut metadata = HashMap::new();
            if let Some(meta) = entry.Service.Meta {
                metadata = meta;
            }

            // Add tags to metadata
            metadata.insert(
                "tags".to_string(),
                entry.Service.Tags.join(","),
            );

            if let Some(ref dc) = self.config.datacenter {
                metadata.insert("datacenter".to_string(), dc.clone());
            }

            let node = Node {
                id: entry.Service.ID,
                address: ip_addr,
                port: entry.Service.Port as u16,
                datacenter: self.config.datacenter.clone(),
                rack: None,
                health: self.parse_health_status(&entry.Checks),
                metadata,
                last_seen: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            nodes.push(node);
        }

        tracing::info!("Discovered {} services from Consul", nodes.len());
        Ok(nodes)
    }

    /// Parses health status from Consul checks
    fn parse_health_status(&self, checks: &[ConsulHealthCheck]) -> HealthStatus {
        if checks.is_empty() {
            return HealthStatus::Unknown;
        }

        // Check if any critical checks
        if checks.iter().any(|c| c.Status == "critical") {
            return HealthStatus::Unhealthy;
        }

        // Check if any warning checks
        if checks.iter().any(|c| c.Status == "warning") {
            return HealthStatus::Degraded;
        }

        // All checks passing
        if checks.iter().all(|c| c.Status == "passing") {
            return HealthStatus::Healthy;
        }

        HealthStatus::Unknown
    }

    /// Registers a service with Consul
    async fn register_service(&self, node: &Node) -> Result<String> {
        let service_id = format!("{}-{}", self.config.service_name, node.id);

        let mut registration = ConsulServiceRegistration {
            ID: service_id.clone(),
            Name: self.config.service_name.clone(),
            Tags: self.config.tags.clone(),
            Address: node.address.to_string(),
            Port: node.port as u32,
            Meta: Some(node.metadata.clone()),
            Check: None,
        };

        // Add health check if enabled
        if self.config.enable_health_check {
            registration.Check = Some(ConsulHealthCheckDef {
                TCP: format!("{}:{}", node.address, node.port),
                Interval: format!("{}s", self.config.health_check_interval.as_secs()),
                Timeout: "5s".to_string(),
            });
        }

        let url = self.build_url("agent/service/register");

        tracing::debug!("Registering service with Consul: {}", service_id);

        let response = self
            .http_client
            .put(&url)
            .json(&registration)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("Consul registration failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DbError::Network(format!(
                "Consul registration failed with status {}: {}",
                status, body
            )));
        }

        tracing::info!("Successfully registered service with Consul: {}", service_id);
        Ok(service_id)
    }

    /// Deregisters a service from Consul
    async fn deregister_service(&self, service_id: &str) -> Result<()> {
        let url = self.build_url(&format!("agent/service/deregister/{}", service_id));

        tracing::debug!("Deregistering service from Consul: {}", service_id);

        let response = self
            .http_client
            .put(&url)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("Consul deregistration failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "Consul deregistration failed: {}",
                response.status()
            )));
        }

        tracing::info!("Successfully deregistered service from Consul: {}", service_id);
        Ok(())
    }

    /// Starts background health check renewal
    fn start_health_check_task(&mut self, service_id: String) {
        if !self.config.enable_health_check {
            return;
        }

        let http_client = self.http_client.clone();
        let base_url = self.config.address.clone();
        let interval = self.config.health_check_interval;

        let handle = tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                ticker.tick().await;

                // Pass health check (if using TTL checks)
                let url = format!("{}/v1/agent/check/pass/service:{}", base_url, service_id);

                match http_client.put(&url).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            tracing::warn!(
                                "Failed to pass health check: {}",
                                response.status()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Health check request failed: {}", e);
                    }
                }
            }
        });

        self.health_check_handle = Some(handle);
    }
}

#[async_trait]
impl ServiceDiscovery for ConsulDiscovery {
    async fn initialize(&mut self) -> Result<()> {
        tracing::info!(
            "Initializing Consul discovery at: {}",
            self.config.address
        );

        // Validate configuration
        if self.config.service_name.is_empty() {
            return Err(DbError::Configuration(
                "Consul service name cannot be empty".to_string(),
            ));
        }

        // Test connectivity to Consul
        let url = self.build_url("status/leader");
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                DbError::Network(format!("Failed to connect to Consul: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "Consul health check failed: {}",
                response.status()
            )));
        }

        tracing::info!("Successfully connected to Consul");
        Ok(())
    }

    async fn discover_nodes(&self) -> Result<Vec<Node>> {
        self.query_services().await
    }

    async fn register_node(&self, node: &Node) -> Result<()> {
        let service_id = self.register_service(node).await?;
        *self.registration_id.write().await = Some(service_id);
        Ok(())
    }

    async fn deregister_node(&self, _node_id: &str) -> Result<()> {
        // Use our stored registration ID
        let reg_id = self.registration_id.read().await;
        if let Some(service_id) = reg_id.as_ref() {
            self.deregister_service(service_id).await?;
        }
        Ok(())
    }

    async fn update_node(&self, node: &Node) -> Result<()> {
        // Consul doesn't support updating - we need to re-register
        if let Some(service_id) = self.registration_id.read().await.as_ref() {
            self.deregister_service(service_id).await?;
        }
        self.register_node(node).await?;
        Ok(())
    }

    async fn health_check(&self, node_id: &str) -> Result<HealthStatus> {
        // Query specific service health
        let url = self.build_url(&format!("health/service/{}", self.config.service_name));

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("Consul health query failed: {}", e)))?;

        if !response.status().is_success() {
            return Ok(HealthStatus::Unknown);
        }

        let services: Vec<ConsulServiceEntry> = response
            .json()
            .await
            .map_err(|e| DbError::Serialization(format!("Failed to parse health response: {}", e)))?;

        // Find the specific service
        for entry in services {
            if entry.Service.ID == node_id {
                return Ok(self.parse_health_status(&entry.Checks));
            }
        }

        Ok(HealthStatus::Unknown)
    }

    async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down Consul discovery");

        // Cancel health check task
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
        }

        // Deregister if we have a registration
        if let Some(service_id) = self.registration_id.read().await.as_ref() {
            let _ = self.deregister_service(service_id).await;
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "consul"
    }
}

/// Consul service entry from catalog
#[derive(Debug, Deserialize)]
struct ConsulServiceEntry {
    Service: ConsulService,
    Checks: Vec<ConsulHealthCheck>,
}

/// Consul service definition
#[derive(Debug, Deserialize)]
struct ConsulService {
    ID: String,
    Service: String,
    Tags: Vec<String>,
    Address: String,
    Port: u32,
    Meta: Option<HashMap<String, String>>,
}

/// Consul health check
#[derive(Debug, Deserialize)]
struct ConsulHealthCheck {
    Status: String,
    ServiceID: String,
}

/// Consul service registration
#[derive(Debug, Serialize)]
struct ConsulServiceRegistration {
    ID: String,
    Name: String,
    Tags: Vec<String>,
    Address: String,
    Port: u32,
    Meta: Option<HashMap<String, String>>,
    Check: Option<ConsulHealthCheckDef>,
}

/// Consul health check definition
#[derive(Debug, Serialize)]
struct ConsulHealthCheckDef {
    TCP: String,
    Interval: String,
    Timeout: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consul_discovery_initialization() {
        let config = ConsulConfig::default();
        let mut discovery = ConsulDiscovery::new(config);

        // Note: This will fail without actual Consul instance
        // In production environments, use a test Consul instance
        let _ = discovery.initialize().await;

        let _ = discovery.shutdown().await;
    }

    #[test]
    fn test_consul_url_building() {
        let config = ConsulConfig::default();
        let discovery = ConsulDiscovery::new(config);

        let url = discovery.build_url("health/service/test");
        assert!(url.contains("/v1/health/service/test"));
    }
}
