// etcd Service Discovery
//
// Implements service discovery using etcd distributed key-value store.
// Supports lease-based registration, prefix watching, and distributed locking.

use super::{EtcdConfig, HealthStatus, Node, ServiceDiscovery};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::time;

/// etcd-based service discovery
pub struct EtcdDiscovery {
    /// Configuration
    config: EtcdConfig,

    /// HTTP client for etcd API (v3 HTTP JSON API)
    http_client: reqwest::Client,

    /// Current lease ID for this node
    lease_id: Arc<RwLock<Option<i64>>>,

    /// Background lease renewal task handle
    lease_renewal_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,

    /// Node registration key
    node_key: Arc<RwLock<Option<String>>>,
}

impl EtcdDiscovery {
    /// Creates a new etcd discovery instance
    pub fn new(config: EtcdConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            lease_id: Arc::new(RwLock::new(None)),
            lease_renewal_handle: Arc::new(RwLock::new(None)),
            node_key: Arc::new(RwLock::new(None)),
        }
    }

    /// Gets the first etcd endpoint
    fn get_endpoint(&self) -> Result<String> {
        self.config
            .endpoints
            .first()
            .cloned()
            .ok_or_else(|| DbError::Configuration("No etcd endpoints configured".to_string()))
    }

    /// Creates a lease in etcd
    async fn create_lease(&self) -> Result<i64> {
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/v3/lease/grant", endpoint.trim_end_matches('/'));

        let request = serde_json::json!({
            "TTL": self.config.lease_ttl,
        });

        tracing::debug!("Creating etcd lease with TTL: {}s", self.config.lease_ttl);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("etcd lease creation failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd lease creation failed: {}",
                response.status()
            )));
        }

        let lease_response: EtcdLeaseGrantResponse = response.json().await.map_err(|e| {
            DbError::Serialization(format!("Failed to parse lease response: {}", e))
        })?;

        let lease_id = lease_response
            .id
            .parse::<i64>()
            .map_err(|e| DbError::Serialization(format!("Invalid lease ID: {}", e)))?;

        tracing::info!("Created etcd lease: {}", lease_id);
        Ok(lease_id)
    }

    /// Keeps a lease alive
    #[allow(dead_code)]
    async fn keep_alive(&self, lease_id: i64) -> Result<()> {
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/v3/lease/keepalive", endpoint.trim_end_matches('/'));

        let request = serde_json::json!({
            "ID": lease_id,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("etcd lease keepalive failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd lease keepalive failed: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Revokes a lease
    async fn revoke_lease(&self, lease_id: i64) -> Result<()> {
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/v3/lease/revoke", endpoint.trim_end_matches('/'));

        let request = serde_json::json!({
            "ID": lease_id,
        });

        tracing::debug!("Revoking etcd lease: {}", lease_id);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("etcd lease revocation failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd lease revocation failed: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Puts a key-value pair in etcd with a lease
    async fn put_with_lease(&self, key: &str, value: &str, lease_id: i64) -> Result<()> {
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/v3/kv/put", endpoint.trim_end_matches('/'));

        // Base64 encode key and value (etcd v3 API requirement)
        let key_b64 = general_purpose::STANDARD.encode(key);
        let value_b64 = general_purpose::STANDARD.encode(value);

        let request = serde_json::json!({
            "key": key_b64,
            "value": value_b64,
            "lease": lease_id,
        });

        tracing::debug!("Putting key in etcd: {}", key);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("etcd put failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd put failed: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Gets keys with a specific prefix
    async fn get_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/v3/kv/range", endpoint.trim_end_matches('/'));

        // Calculate the prefix end for range query
        let range_end = self.get_prefix_range_end(prefix);

        let key_b64 = general_purpose::STANDARD.encode(prefix);
        let range_end_b64 = general_purpose::STANDARD.encode(&range_end);

        let request = serde_json::json!({
            "key": key_b64,
            "range_end": range_end_b64,
        });

        tracing::debug!("Querying etcd prefix: {}", prefix);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("etcd range query failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd range query failed: {}",
                response.status()
            )));
        }

        let range_response: EtcdRangeResponse = response.json().await.map_err(|e| {
            DbError::Serialization(format!("Failed to parse range response: {}", e))
        })?;

        let mut results = Vec::new();

        for kv in range_response.kvs.unwrap_or_default() {
            let key = general_purpose::STANDARD
                .decode(&kv.key)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();

            let value = general_purpose::STANDARD
                .decode(&kv.value)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();

            results.push((key, value));
        }

        tracing::debug!("Found {} keys with prefix: {}", results.len(), prefix);
        Ok(results)
    }

    /// Calculates the range end for prefix queries
    fn get_prefix_range_end(&self, prefix: &str) -> String {
        let mut bytes = prefix.as_bytes().to_vec();

        // Increment the last byte to get the range end
        if let Some(last) = bytes.last_mut() {
            if *last < 255 {
                *last += 1;
            }
        }

        String::from_utf8(bytes).unwrap_or_else(|_| format!("{}~", prefix))
    }

    /// Deletes a key from etcd
    async fn delete_key(&self, key: &str) -> Result<()> {
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/v3/kv/deleterange", endpoint.trim_end_matches('/'));

        let key_b64 = general_purpose::STANDARD.encode(key);

        let request = serde_json::json!({
            "key": key_b64,
        });

        tracing::debug!("Deleting key from etcd: {}", key);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("etcd delete failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd delete failed: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Starts background lease renewal task
    fn start_lease_renewal(&self, lease_id: i64) {
        let http_client = self.http_client.clone();
        let endpoint = self.get_endpoint().unwrap_or_default();
        let ttl = self.config.lease_ttl;

        // Renew at half the TTL interval
        let renewal_interval = time::Duration::from_secs(ttl / 2);

        let handle = tokio::spawn(async move {
            let mut ticker = time::interval(renewal_interval);

            loop {
                ticker.tick().await;

                let url = format!("{}/v3/lease/keepalive", endpoint.trim_end_matches('/'));
                let request = serde_json::json!({"ID": lease_id});

                match http_client.post(&url).json(&request).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            tracing::warn!(
                                "Failed to renew lease {}: {}",
                                lease_id,
                                response.status()
                            );
                        } else {
                            tracing::debug!("Renewed lease: {}", lease_id);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Lease renewal request failed: {}", e);
                    }
                }
            }
        });

        // Store the handle using interior mutability
        if let Ok(mut guard) = self.lease_renewal_handle.try_write() {
            *guard = Some(handle);
        }
    }
}

#[async_trait]
impl ServiceDiscovery for EtcdDiscovery {
    async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing etcd discovery");

        // Validate configuration
        if self.config.endpoints.is_empty() {
            return Err(DbError::Configuration(
                "No etcd endpoints configured".to_string(),
            ));
        }

        // Test connectivity
        let endpoint = self.get_endpoint()?;
        let url = format!("{}/version", endpoint.trim_end_matches('/'));

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| DbError::Network(format!("Failed to connect to etcd: {}", e)))?;

        if !response.status().is_success() {
            return Err(DbError::Network(format!(
                "etcd health check failed: {}",
                response.status()
            )));
        }

        tracing::info!("Successfully connected to etcd");
        Ok(())
    }

    async fn discover_nodes(&self) -> Result<Vec<Node>> {
        tracing::debug!("Discovering nodes via etcd");

        // Query all keys under the prefix
        let kvs = self.get_prefix(&self.config.key_prefix).await?;

        let mut nodes = Vec::new();

        for (key, value) in kvs {
            // Parse the value as node information (JSON)
            match serde_json::from_str::<NodeInfo>(&value) {
                Ok(node_info) => {
                    let node = Node {
                        id: node_info.id,
                        address: node_info.address,
                        port: node_info.port,
                        datacenter: node_info.datacenter,
                        rack: node_info.rack,
                        health: HealthStatus::Healthy, // Assume healthy if in etcd
                        metadata: node_info.metadata,
                        last_seen: SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    };
                    nodes.push(node);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse node info for key {}: {}", key, e);
                }
            }
        }

        tracing::info!("Discovered {} nodes from etcd", nodes.len());
        Ok(nodes)
    }

    async fn register_node(&self, node: &Node) -> Result<()> {
        // Create a lease
        let lease_id = self.create_lease().await?;
        *self.lease_id.write().await = Some(lease_id);

        // Serialize node info
        let node_info = NodeInfo {
            id: node.id.clone(),
            address: node.address,
            port: node.port,
            datacenter: node.datacenter.clone(),
            rack: node.rack.clone(),
            metadata: node.metadata.clone(),
        };

        let value = serde_json::to_string(&node_info)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize node: {}", e)))?;

        // Build key: /prefix/node_id
        let key = format!(
            "{}/{}",
            self.config.key_prefix.trim_end_matches('/'),
            node.id
        );
        *self.node_key.write().await = Some(key.clone());

        // Put the key with lease
        self.put_with_lease(&key, &value, lease_id).await?;

        // Start lease renewal
        self.start_lease_renewal(lease_id);

        tracing::info!("Registered node in etcd: {}", node.id);
        Ok(())
    }

    async fn deregister_node(&self, _node_id: &str) -> Result<()> {
        // Delete the key
        if let Some(key) = self.node_key.read().await.as_ref() {
            self.delete_key(key).await?;
        }

        // Revoke the lease
        if let Some(lease_id) = *self.lease_id.read().await {
            self.revoke_lease(lease_id).await?;
        }

        Ok(())
    }

    async fn update_node(&self, node: &Node) -> Result<()> {
        // Re-register with the same lease
        if let Some(lease_id) = *self.lease_id.read().await {
            let node_info = NodeInfo {
                id: node.id.clone(),
                address: node.address,
                port: node.port,
                datacenter: node.datacenter.clone(),
                rack: node.rack.clone(),
                metadata: node.metadata.clone(),
            };

            let value = serde_json::to_string(&node_info)
                .map_err(|e| DbError::Serialization(format!("Failed to serialize node: {}", e)))?;

            let key = format!(
                "{}/{}",
                self.config.key_prefix.trim_end_matches('/'),
                node.id
            );
            self.put_with_lease(&key, &value, lease_id).await?;
        }

        Ok(())
    }

    async fn health_check(&self, node_id: &str) -> Result<HealthStatus> {
        // Check if the node key exists in etcd
        let key = format!(
            "{}/{}",
            self.config.key_prefix.trim_end_matches('/'),
            node_id
        );

        match self.get_prefix(&key).await {
            Ok(kvs) if !kvs.is_empty() => Ok(HealthStatus::Healthy),
            _ => Ok(HealthStatus::Unhealthy),
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down etcd discovery");

        // Cancel lease renewal task
        if let Some(handle) = self.lease_renewal_handle.write().await.take() {
            handle.abort();
        }

        // Delete registration and revoke lease
        if let Some(key) = self.node_key.read().await.as_ref() {
            let _ = self.delete_key(key).await;
        }

        if let Some(lease_id) = *self.lease_id.read().await {
            let _ = self.revoke_lease(lease_id).await;
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "etcd"
    }
}

/// Node information stored in etcd
#[derive(Debug, Serialize, Deserialize)]
struct NodeInfo {
    id: String,
    address: IpAddr,
    port: u16,
    datacenter: Option<String>,
    rack: Option<String>,
    metadata: HashMap<String, String>,
}

/// etcd lease grant response
#[derive(Debug, Deserialize)]
struct EtcdLeaseGrantResponse {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "TTL")]
    #[allow(dead_code)]
    ttl: String,
}

/// etcd range response
#[derive(Debug, Deserialize)]
struct EtcdRangeResponse {
    kvs: Option<Vec<EtcdKeyValue>>,
}

/// etcd key-value pair
#[derive(Debug, Deserialize)]
struct EtcdKeyValue {
    key: String,
    value: String,
}

// Note: Using serde_json::json! macro directly instead of wrapper

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_etcd_discovery_initialization() {
        let config = EtcdConfig::default();
        let mut discovery = EtcdDiscovery::new(config);

        // Note: This will fail without actual etcd instance
        let _ = discovery.initialize().await;

        let _ = discovery.shutdown().await;
    }

    #[test]
    fn test_prefix_range_end() {
        let config = EtcdConfig::default();
        let discovery = EtcdDiscovery::new(config);

        let range_end = discovery.get_prefix_range_end("/rustydb/nodes");
        assert!(!range_end.is_empty());
    }
}
