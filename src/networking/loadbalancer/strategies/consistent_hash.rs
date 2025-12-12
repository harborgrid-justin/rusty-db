// Consistent hashing load balancing strategy.
//
// Provides key-based routing with virtual nodes for even distribution.
// Useful for cache affinity and session persistence.

use super::{Backend, LoadBalancerContext, LoadBalancingStrategy};
use crate::error::{DbError, Result};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Hash function for consistent hashing
#[derive(Debug, Clone, Copy)]
pub enum HashFunction {
    /// FNV-1a hash (fast, good distribution)
    Fnv1a,
    /// xxHash (very fast, excellent distribution)
    XxHash,
    /// CRC32 (fast, hardware accelerated)
    Crc32,
}

impl HashFunction {
    /// Compute hash of input data
    fn hash(&self, data: &[u8]) -> u64 {
        match self {
            HashFunction::Fnv1a => self.fnv1a_hash(data),
            HashFunction::XxHash => self.xxhash(data),
            HashFunction::Crc32 => self.crc32_hash(data) as u64,
        }
    }

    /// FNV-1a hash implementation
    fn fnv1a_hash(&self, data: &[u8]) -> u64 {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = FNV_OFFSET;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// xxHash implementation (simplified)
    fn xxhash(&self, data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }

    /// CRC32 hash implementation
    fn crc32_hash(&self, data: &[u8]) -> u32 {
        const CRC32_POLYNOMIAL: u32 = 0xEDB88320;
        let mut crc = 0xFFFFFFFF;

        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ CRC32_POLYNOMIAL;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }
}

/// Virtual node on the hash ring
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for consistent hash ring implementation
struct VirtualNode {
    /// Backend ID this virtual node represents
    backend_id: String,
    /// Position on the hash ring
    hash: u64,
}

/// Consistent hashing load balancer
///
/// Uses a hash ring with virtual nodes to distribute keys across backends.
/// Provides excellent cache affinity - the same key always routes to the same backend
/// (unless backends are added/removed).
pub struct ConsistentHashBalancer {
    /// Hash ring mapping hash values to backend IDs
    ring: Arc<RwLock<BTreeMap<u64, String>>>,
    /// Number of virtual nodes per backend
    virtual_nodes: usize,
    /// Hash function to use
    hasher: HashFunction,
    /// Whether to use bounded loads (limit load on any single backend)
    bounded_loads: bool,
    /// Maximum load multiplier (e.g., 1.25 = 125% of average)
    max_load_factor: f64,
}

impl ConsistentHashBalancer {
    /// Create a new consistent hash balancer
    ///
    /// # Arguments
    /// * `virtual_nodes` - Number of virtual nodes per backend (higher = better distribution)
    /// * `hasher` - Hash function to use
    pub fn new(virtual_nodes: usize, hasher: HashFunction) -> Self {
        Self {
            ring: Arc::new(RwLock::new(BTreeMap::new())),
            virtual_nodes,
            hasher,
            bounded_loads: false,
            max_load_factor: 1.25,
        }
    }

    /// Enable bounded loads to prevent hotspots
    pub fn with_bounded_loads(mut self, max_load_factor: f64) -> Self {
        self.bounded_loads = true;
        self.max_load_factor = max_load_factor;
        self
    }

    /// Update the hash ring based on current backends
    async fn update_ring(&self, backends: &[Backend]) {
        let mut ring = self.ring.write().await;
        ring.clear();

        for backend in backends {
            // Create virtual nodes for each backend
            for i in 0..self.virtual_nodes {
                let key = format!("{}:{}", backend.id, i);
                let hash = self.hasher.hash(key.as_bytes());
                ring.insert(hash, backend.id.clone());
            }
        }
    }

    /// Find the backend for a given key using consistent hashing
    async fn find_backend(&self, key: &str, backends: &[Backend]) -> Result<Backend> {
        let ring = self.ring.read().await;

        if ring.is_empty() {
            return Err(DbError::Unavailable(
                "Hash ring is empty".to_string(),
            ));
        }

        let key_hash = self.hasher.hash(key.as_bytes());

        // Find the first virtual node with hash >= key_hash
        let backend_id = if let Some((_hash, backend_id)) = ring.range(key_hash..).next() {
            backend_id.clone()
        } else {
            // Wrap around to the first node
            ring.values().next().unwrap().clone()
        };

        // Find the actual backend
        backends
            .iter()
            .find(|b| b.id == backend_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Backend {} not found", backend_id)))
    }

    /// Find backend with bounded loads
    async fn find_backend_bounded(
        &self,
        key: &str,
        backends: &[Backend],
    ) -> Result<Backend> {
        let ring = self.ring.read().await;

        if ring.is_empty() {
            return Err(DbError::Unavailable(
                "Hash ring is empty".to_string(),
            ));
        }

        // Calculate average load
        let total_connections: u32 = backends.iter().map(|b| b.active_connections).sum();
        let avg_load = total_connections as f64 / backends.len() as f64;
        let max_load = avg_load * self.max_load_factor;

        let key_hash = self.hasher.hash(key.as_bytes());

        // Try to find a backend starting from the hash position
        let mut tried_backends = std::collections::HashSet::new();

        // Start from the key hash and walk the ring
        for (_hash, backend_id) in ring.range(key_hash..) {
            if tried_backends.insert(backend_id.clone()) {
                if let Some(backend) = backends.iter().find(|b| b.id == *backend_id) {
                    if backend.active_connections as f64 <= max_load {
                        return Ok(backend.clone());
                    }
                }
            }

            if tried_backends.len() >= backends.len() {
                break;
            }
        }

        // Wrap around to the beginning if needed
        for (_hash, backend_id) in ring.range(..key_hash) {
            if tried_backends.insert(backend_id.clone()) {
                if let Some(backend) = backends.iter().find(|b| b.id == *backend_id) {
                    if backend.active_connections as f64 <= max_load {
                        return Ok(backend.clone());
                    }
                }
            }

            if tried_backends.len() >= backends.len() {
                break;
            }
        }

        // If all backends are over capacity, fall back to standard consistent hashing
        self.find_backend(key, backends).await
    }
}

impl Default for ConsistentHashBalancer {
    fn default() -> Self {
        Self::new(150, HashFunction::Fnv1a)
    }
}

#[async_trait]
impl LoadBalancingStrategy for ConsistentHashBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        context: &LoadBalancerContext,
    ) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        // Update ring to match current backends
        self.update_ring(backends).await;

        // Extract key from context
        let key = context.key.as_deref().unwrap_or_else(|| {
            // Fallback: use client ID or empty string
            context.client_id.as_deref().unwrap_or("")
        });

        if key.is_empty() {
            // If no key provided, select first backend
            return Ok(backends[0].clone());
        }

        // Select backend based on bounded loads setting
        if self.bounded_loads {
            self.find_backend_bounded(key, backends).await
        } else {
            self.find_backend(key, backends).await
        }
    }

    fn name(&self) -> &str {
        if self.bounded_loads {
            "consistent-hash-bounded"
        } else {
            "consistent-hash"
        }
    }

    async fn reset(&self) {
        let mut ring = self.ring.write().await;
        ring.clear();
    }
}

/// Rendezvous (Highest Random Weight) hashing
///
/// Alternative to consistent hashing that doesn't require a hash ring.
/// Each backend is hashed with the key, and the backend with highest hash wins.
pub struct RendezvousHashBalancer {
    hasher: HashFunction,
}

impl RendezvousHashBalancer {
    /// Create a new rendezvous hash balancer
    pub fn new(hasher: HashFunction) -> Self {
        Self { hasher }
    }

    /// Compute combined hash of key and backend
    fn combined_hash(&self, key: &str, backend_id: &str) -> u64 {
        let combined = format!("{}:{}", key, backend_id);
        self.hasher.hash(combined.as_bytes())
    }
}

impl Default for RendezvousHashBalancer {
    fn default() -> Self {
        Self::new(HashFunction::Fnv1a)
    }
}

#[async_trait]
impl LoadBalancingStrategy for RendezvousHashBalancer {
    async fn select(
        &self,
        backends: &[Backend],
        context: &LoadBalancerContext,
    ) -> Result<Backend> {
        if backends.is_empty() {
            return Err(DbError::Unavailable("No backends available".to_string()));
        }

        let key = context
            .key
            .as_deref()
            .or(context.client_id.as_deref())
            .unwrap_or("");

        if key.is_empty() {
            return Ok(backends[0].clone());
        }

        // Find backend with highest hash
        let mut best_backend = None;
        let mut best_hash = 0u64;

        for backend in backends {
            let hash = self.combined_hash(key, &backend.id);
            if hash > best_hash {
                best_hash = hash;
                best_backend = Some(backend.clone());
            }
        }

        best_backend.ok_or_else(|| DbError::Unavailable("No backends available".to_string()))
    }

    fn name(&self) -> &str {
        "rendezvous-hash"
    }

    async fn reset(&self) {
        // No state to reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn create_test_backends(count: usize) -> Vec<Backend> {
        (0..count)
            .map(|i| {
                let addr: SocketAddr = format!("127.0.0.1:{}", 8080 + i).parse().unwrap();
                Backend::new(format!("node{}", i), addr)
            })
            .collect()
    }

    #[tokio::test]
    async fn test_consistent_hash_same_key() {
        let balancer = ConsistentHashBalancer::new(100, HashFunction::Fnv1a);
        let backends = create_test_backends(5);
        let ctx = LoadBalancerContext::with_key("user123");

        // Same key should always route to same backend
        let b1 = balancer.select(&backends, &ctx).await.unwrap();
        let b2 = balancer.select(&backends, &ctx).await.unwrap();
        let b3 = balancer.select(&backends, &ctx).await.unwrap();

        assert_eq!(b1.id, b2.id);
        assert_eq!(b2.id, b3.id);
    }

    #[tokio::test]
    async fn test_consistent_hash_different_keys() {
        let balancer = ConsistentHashBalancer::new(100, HashFunction::Fnv1a);
        let backends = create_test_backends(5);

        let mut backend_ids = std::collections::HashSet::new();

        // Different keys should distribute across backends
        for i in 0..50 {
            let ctx = LoadBalancerContext::with_key(format!("user{}", i));
            let backend = balancer.select(&backends, &ctx).await.unwrap();
            backend_ids.insert(backend.id);
        }

        // Should use at least 3 different backends
        assert!(backend_ids.len() >= 3, "Should distribute across multiple backends");
    }

    #[tokio::test]
    async fn test_consistent_hash_bounded_loads() {
        let balancer =
            ConsistentHashBalancer::new(100, HashFunction::Fnv1a).with_bounded_loads(1.5);

        let mut backends = create_test_backends(3);
        // Make one backend heavily loaded
        backends[0].active_connections = 100;
        backends[1].active_connections = 1;
        backends[2].active_connections = 1;

        let ctx = LoadBalancerContext::with_key("test");
        let backend = balancer.select(&backends, &ctx).await.unwrap();

        // Should avoid the overloaded backend if possible
        assert_ne!(backend.active_connections, 100);
    }

    #[tokio::test]
    async fn test_rendezvous_hash() {
        let balancer = RendezvousHashBalancer::new(HashFunction::Fnv1a);
        let backends = create_test_backends(5);
        let ctx = LoadBalancerContext::with_key("user123");

        // Same key should always route to same backend
        let b1 = balancer.select(&backends, &ctx).await.unwrap();
        let b2 = balancer.select(&backends, &ctx).await.unwrap();

        assert_eq!(b1.id, b2.id);
    }

    #[tokio::test]
    async fn test_hash_functions() {
        let data = b"test data";

        let fnv = HashFunction::Fnv1a;
        let xxhash = HashFunction::XxHash;
        let crc32 = HashFunction::Crc32;

        let h1 = fnv.hash(data);
        let h2 = xxhash.hash(data);
        let h3 = crc32.hash(data);

        // All should produce different results
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);

        // Same data should produce same hash
        assert_eq!(fnv.hash(data), fnv.hash(data));
    }

    #[tokio::test]
    async fn test_virtual_nodes_distribution() {
        let balancer = ConsistentHashBalancer::new(150, HashFunction::Fnv1a);
        let backends = create_test_backends(3);

        let mut selections = std::collections::HashMap::new();

        // Test distribution with many keys
        for i in 0..300 {
            let ctx = LoadBalancerContext::with_key(format!("key{}", i));
            let backend = balancer.select(&backends, &ctx).await.unwrap();
            *selections.entry(backend.id).or_insert(0) += 1;
        }

        // Each backend should get roughly 100 selections (with some variance)
        for count in selections.values() {
            assert!(*count > 50 && *count < 150, "Distribution should be relatively even");
        }
    }
}
