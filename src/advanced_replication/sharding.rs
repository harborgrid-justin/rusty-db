// # Sharding Engine
//
// Advanced sharding with hash, range, list, and composite strategies.
// Includes shard rebalancing and cross-shard query support.

use tokio::time::sleep;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, SystemTime};
use crate::error::DbError;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

type Result<T> = std::result::Result<T, DbError>;

/// Sharding strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShardingStrategy {
    /// Hash-based sharding
    Hash {
        num_shards: usize,
        hash_function: HashFunction,
    },
    /// Range-based sharding
    Range {
        ranges: Vec<RangeDefinition>,
    },
    /// List-based sharding
    List {
        lists: Vec<ListDefinition>,
    },
    /// Composite sharding (combine multiple strategies)
    Composite {
        strategies: Vec<Box<ShardingStrategy>>,
    },
}

/// Hash function for sharding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HashFunction {
    /// Standard hash
    Default,
    /// Consistent hashing
    Consistent,
    /// Murmur3 hash
    Murmur3,
    /// FNV hash
    Fnv,
}

/// Range definition for range-based sharding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RangeDefinition {
    /// Shard ID
    pub shard_id: String,
    /// Range start (inclusive)
    pub start: ShardKey,
    /// Range end (exclusive)
    pub end: ShardKey,
}

/// List definition for list-based sharding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListDefinition {
    /// Shard ID
    pub shard_id: String,
    /// Values that belong to this shard
    pub values: Vec<ShardKey>,
}

/// Shard key value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShardKey {
    Integer(i64),
    String(String),
    Binary(Vec<u8>),
    Composite(Vec<Box<ShardKey>>),
}

impl ShardKey {
    /// Calculate hash of the shard key
    pub fn hash_key(&self, hash_fn: &HashFunction) -> u64 {
        match hash_fn {
            HashFunction::Default | HashFunction::Consistent => {
                let mut hasher = DefaultHasher::new();
                Hash::hash(self, &mut hasher);
                hasher.finish()
            }
            HashFunction::Murmur3 => {
                // Simplified murmur3 implementation
                let mut hasher = DefaultHasher::new();
                Hash::hash(self, &mut hasher);
                hasher.finish().wrapping_mul(0xc6a4a7935bd1e995)
            }
            HashFunction::Fnv => {
                // FNV-1a hash
                let mut hash = 0xcbf29ce484222325u64;
                let bytes = bincode::serialize(self).unwrap_or_default();
                for byte in bytes {
                    hash ^= byte as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                hash
            }
        }
    }
}

/// Shard information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    /// Shard ID
    pub id: String,
    /// Shard name
    pub name: String,
    /// Server/node hosting this shard
    pub server: String,
    /// Shard status
    pub status: ShardStatus,
    /// Number of rows in shard
    pub row_count: u64,
    /// Size in bytes
    pub size_bytes: u64,
    /// Last rebalance time
    pub last_rebalance: u64,
    /// Shard metadata
    pub metadata: HashMap<String, String>,
}

/// Shard status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShardStatus {
    /// Shard is active and serving requests
    Active,
    /// Shard is being created
    Creating,
    /// Shard is being rebalanced/migrated
    Rebalancing,
    /// Shard is read-only
    ReadOnly,
    /// Shard is offline
    Offline,
    /// Shard is being dropped
    Dropping,
}

/// Sharding configuration for a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardedTable {
    /// Table name
    pub table_name: String,
    /// Schema name
    pub schema_name: String,
    /// Shard key columns
    pub shard_key_columns: Vec<String>,
    /// Sharding strategy
    pub strategy: ShardingStrategy,
    /// Shards
    pub shards: Vec<Shard>,
    /// Creation time
    pub created_at: u64,
}

/// Cross-shard query plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardQuery {
    /// Query ID
    pub query_id: String,
    /// SQL query
    pub sql: String,
    /// Shards involved
    pub shards: Vec<String>,
    /// Per-shard queries
    pub shard_queries: HashMap<String, String>,
    /// Aggregation required
    pub requires_aggregation: bool,
    /// Sort order
    pub sort_columns: Vec<String>,
}

/// Shard rebalancing plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancePlan {
    /// Plan ID
    pub id: String,
    /// Table being rebalanced
    pub table: String,
    /// Source shard
    pub source_shard: String,
    /// Target shard
    pub target_shard: String,
    /// Number of rows to move
    pub row_count: u64,
    /// Estimated time (seconds)
    pub estimated_duration: u64,
    /// Plan state
    pub state: RebalanceState,
    /// Progress (0-100)
    pub progress: u8,
}

/// Rebalance state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RebalanceState {
    Planned,
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

/// Sharding engine
pub struct ShardingEngine {
    /// Sharded tables
    tables: Arc<RwLock<HashMap<String, ShardedTable>>>,
    /// Shard directory
    shards: Arc<RwLock<HashMap<String, Shard>>>,
    /// Active rebalance plans
    rebalance_plans: Arc<RwLock<HashMap<String, RebalancePlan>>>,
    /// Statistics
    stats: Arc<RwLock<ShardingStats>>,
}

/// Sharding statistics with cache alignment
#[repr(C, align(64))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShardingStats {
    pub total_shards: u64,
    pub active_shards: u64,
    pub total_tables: u64,
    pub cross_shard_queries: u64,
    pub rebalances_completed: u64,
    pub rebalances_failed: u64,
    pub total_rows_sharded: u64,
    pub queries_by_strategy: HashMap<String, u64>,
}

impl ShardingEngine {
    /// Create a new sharding engine
    pub fn new() -> Self {
        Self {
            tables: Arc::new(RwLock::new(HashMap::new())),
            shards: Arc::new(RwLock::new(HashMap::new())),
            rebalance_plans: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ShardingStats::default())),
        }
    }

    /// Create a sharded table
    pub fn create_sharded_table(&self, table: ShardedTable) -> Result<()> {
        let mut tables = self.tables.write();

        let key = format!("{}.{}", table.schema_name, table.table_name)));
        if tables.contains_key(&key) {
            return Err(DbError::Replication(
                format!("Table {} is already sharded", key)
            ))));
        }

        // Validate shards
        for shard in &table.shards {
            let mut shards = self.shards.write();
            shards.insert(shard.id.clone(), shard.clone());
        }

        tables.insert(key, table);

        let mut stats = self.stats.write();
        stats.total_tables += 1;
        stats.total_shards += 1;
        stats.active_shards += 1;

        Ok(())
    }

    /// Route a query to the appropriate shard(s)
    pub fn route_query(&self, table: &str, shard_key: &ShardKey) -> Result<Vec<String>> {
        let tables = self.tables.read();

        let sharded_table = tables.get(table)
            .ok_or_else(|| DbError::Replication(
                format!("Table {} is not sharded", table)
            ))?);

        let shard_ids = self.determine_shards(&sharded_table.strategy, shard_key)?;

        Ok(shard_ids)
    }

    /// Determine which shard(s) a key belongs to
    fn determine_shards(&self, strategy: &ShardingStrategy, key: &ShardKey) -> Result<Vec<String>> {
        match strategy {
            ShardingStrategy::Hash { num_shards, hash_function } => {
                let hash = key.hash_key(hash_function);
                let shard_num = (hash % (*num_shards as u64)) as usize;
                Ok(vec![format!("shard-{}", shard_num)])
            }
            ShardingStrategy::Range { ranges } => {
                for range in ranges {
                    if key >= &range.start && key < &range.end {
                        return Ok(vec![range.shard_id.clone()])));
                    }
                }
                Err(DbError::Replication(
                    format!("No shard found for key {:?}", key)
                ))
            }
            ShardingStrategy::List { lists } => {
                for list in lists {
                    if list.values.contains(key) {
                        return Ok(vec![list.shard_id.clone()])));
                    }
                }
                Err(DbError::Replication(
                    format!("No shard found for key {:?}", key)
                ))
            }
            ShardingStrategy::Composite { strategies } => {
                let mut all_shards = Vec::new()));
                for strategy in strategies {
                    let shards = self.determine_shards(strategy, key)?;
                    all_shards.extend(shards);
                }
                Ok(all_shards)
            }
        }
    }

    /// Plan a cross-shard query
    pub fn plan_cross_shard_query(&self, table: &str, sql: &str) -> Result<CrossShardQuery> {
        let tables = self.tables.read();

        let sharded_table = tables.get(table)
            .ok_or_else(|| DbError::Replication(
                format!("Table {} is not sharded", table)
            ))?);

        // Get all shards for this table
        let shard_ids: Vec<String> = sharded_table.shards.iter()
            .filter(|s| s.status == ShardStatus::Active)
            .map(|s| s.id.clone())
            .collect();

        // Create per-shard queries (in practice, would rewrite SQL for each shard)
        let mut shard_queries = HashMap::new();
        for shard_id in &shard_ids {
            shard_queries.insert(shard_id.clone(), sql.to_string());
        }

        let query = CrossShardQuery {
            query_id: format!("query-{}", uuid::Uuid::new_v4()),
            sql: sql.to_string(),
            shards: shard_ids,
            shard_queries,
            requires_aggregation: sql.to_uppercase().contains("GROUP BY") ||
                                 sql.to_uppercase().contains("COUNT") ||
                                 sql.to_uppercase().contains("SUM"),
            sort_columns: vec![],
        }));

        let mut stats = self.stats.write();
        stats.cross_shard_queries += 1;

        Ok(query)
    }

    /// Execute a cross-shard query
    pub async fn execute_cross_shard_query(
        &self,
        query: &CrossShardQuery,
    ) -> Result<Vec<Vec<u8>>> {
        let mut results = Vec::new();

        // Execute query on each shard
        for (shard_id, shard_sql) in &query.shard_queries {
            let shard_result = self.execute_on_shard(shard_id, shard_sql).await?;
            results.extend(shard_result);
        }

        // If aggregation is required, combine results
        if query.requires_aggregation {
            results = self.aggregate_results(results, query)?;
        }

        // Sort if needed
        if !query.sort_columns.is_empty() {
            results.sort();
        }

        Ok(results)
    }

    /// Execute query on a single shard
    async fn execute_on_shard(&self, shard_id: &str, _sql: &str) -> Result<Vec<Vec<u8>>> {
        let shards = self.shards.read();

        let shard = shards.get(shard_id)
            .ok_or_else(|| DbError::Replication(
                format!("Shard {} not found", shard_id)
            ))?);

        // In a real implementation, would execute on the shard's server
        // For now, return empty result
        Ok(Vec::new())
    }

    /// Aggregate results from multiple shards
    fn aggregate_results(
        &self,
        results: Vec<Vec<u8>>,
        _query: &CrossShardQuery,
    ) -> Result<Vec<Vec<u8>>> {
        // In a real implementation, would perform aggregation
        // For now, just return the results as-is
        Ok(results)
    }

    /// Create a rebalancing plan
    pub fn plan_rebalance(
        &self,
        table: &str,
        sourceshard: &str,
        targetshard: &str,
    ) -> Result<RebalancePlan> {
        let shards = self.shards.read();

        let source = shards.get(source_shard)
            .ok_or_else(|| DbError::Replication(
                format!("Source shard {} not found", source_shard)
            ))?);

        let _target = shards.get(target_shard)
            .ok_or_else(|| DbError::Replication(
                format!("Target shard {} not found", target_shard)
            ))?);

        let plan = RebalancePlan {
            id: format!("rebalance-{}", uuid::Uuid::new_v4()),
            table: table.to_string(),
            source_shard: source_shard.to_string(),
            target_shard: target_shard.to_string(),
            row_count: source.row_count / 2, // Move half the rows
            estimated_duration: source.row_count / 1000, // 1000 rows per second
            state: RebalanceState::Planned,
            progress: 0,
        }));

        self.rebalance_plans.write().insert(plan.id.clone(), plan.clone());

        Ok(plan)
    }

    /// Execute a rebalancing plan
    pub async fn execute_rebalance(&self, plan_id: &str) -> Result<()> {
        let plan = {
            let plans = self.rebalance_plans.read();
            plans.get(plan_id)
                .ok_or_else(|| DbError::Replication(
                    format!("Rebalance plan {} not found", plan_id)
                ))?
                .clone()
        }));

        // Update state to in progress
        {
            let mut plans = self.rebalance_plans.write();
            if let Some(p) = plans.get_mut(plan_id) {
                p.state = RebalanceState::InProgress;
            }
        }

        // In a real implementation, would move data between shards
        // For now, simulate with a delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Update progress
        for progress in (0..=100).step_by(10) {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let mut plans = self.rebalance_plans.write();
            if let Some(p) = plans.get_mut(plan_id) {
                p.progress = progress;
            }
        }

        // Mark as completed
        {
            let mut plans = self.rebalance_plans.write();
            if let Some(p) = plans.get_mut(plan_id) {
                p.state = RebalanceState::Completed;
                p.progress = 100;
            }
        }

        let mut stats = self.stats.write();
        stats.rebalances_completed += 1;

        Ok(())
    }

    /// Get shard key selection advice
    pub fn advise_shard_key(&self, table: &str, columns: &[String]) -> ShardKeyAdvice {
        // Analyze columns for shard key suitability
        let mut column_scores = HashMap::new();

        for column in columns {
            let mut score = 0;

            // Prefer columns with high cardinality
            if column.contains("id") || column.contains("uuid") {
                score += 10;
            }

            // Prefer columns frequently used in WHERE clauses
            if column.contains("user") || column.contains("customer") {
                score += 5;
            }

            // Avoid frequently updated columns
            if column.contains("updated_at") || column.contains("status") {
                score -= 5;
            }

            column_scores.insert(column.clone(), score);
        }

        // Sort by score
        let mut sorted: Vec<_> = column_scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let recommended_columns: Vec<_> = sorted.iter()
            .take(3)
            .map(|(col, _)| col.clone())
            .collect();

        let recommended_strategy = if recommended_columns.is_empty() {
            None
        } else {
            Some(ShardingStrategy::Hash {
                num_shards: 16,
                hash_function: HashFunction::Consistent,
            })
        };

        ShardKeyAdvice {
            table: table.to_string(),
            recommended_columns,
            recommended_strategy,
            considerations: vec![
                "Choose columns with high cardinality".to_string(),
                "Avoid frequently updated columns".to_string(),
                "Consider query patterns".to_string(),
            ],
        }
    }

    /// Get shard statistics
    #[inline]
    pub fn get_shard_stats(&self, shardid: &str) -> Result<ShardStatistics> {
        let shards = self.shards.read();

        let shard = shards.get(shard_id)
            .ok_or_else(|| DbError::Replication(
                format!("Shard {} not found", shard_id)
            ))?);

        Ok(ShardStatistics {
            shard_id: shard.id.clone(),
            row_count: shard.row_count,
            size_bytes: shard.size_bytes,
            status: shard.status.clone(),
            queries_per_second: 0.0, // Would be tracked in real implementation
            avg_query_time_ms: 0.0,
        })
    }

    /// Get overall sharding statistics
    #[inline]
    pub fn get_stats(&self) -> ShardingStats {
        self.stats.read().clone()
    }

    /// Add a new shard
    pub fn add_shard(&self, shard: Shard) -> Result<()> {
        let mut shards = self.shards.write();

        if shards.contains_key(&shard.id) {
            return Err(DbError::Replication(
                format!("Shard {} already exists", shard.id)
            ))));
        }

        shards.insert(shard.id.clone(), shard);

        let mut stats = self.stats.write();
        stats.total_shards += 1;
        stats.active_shards += 1;

        Ok(())
    }

    /// Remove a shard
    pub fn remove_shard(&self, shard_id: &str) -> Result<()> {
        let mut shards = self.shards.write();

        shards.remove(shard_id)
            .ok_or_else(|| DbError::Replication(
                format!("Shard {} not found", shard_id)
            ))?);

        let mut stats = self.stats.write();
        stats.total_shards = stats.total_shards.saturating_sub(1);
        stats.active_shards = stats.active_shards.saturating_sub(1);

        Ok(())
    }

    /// Update shard status
    pub fn update_shard_status(&self, shard_id: &str, status: ShardStatus) -> Result<()> {
        let mut shards = self.shards.write();

        let shard = shards.get_mut(shard_id)
            .ok_or_else(|| DbError::Replication(
                format!("Shard {} not found", shard_id)
            ))?);

        let old_status = shard.status.clone();
        shard.status = status.clone();

        // Update statistics
        let mut stats = self.stats.write();
        if old_status == ShardStatus::Active && status != ShardStatus::Active {
            stats.active_shards = stats.active_shards.saturating_sub(1);
        } else if old_status != ShardStatus::Active && status == ShardStatus::Active {
            stats.active_shards += 1;
        }

        Ok(())
    }
}

/// Shard key selection advice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardKeyAdvice {
    pub table: String,
    pub recommended_columns: Vec<String>,
    pub recommended_strategy: Option<ShardingStrategy>,
    pub considerations: Vec<String>,
}

/// Statistics for a single shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardStatistics {
    pub shard_id: String,
    pub row_count: u64,
    pub size_bytes: u64,
    pub status: ShardStatus,
    pub queries_per_second: f64,
    pub avg_query_time_ms: f64,
}

impl Default for ShardingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_sharding() {
        let engine = ShardingEngine::new();

        let strategy = ShardingStrategy::Hash {
            num_shards: 4,
            hash_function: HashFunction::Default,
        };

        let key = ShardKey::Integer(12345);
        let shards = engine.determine_shards(&strategy, &key).unwrap();

        assert_eq!(shards.len(), 1);
    }

    #[test]
    fn test_range_sharding() {
        let engine = ShardingEngine::new();

        let strategy = ShardingStrategy::Range {
            ranges: vec![
                RangeDefinition {
                    shard_id: "shard-0".to_string(),
                    start: ShardKey::Integer(0),
                    end: ShardKey::Integer(1000),
                },
                RangeDefinition {
                    shard_id: "shard-1".to_string(),
                    start: ShardKey::Integer(1000),
                    end: ShardKey::Integer(2000),
                },
            ],
        };

        let key = ShardKey::Integer(500);
        let shards = engine.determine_shards(&strategy, &key).unwrap();

        assert_eq!(shards.len(), 1);
        assert_eq!(shards[0], "shard-0");
    }
}
