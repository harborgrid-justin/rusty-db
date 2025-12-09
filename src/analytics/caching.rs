/// Advanced Caching and Query Result Management
/// 
/// This module provides sophisticated caching mechanisms:
/// - Multi-level cache hierarchy (L1, L2, L3)
/// - Adaptive cache replacement policies
/// - Query result caching with dependencies
/// - Cache warming and preloading
/// - Distributed cache coordination
/// - Cache statistics and monitoring

use std::time::SystemTime;
use std::time::Instant;
use std::collections::VecDeque;
use crate::error::Result;
use crate::execution::QueryResult;
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Multi-level cache manager
pub struct MultiLevelCache {
    l1_cache: Arc<RwLock<L1Cache>>,
    l2_cache: Arc<RwLock<L2Cache>>,
    l3_cache: Arc<RwLock<L3Cache>>,
    stats: Arc<RwLock<CacheStatistics>>,
}

impl MultiLevelCache {
    pub fn new(l1_size: usize, l2_size: usize, l3_size: usize) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(L1Cache::new(l1_size))),
            l2_cache: Arc::new(RwLock::new(L2Cache::new(l2_size))),
            l3_cache: Arc::new(RwLock::new(L3Cache::new(l3_size))),
            stats: Arc::new(RwLock::new(CacheStatistics::new())),
        }
    }
    
    /// Get value from cache (tries L1 -> L2 -> L3)
    pub fn get(&self, key: &str) -> Option<QueryResult> {
        // Try L1 first (fastest)
        if let Some(result) = self.l1_cache.write().get(key) {
            self.stats.write().record_hit(CacheLevel::L1);
            return Some(result);
        }
        
        // Try L2
        if let Some(result) = self.l2_cache.write().get(key) {
            self.stats.write().record_hit(CacheLevel::L2);
            // Promote to L1
            self.l1_cache.write().put(key.to_string(), result.clone());
            return Some(result);
        }
        
        // Try L3
        if let Some(result) = self.l3_cache.write().get(key) {
            self.stats.write().record_hit(CacheLevel::L3);
            // Promote to L2
            self.l2_cache.write().put(key.to_string(), result.clone());
            return Some(result);
        }
        
        self.stats.write().record_miss();
        None
    }
    
    /// Put value into cache
    pub fn put(&self, key: String, value: QueryResult) {
        // Store in all levels
        self.l1_cache.write().put(key.clone(), value.clone());
        self.l2_cache.write().put(key.clone(), value.clone());
        self.l3_cache.write().put(key, value);
    }
    
    /// Invalidate key from all levels
    pub fn invalidate(&self, key: &str) {
        self.l1_cache.write().invalidate(key);
        self.l2_cache.write().invalidate(key);
        self.l3_cache.write().invalidate(key);
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStatistics {
        self.stats.read().clone()
    }
}

/// L1 Cache (smallest, fastest) - LRU policy
struct L1Cache {
    capacity: usize,
    cache: HashMap<String, CacheEntry>,
    lru_queue: VecDeque<String>,
}

impl L1Cache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
            lru_queue: VecDeque::new(),
        }
    }
    
    fn get(&mut self, key: &str) -> Option<QueryResult> {
        if let Some(entry) = self.cache.get_mut(key) {
            if !entry.is_expired() {
                entry.access_count += 1;
                entry.last_accessed = Instant::now();
                
                // Move to front of LRU queue
                self.lru_queue.retain(|k| k != key);
                self.lru_queue.push_front(key.to_string());
                
                return Some(entry.value.clone());
            } else {
                // Remove expired entry
                self.cache.remove(key);
                self.lru_queue.retain(|k| k != key);
            }
        }
        None
    }
    
    fn put(&mut self, key: String, value: QueryResult) {
        // Evict if at capacity
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            if let Some(lru_key) = self.lru_queue.pop_back() {
                self.cache.remove(&lru_key);
            }
        }
        
        let entry = CacheEntry {
            value,
            inserted_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl: Duration::from_secs(300), // 5 minutes
        };
        
        self.cache.insert(key.clone(), entry);
        self.lru_queue.push_front(key);
    }
    
    fn invalidate(&mut self, key: &str) {
        self.cache.remove(key);
        self.lru_queue.retain(|k| k != key);
    }
}

/// L2 Cache (medium size) - LFU policy (Least Frequently Used)
struct L2Cache {
    capacity: usize,
    cache: HashMap<String, CacheEntry>,
}

impl L2Cache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
        }
    }
    
    fn get(&mut self, key: &str) -> Option<QueryResult> {
        if let Some(entry) = self.cache.get_mut(key) {
            if !entry.is_expired() {
                entry.access_count += 1;
                entry.last_accessed = Instant::now();
                return Some(entry.value.clone());
            } else {
                self.cache.remove(key);
            }
        }
        None
    }
    
    fn put(&mut self, key: String, value: QueryResult) {
        // Evict least frequently used if at capacity
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            if let Some(lfu_key) = self.find_lfu_key() {
                self.cache.remove(&lfu_key);
            }
        }
        
        let entry = CacheEntry {
            value,
            inserted_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl: Duration::from_secs(600), // 10 minutes
        };
        
        self.cache.insert(key, entry);
    }
    
    fn find_lfu_key(&self) -> Option<String> {
        self.cache
            .iter()
            .min_by_key(|(_, entry)| entry.access_count)
            .map(|(key, _)| key.clone())
    }
    
    fn invalidate(&mut self, key: &str) {
        self.cache.remove(key);
    }
}

/// L3 Cache (largest, slowest) - FIFO policy
struct L3Cache {
    capacity: usize,
    cache: HashMap<String, CacheEntry>,
    insertion_order: VecDeque<String>,
}

impl L3Cache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
            insertion_order: VecDeque::new(),
        }
    }
    
    fn get(&mut self, key: &str) -> Option<QueryResult> {
        if let Some(entry) = self.cache.get_mut(key) {
            if !entry.is_expired() {
                entry.access_count += 1;
                entry.last_accessed = Instant::now();
                return Some(entry.value.clone());
            } else {
                self.cache.remove(key);
                self.insertion_order.retain(|k| k != key);
            }
        }
        None
    }
    
    fn put(&mut self, key: String, value: QueryResult) {
        // Evict oldest if at capacity
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            if let Some(old_key) = self.insertion_order.pop_front() {
                self.cache.remove(&old_key);
            }
        }
        
        let entry = CacheEntry {
            value,
            inserted_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl: Duration::from_secs(1800), // 30 minutes
        };
        
        self.cache.insert(key.clone(), entry);
        self.insertion_order.push_back(key);
    }
    
    fn invalidate(&mut self, key: &str) {
        self.cache.remove(key);
        self.insertion_order.retain(|k| k != key);
    }
}

/// Cache entry
#[derive(Clone)]
struct CacheEntry {
    value: QueryResult,
    inserted_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheLevel {
    L1,
    L2,
    L3,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    l1_hits: u64,
    l2_hits: u64,
    l3_hits: u64,
    misses: u64,
}

impl CacheStatistics {
    fn new() -> Self {
        Self {
            l1_hits: 0,
            l2_hits: 0,
            l3_hits: 0,
            misses: 0,
        }
    }
    
    fn record_hit(&mut self, level: CacheLevel) {
        match level {
            CacheLevel::L1 => self.l1_hits += 1,
            CacheLevel::L2 => self.l2_hits += 1,
            CacheLevel::L3 => self.l3_hits += 1,
        }
    }
    
    fn record_miss(&mut self) {
        self.misses += 1;
    }
    
    pub fn total_hits(&self) -> u64 {
        self.l1_hits + self.l2_hits + self.l3_hits
    }
    
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits() + self.misses;
        if total == 0 {
            0.0
        } else {
            self.total_hits() as f64 / total as f64
        }
    }
}

/// Query result cache with dependency tracking
pub struct DependencyAwareCache {
    cache: Arc<RwLock<HashMap<String, CachedQuery>>>,
    dependencies: Arc<RwLock<HashMap<String, Vec<String>>>>, // table -> queries
}

impl DependencyAwareCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Cache query result with table dependencies
    pub fn cache_query(
        &self,
        query: String,
        result: QueryResult,
        tables: Vec<String>,
    ) {
        let query_hash = Self::hash_query(&query);
        
        // Store query result
        self.cache.write().insert(
            query_hash.clone(),
            CachedQuery {
                query: query.clone(),
                result,
                tables: tables.clone(),
                cached_at: SystemTime::now(),
            },
        );
        
        // Register dependencies
        let mut deps = self.dependencies.write();
        for table in tables {
            deps.entry(table)
                .or_insert_with(Vec::new)
                .push(query_hash.clone());
        }
    }
    
    /// Get cached query result
    pub fn get_cached(&self, query: &str) -> Option<QueryResult> {
        let query_hash = Self::hash_query(query);
        self.cache.read().get(&query_hash).map(|cq| cq.result.clone())
    }
    
    /// Invalidate all queries dependent on a table
    pub fn invalidate_table(&self, table: &str) {
        if let Some(queries) = self.dependencies.write().remove(table) {
            let mut cache = self.cache.write();
            for query_hash in queries {
                cache.remove(&query_hash);
            }
        }
    }
    
    fn hash_query(query: &str) -> String {
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Cached query with metadata
struct CachedQuery {
    query: String,
    result: QueryResult,
    tables: Vec<String>,
    cached_at: SystemTime,
}

/// Cache warming strategy
pub struct CacheWarmer {
    cache: Arc<MultiLevelCache>,
    warming_queries: Vec<WarmingQuery>,
}

impl CacheWarmer {
    pub fn new(cache: Arc<MultiLevelCache>) -> Self {
        Self {
            cache,
            warming_queries: Vec::new(),
        }
    }
    
    /// Register query for cache warming
    pub fn register_warming_query(&mut self, query: String, schedule: WarmingSchedule) {
        self.warming_queries.push(WarmingQuery { query, schedule });
    }
    
    /// Execute cache warming
    pub async fn warm_cache(&self) -> Result<()> {
        for warming_query in &self.warming_queries {
            // In real implementation, would execute query
            // and cache the result
            let _query = &warming_query.query;
            // let _result = executor.execute(query).await?;
            // self.cache.put(query.clone(), result);
        }
        Ok(())
    }
}

/// Warming query configuration
struct WarmingQuery {
    query: String,
    schedule: WarmingSchedule,
}

/// Cache warming schedule
pub enum WarmingSchedule {
    OnStartup,
    Periodic(Duration),
    AfterHours,
}

/// Adaptive cache replacement policy
pub struct AdaptiveCachePolicy {
    /// Tracks access patterns
    access_history: VecDeque<AccessRecord>,
    /// Maximum history size
    max_history: usize,
}

impl AdaptiveCachePolicy {
    pub fn new(max_history: usize) -> Self {
        Self {
            access_history: VecDeque::new(),
            max_history,
        }
    }
    
    /// Record cache access
    pub fn record_access(&mut self, key: String, hit: bool) {
        if self.access_history.len() >= self.max_history {
            self.access_history.pop_front();
        }
        
        self.access_history.push_back(AccessRecord {
            key,
            hit,
            timestamp: Instant::now(),
        });
    }
    
    /// Decide eviction victim based on access patterns
    pub fn select_victim(&self, candidates: &[String]) -> Option<String> {
        // Analyze access patterns
        let mut scores: HashMap<String, f64> = HashMap::new();
        
        for candidate in candidates {
            let score = self.calculate_score(candidate);
            scores.insert(candidate.clone(), score);
        }
        
        // Evict item with lowest score
        scores
            .into_iter()
            .min_by(|(_, s1), (_, s2)| s1.partial_cmp(s2).unwrap())
            .map(|(key, _)| key)
    }
    
    fn calculate_score(&self, key: &str) -> f64 {
        let mut score = 0.0;
        let mut access_count = 0u64;
        let mut recent_access_count = 0u64;
        let recent_threshold = Instant::now() - Duration::from_secs(60);
        
        for record in &self.access_history {
            if record.key == key {
                access_count += 1;
                if record.timestamp > recent_threshold {
                    recent_access_count += 1;
                }
            }
        }
        
        // Score based on recency and frequency
        score += access_count as f64;
        score += recent_access_count as f64 * 2.0; // Weight recent accesses more
        
        score
    }
}

/// Access record for adaptive policy
struct AccessRecord {
    key: String,
    hit: bool,
    timestamp: Instant,
}

/// Distributed cache coordinator
pub struct DistributedCacheCoordinator {
    local_cache: Arc<MultiLevelCache>,
    remote_nodes: Vec<RemoteNode>,
}

impl DistributedCacheCoordinator {
    pub fn new(local_cache: Arc<MultiLevelCache>) -> Self {
        Self {
            local_cache,
            remote_nodes: Vec::new(),
        }
    }
    
    /// Add remote cache node
    pub fn add_node(&mut self, node: RemoteNode) {
        self.remote_nodes.push(node);
    }
    
    /// Get from distributed cache
    pub async fn get_distributed(&self, key: &str) -> Option<QueryResult> {
        // Try local first
        if let Some(result) = self.local_cache.get(key) {
            return Some(result);
        }
        
        // Try remote nodes
        for node in &self.remote_nodes {
            if let Ok(Some(result)) = node.get(key).await {
                // Cache locally
                self.local_cache.put(key.to_string(), result.clone());
                return Some(result);
            }
        }
        
        None
    }
    
    /// Invalidate across all nodes
    pub async fn invalidate_distributed(&self, key: &str) {
        // Invalidate locally
        self.local_cache.invalidate(key);
        
        // Invalidate on remote nodes
        for node in &self.remote_nodes {
            let _ = node.invalidate(key).await;
        }
    }
}

/// Remote cache node
pub struct RemoteNode {
    address: String,
}

impl RemoteNode {
    pub fn new(address: String) -> Self {
        Self { address }
    }
    
    pub async fn get(&self, _key: &str) -> Result<Option<QueryResult>> {
        // Would make network request to remote node
        Ok(None)
    }
    
    pub async fn invalidate(&self, _key: &str) -> Result<()> {
        // Would make network request to remote node
        Ok(())
    }
}

/// Cache preloader for frequently accessed data
pub struct CachePreloader {
    cache: Arc<MultiLevelCache>,
    preload_rules: Vec<PreloadRule>,
}

impl CachePreloader {
    pub fn new(cache: Arc<MultiLevelCache>) -> Self {
        Self {
            cache,
            preload_rules: Vec::new(),
        }
    }
    
    /// Add preload rule
    pub fn add_rule(&mut self, rule: PreloadRule) {
        self.preload_rules.push(rule);
    }
    
    /// Execute preloading
    pub async fn preload(&self) -> Result<usize> {
        let mut preloaded = 0;
        
        for rule in &self.preload_rules {
            // Execute preload query and cache results
            // In real implementation, would execute actual query
            preloaded += 1;
        }
        
        Ok(preloaded)
    }
}

/// Preload rule
pub struct PreloadRule {
    pub name: String,
    pub query: String,
    pub priority: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multi_level_cache() {
        let cache = MultiLevelCache::new(10, 50, 200);
        
        let _result = QueryResult::new(
            vec!["id".to_string(), "name".to_string()],
            vec![vec!["1".to_string(), "Alice".to_string()]],
        );
        
        cache.put("query1".to_string(), result.clone());
        
        let cached = cache.get("query1");
        assert!(cached.is_some());
        
        let _stats = cache.get_stats();
        assert_eq!(stats.l1_hits, 1);
        assert_eq!(stats.total_hits(), 1);
    }
    
    #[test]
    fn test_dependency_aware_cache() {
        let cache = DependencyAwareCache::new();
        
        let _result = QueryResult::new(vec!["id".to_string()], vec![]);
        
        cache.cache_query(
            "SELECT * FROM users".to_string(),
            result.clone(),
            vec!["users".to_string()],
        );
        
        assert!(cache.get_cached("SELECT * FROM users").is_some());
        
        // Invalidate based on table
        cache.invalidate_table("users");
        assert!(cache.get_cached("SELECT * FROM users").is_none());
    }
    
    #[test]
    fn test_adaptive_cache_policy() {
        let mut policy = AdaptiveCachePolicy::new(100);

        // Record access patterns
        policy.record_access("key1".to_string(), true);
        policy.record_access("key1".to_string(), true);
        policy.record_access("key2".to_string(), true);

        // key1 should have higher score (accessed more)
        let victim = policy.select_victim(&["key1".to_string(), "key2".to_string()]);
        assert_eq!(victim, Some("key2".to_string()));
    }
}

/// Semantic query cache with normalization
///
/// Recognizes semantically equivalent queries even if syntactically different
pub struct SemanticQueryCache {
    cache: Arc<RwLock<HashMap<String, CachedQueryResult>>>,
    normalizer: Arc<QueryNormalizer>,
    partial_cache: Arc<RwLock<PartialResultCache>>,
}

impl SemanticQueryCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            normalizer: Arc::new(QueryNormalizer::new()),
            partial_cache: Arc::new(RwLock::new(PartialResultCache::new())),
        }
    }

    /// Get cached result for query
    pub fn get(&self, query: &str) -> Option<QueryResult> {
        // Normalize query
        let normalized = self.normalizer.normalize(query).ok()?;

        let cache = self.cache.read();
        cache.get(&normalized).map(|cr| cr.result.clone())
    }

    /// Cache query result
    pub fn put(&self, query: &str, result: QueryResult) -> Result<()> {
        let normalized = self.normalizer.normalize(query)?;

        let cached_result = CachedQueryResult {
            result: result.clone(),
            query: query.to_string(),
            normalized_query: normalized.clone(),
            cached_at: SystemTime::now(),
            access_count: 0,
            size_bytes: self.estimate_size(&result),
        };

        self.cache.write().insert(normalized, cached_result);

        // Also cache partial results if applicable
        self.cache_partial_results(query, &result)?;

        Ok(())
    }

    /// Try to answer query using partial results
    pub fn get_partial(&self, query: &str) -> Option<PartialMatch> {
        self.partial_cache.read().find_partial_match(query)
    }

    /// Invalidate queries matching pattern
    pub fn invalidate_pattern(&self, pattern: &str) -> usize {
        let mut cache = self.cache.write();
        let keys_to_remove: Vec<_> = cache.keys()
            .filter(|k| k.contains(pattern))
            .cloned()
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            cache.remove(&key);
        }

        count
    }

    fn estimate_size(&self, result: &QueryResult) -> usize {
        // Rough estimate
        result.columns.len() * 100 + result.rows.len() * 100
    }

    fn cache_partial_results(&self, query: &str, result: &QueryResult) -> Result<()> {
        // Extract subqueries and cache them separately
        // In production, would parse query and cache intermediate results
        Ok(())
    }
}

/// Cached query result with metadata
struct CachedQueryResult {
    result: QueryResult,
    query: String,
    normalized_query: String,
    cached_at: SystemTime,
    access_count: u64,
    size_bytes: usize,
}

/// Query normalizer for semantic equivalence
pub struct QueryNormalizer {
    rules: Vec<NormalizationRule>,
}

impl QueryNormalizer {
    pub fn new() -> Self {
        Self {
            rules: vec![
                NormalizationRule::RemoveWhitespace,
                NormalizationRule::Lowercase,
                NormalizationRule::SortPredicates,
                NormalizationRule::CanonicalizeNames,
                NormalizationRule::RemoveComments,
            ],
        }
    }

    /// Normalize query to canonical form
    pub fn normalize(&self, query: &str) -> Result<String> {
        let mut normalized = query.to_string();

        for rule in &self.rules {
            normalized = self.apply_rule(&normalized, rule)?;
        }

        Ok(normalized)
    }

    fn apply_rule(&self, query: &str, rule: &NormalizationRule) -> Result<String> {
        match rule {
            NormalizationRule::RemoveWhitespace => {
                Ok(query.split_whitespace().collect::<Vec<_>>().join(" "))
            }
            NormalizationRule::Lowercase => {
                Ok(query.to_lowercase())
            }
            NormalizationRule::SortPredicates => {
                // Simplified - would parse and sort WHERE predicates
                Ok(query.to_string())
            }
            NormalizationRule::CanonicalizeNames => {
                // Simplified - would resolve aliases and use canonical names
                Ok(query.to_string())
            }
            NormalizationRule::RemoveComments => {
                Ok(query.lines()
                    .filter(|line| !line.trim_start().starts_with("--"))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
    }
}

#[derive(Debug, Clone)]
enum NormalizationRule {
    RemoveWhitespace,
    Lowercase,
    SortPredicates,
    CanonicalizeNames,
    RemoveComments,
}

/// Partial result cache for query fragments
pub struct PartialResultCache {
    fragments: HashMap<String, QueryFragment>,
}

impl PartialResultCache {
    pub fn new() -> Self {
        Self {
            fragments: HashMap::new(),
        }
    }

    pub fn add_fragment(&mut self, fragment: QueryFragment) {
        self.fragments.insert(fragment.key.clone(), fragment);
    }

    pub fn find_partial_match(&self, query: &str) -> Option<PartialMatch> {
        // Find fragments that match parts of the query
        for (key, fragment) in &self.fragments {
            if query.contains(key) {
                return Some(PartialMatch {
                    fragment: fragment.clone(),
                    coverage: 0.5, // Simplified
                });
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct QueryFragment {
    pub key: String,
    pub result: QueryResult,
    pub tables: Vec<String>,
    pub predicates: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PartialMatch {
    pub fragment: QueryFragment,
    pub coverage: f64, // 0.0 to 1.0
}

/// Memory pressure handler for cache eviction
pub struct MemoryPressureHandler {
    max_memory_bytes: usize,
    current_memory_bytes: Arc<RwLock<usize>>,
    eviction_threshold: f64,
}

impl MemoryPressureHandler {
    pub fn new(max_memory_bytes: usize) -> Self {
        Self {
            max_memory_bytes,
            current_memory_bytes: Arc::new(RwLock::new(0)),
            eviction_threshold: 0.9,
        }
    }

    /// Check if under memory pressure
    pub fn is_under_pressure(&self) -> bool {
        let current = *self.current_memory_bytes.read();
        current as f64 > self.max_memory_bytes as f64 * self.eviction_threshold
    }

    /// Record memory allocation
    pub fn allocate(&self, bytes: usize) {
        *self.current_memory_bytes.write() += bytes;
    }

    /// Record memory deallocation
    pub fn deallocate(&self, bytes: usize) {
        let mut current = self.current_memory_bytes.write();
        *current = current.saturating_sub(bytes);
    }

    /// Get current memory usage
    pub fn current_usage(&self) -> usize {
        *self.current_memory_bytes.read()
    }

    /// Get memory usage percentage
    pub fn usage_percentage(&self) -> f64 {
        let current = *self.current_memory_bytes.read();
        current as f64 / self.max_memory_bytes as f64
    }
}

/// Cache update notification system
pub struct CacheInvalidationNotifier {
    subscribers: Arc<RwLock<Vec<InvalidationSubscriber>>>,
}

impl CacheInvalidationNotifier {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Subscribe to invalidation notifications
    pub fn subscribe(&self, subscriber: InvalidationSubscriber) {
        self.subscribers.write().push(subscriber);
    }

    /// Notify all subscribers of invalidation
    pub fn notify(&self, event: InvalidationEvent) {
        let subscribers = self.subscribers.read();
        for subscriber in subscribers.iter() {
            subscriber.notify(&event);
        }
    }
}

pub struct InvalidationSubscriber {
    pub name: String,
    pub callback: Arc<dyn Fn(&InvalidationEvent) + Send + Sync>,
}

impl InvalidationSubscriber {
    pub fn notify(&self, event: &InvalidationEvent) {
        (self.callback)(event);
    }
}

#[derive(Debug, Clone)]
pub struct InvalidationEvent {
    pub event_type: InvalidationType,
    pub affected_tables: Vec<String>,
    pub affected_queries: Vec<String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub enum InvalidationType {
    TableUpdate,
    TableDelete,
    SchemaChange,
    Manual,
}

#[cfg(test)]
mod semantic_tests {
    use super::*;

    #[test]
    fn test_query_normalization() {
        let normalizer = QueryNormalizer::new();

        let query1 = "SELECT * FROM users WHERE id = 1";
        let query2 = "select   *   from   users   where   id=1";

        let norm1 = normalizer.normalize(query1).unwrap();
        let norm2 = normalizer.normalize(query2).unwrap();

        // Should normalize to same form
        assert_eq!(norm1, norm2);
    }

    #[test]
    fn test_semantic_cache() {
        let cache = SemanticQueryCache::new();

        let _result = QueryResult::new(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        );

        cache.put("SELECT * FROM users", result.clone()).unwrap();

        // Should retrieve with different whitespace
        let cached = cache.get("SELECT   *   FROM   users");
        assert!(cached.is_some());
    }

    #[test]
    fn test_memory_pressure() {
        let handler = MemoryPressureHandler::new(1000);

        handler.allocate(500);
        assert!(!handler.is_under_pressure());

        handler.allocate(500);
        assert!(handler.is_under_pressure());

        handler.deallocate(200);
        assert!(!handler.is_under_pressure());
    }
}


