//! # Address Resolution
//!
//! Advanced address resolution with caching and load balancing.
//!
//! ## Features
//!
//! - **DNS Resolution**: Hostname to IP address resolution
//! - **SRV Records**: Service discovery via DNS SRV records
//! - **Load Balancing**: Distribute across multiple endpoints
//! - **Caching**: TTL-based caching for performance

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Resolved endpoint with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEndpoint {
    /// Socket address
    pub addr: SocketAddr,

    /// Priority (lower is higher priority, like DNS SRV)
    pub priority: u16,

    /// Weight for load balancing
    pub weight: u16,

    /// Time when this endpoint was resolved
    pub resolved_at: SystemTime,

    /// TTL for this resolution
    pub ttl: Duration,
}

impl ResolvedEndpoint {
    /// Create a new resolved endpoint
    pub fn new(addr: SocketAddr, priority: u16, weight: u16, ttl: Duration) -> Self {
        Self {
            addr,
            priority,
            weight,
            resolved_at: SystemTime::now(),
            ttl,
        }
    }

    /// Check if this endpoint has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.resolved_at)
            .map(|elapsed| elapsed >= self.ttl)
            .unwrap_or(true)
    }
}

/// Resolver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    /// Default TTL for cached entries (in seconds)
    pub cache_ttl: u64,

    /// Maximum cache size
    pub max_cache_size: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            cache_ttl: 300, // 5 minutes
            max_cache_size: 1000,
        }
    }
}

/// Cache entry
struct CacheEntry {
    endpoints: Vec<ResolvedEndpoint>,
    inserted_at: SystemTime,
}

impl CacheEntry {
    fn new(endpoints: Vec<ResolvedEndpoint>) -> Self {
        Self {
            endpoints,
            inserted_at: SystemTime::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.inserted_at)
            .map(|elapsed| elapsed >= ttl)
            .unwrap_or(true)
    }
}

/// Address resolver with caching
pub struct AddressResolver {
    config: ResolverConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl AddressResolver {
    /// Create a new address resolver
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolve an address to endpoints
    ///
    /// Supports:
    /// - IP addresses: "192.168.1.1:5432"
    /// - Hostnames: "db.example.com:5432"
    /// - SRV records: "_postgres._tcp.example.com" (future)
    pub async fn resolve(&mut self, address: &str) -> Result<Vec<ResolvedEndpoint>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(address) {
                let ttl = Duration::from_secs(self.config.cache_ttl);
                if !entry.is_expired(ttl) {
                    return Ok(entry.endpoints.clone());
                }
            }
        }

        // Resolve the address
        let endpoints = self.resolve_uncached(address).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;

            // Evict old entries if cache is full
            if cache.len() >= self.config.max_cache_size {
                self.evict_expired(&mut cache);
            }

            // If still full, remove oldest entry
            if cache.len() >= self.config.max_cache_size {
                if let Some(oldest_key) = self.find_oldest_entry(&cache) {
                    cache.remove(&oldest_key);
                }
            }

            cache.insert(address.to_string(), CacheEntry::new(endpoints.clone()));
        }

        Ok(endpoints)
    }

    /// Resolve without using cache
    async fn resolve_uncached(&self, address: &str) -> Result<Vec<ResolvedEndpoint>> {
        // Try to parse as direct socket address first
        if let Ok(addr) = address.parse::<SocketAddr>() {
            return Ok(vec![ResolvedEndpoint::new(
                addr,
                0,
                1,
                Duration::from_secs(self.config.cache_ttl),
            )]);
        }

        // Try hostname resolution
        let addrs = tokio::task::spawn_blocking({
            let address = address.to_string();
            move || {
                address.to_socket_addrs()
                    .map(|iter| iter.collect::<Vec<_>>())
            }
        })
        .await
        .map_err(|e| DbError::Network(format!("Task join error: {}", e)))?
        .map_err(|e| DbError::Network(format!("DNS resolution failed for {}: {}", address, e)))?;

        if addrs.is_empty() {
            return Err(DbError::Network(format!("No addresses found for {}", address)));
        }

        // Convert to resolved endpoints
        let endpoints = addrs
            .into_iter()
            .enumerate()
            .map(|(_i, addr)| {
                ResolvedEndpoint::new(
                    addr,
                    0,                                          // All have same priority
                    1,                                          // Equal weight
                    Duration::from_secs(self.config.cache_ttl),
                )
            })
            .collect();

        Ok(endpoints)
    }

    /// Resolve multiple addresses
    pub async fn resolve_many(&mut self, addresses: &[String]) -> Result<HashMap<String, Vec<ResolvedEndpoint>>> {
        let mut results = HashMap::new();

        for address in addresses {
            match self.resolve(address).await {
                Ok(endpoints) => {
                    results.insert(address.clone(), endpoints);
                }
                Err(e) => {
                    tracing::warn!("Failed to resolve {}: {}", address, e);
                }
            }
        }

        Ok(results)
    }

    /// Select an endpoint using round-robin from resolved endpoints
    pub fn select_round_robin(endpoints: &[ResolvedEndpoint], counter: &mut usize) -> Option<SocketAddr> {
        if endpoints.is_empty() {
            return None;
        }

        let index = *counter % endpoints.len();
        *counter = (*counter + 1) % endpoints.len();

        Some(endpoints[index].addr)
    }

    /// Select an endpoint using weighted random selection
    pub fn select_weighted_random(endpoints: &[ResolvedEndpoint]) -> Option<SocketAddr> {
        if endpoints.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: u32 = endpoints.iter().map(|e| e.weight as u32).sum();

        if total_weight == 0 {
            // If all weights are 0, use uniform random
            use rand::Rng;
            let index = rand::thread_rng().gen_range(0..endpoints.len());
            return Some(endpoints[index].addr);
        }

        // Weighted random selection
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut random_weight = rng.gen_range(0..total_weight);

        for endpoint in endpoints {
            if random_weight < endpoint.weight as u32 {
                return Some(endpoint.addr);
            }
            random_weight -= endpoint.weight as u32;
        }

        // Fallback to first endpoint
        Some(endpoints[0].addr)
    }

    /// Select an endpoint by priority (lowest priority value first)
    pub fn select_by_priority(endpoints: &[ResolvedEndpoint]) -> Option<SocketAddr> {
        endpoints
            .iter()
            .min_by_key(|e| e.priority)
            .map(|e| e.addr)
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Evict expired entries from cache
    fn evict_expired(&self, cache: &mut HashMap<String, CacheEntry>) {
        let ttl = Duration::from_secs(self.config.cache_ttl);
        cache.retain(|_, entry| !entry.is_expired(ttl));
    }

    /// Find the oldest entry in cache
    fn find_oldest_entry(&self, cache: &HashMap<String, CacheEntry>) -> Option<String> {
        cache
            .iter()
            .min_by_key(|(_, entry)| entry.inserted_at)
            .map(|(key, _)| key.clone())
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), self.config.max_cache_size)
    }

    /// Resolve SRV record (placeholder for future implementation)
    pub async fn resolve_srv(&mut self, service: &str, proto: &str, domain: &str) -> Result<Vec<ResolvedEndpoint>> {
        // SRV record format: _service._proto.domain
        let srv_name = format!("_{}._{}.{}", service, proto, domain);

        // For now, return not implemented
        // A full implementation would use a DNS library to query SRV records
        Err(DbError::NotImplemented(format!("SRV resolution for {} not yet implemented", srv_name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_ip_address() {
        let config = ResolverConfig::default();
        let mut resolver = AddressResolver::new(config);

        let result = resolver.resolve("127.0.0.1:5432").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].addr.to_string(), "127.0.0.1:5432");
    }

    #[tokio::test]
    async fn test_resolve_localhost() {
        let config = ResolverConfig::default();
        let mut resolver = AddressResolver::new(config);

        let result = resolver.resolve("localhost:5432").await.unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_round_robin_selection() {
        let endpoints = vec![
            ResolvedEndpoint::new("127.0.0.1:5432".parse().unwrap(), 0, 1, Duration::from_secs(300)),
            ResolvedEndpoint::new("127.0.0.1:5433".parse().unwrap(), 0, 1, Duration::from_secs(300)),
            ResolvedEndpoint::new("127.0.0.1:5434".parse().unwrap(), 0, 1, Duration::from_secs(300)),
        ];

        let mut counter = 0;

        let addr1 = AddressResolver::select_round_robin(&endpoints, &mut counter).unwrap();
        let addr2 = AddressResolver::select_round_robin(&endpoints, &mut counter).unwrap();
        let addr3 = AddressResolver::select_round_robin(&endpoints, &mut counter).unwrap();
        let addr4 = AddressResolver::select_round_robin(&endpoints, &mut counter).unwrap();

        assert_eq!(addr1.port(), 5432);
        assert_eq!(addr2.port(), 5433);
        assert_eq!(addr3.port(), 5434);
        assert_eq!(addr4.port(), 5432); // Wraps around
    }

    #[test]
    fn test_priority_selection() {
        let endpoints = vec![
            ResolvedEndpoint::new("127.0.0.1:5432".parse().unwrap(), 10, 1, Duration::from_secs(300)),
            ResolvedEndpoint::new("127.0.0.1:5433".parse().unwrap(), 5, 1, Duration::from_secs(300)),
            ResolvedEndpoint::new("127.0.0.1:5434".parse().unwrap(), 20, 1, Duration::from_secs(300)),
        ];

        let addr = AddressResolver::select_by_priority(&endpoints).unwrap();
        assert_eq!(addr.port(), 5433); // Priority 5 is highest (lowest value)
    }

    #[tokio::test]
    async fn test_cache() {
        let config = ResolverConfig {
            cache_ttl: 300,
            max_cache_size: 10,
        };
        let mut resolver = AddressResolver::new(config);

        // First resolution
        let result1 = resolver.resolve("127.0.0.1:5432").await.unwrap();

        // Second resolution (should use cache)
        let result2 = resolver.resolve("127.0.0.1:5432").await.unwrap();

        assert_eq!(result1.len(), result2.len());

        let (cache_size, _) = resolver.cache_stats().await;
        assert_eq!(cache_size, 1);
    }

    #[test]
    fn test_endpoint_expiration() {
        let endpoint = ResolvedEndpoint::new(
            "127.0.0.1:5432".parse().unwrap(),
            0,
            1,
            Duration::from_millis(1),
        );

        std::thread::sleep(Duration::from_millis(10));
        assert!(endpoint.is_expired());
    }

    #[test]
    fn test_weighted_random_selection() {
        let endpoints = vec![
            ResolvedEndpoint::new("127.0.0.1:5432".parse().unwrap(), 0, 10, Duration::from_secs(300)),
            ResolvedEndpoint::new("127.0.0.1:5433".parse().unwrap(), 0, 5, Duration::from_secs(300)),
        ];

        // Just verify it returns something valid
        let addr = AddressResolver::select_weighted_random(&endpoints).unwrap();
        assert!(addr.port() == 5432 || addr.port() == 5433);
    }
}
