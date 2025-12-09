/// Partition Cost Model and Optimizer

use super::types::*;
use std::collections::HashMap;

/// Partition access cost estimator
pub struct PartitionCostEstimator {
    io_cost_per_page: f64,
    cpu_cost_per_row: f64,
}

impl PartitionCostEstimator {
    pub fn new() -> Self {
        Self {
            io_cost_per_page: 1.0,
            cpu_cost_per_row: 0.01,
        }
    }

    pub fn estimate_access_cost(
        &self,
        partitions: &[String],
        stats: &HashMap<String, PartitionStatistics>,
    ) -> f64 {
        let mut total_cost = 0.0;

        for partition in partitions {
            if let Some(partition_stats) = stats.get(partition) {
                let io_cost = (partition_stats.data_size / 4096) as f64 * self.io_cost_per_page;
                let cpu_cost = partition_stats.row_count as f64 * self.cpu_cost_per_row;
                total_cost += io_cost + cpu_cost;
            }
        }

        total_cost
    }

    pub fn estimate_join_cost(
        &self,
        left_partitions: usize,
        right_partitions: usize,
        partition_wise: bool,
    ) -> f64 {
        if partition_wise && left_partitions == right_partitions {
            (left_partitions + right_partitions) as f64 * 100.0
        } else {
            (left_partitions * right_partitions) as f64 * 200.0
        }
    }
}

impl Default for PartitionCostEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Partition strategy recommender
pub struct PartitionStrategyRecommender {
    workload_patterns: Vec<WorkloadPattern>,
}

#[derive(Debug, Clone)]
pub struct WorkloadPattern {
    pub query_type: QueryType,
    pub access_pattern: AccessPattern,
    pub frequency: f64,
}

#[derive(Debug, Clone)]
pub enum QueryType {
    PointQuery,
    RangeScan,
    FullScan,
    Join,
    Aggregate,
}

#[derive(Debug, Clone)]
pub enum AccessPattern {
    Sequential,
    Random,
    ByDate,
    ByKey,
}

impl PartitionStrategyRecommender {
    pub fn new() -> Self {
        Self {
            workload_patterns: Vec::new(),
        }
    }

    pub fn add_workload_pattern(&mut self, pattern: WorkloadPattern) {
        self.workload_patterns.push(pattern);
    }

    pub fn recommend_strategy(&self, column: &str) -> PartitionStrategy {
        let has_date_access = self.workload_patterns
            .iter()
            .any(|p| matches!(p.access_pattern, AccessPattern::ByDate));

        let has_key_access = self.workload_patterns
            .iter()
            .any(|p| matches!(p.access_pattern, AccessPattern::ByKey));

        if has_date_access {
            PartitionStrategy::Range {
                column: column.to_string(),
                ranges: Vec::new(),
            }
        } else if has_key_access {
            PartitionStrategy::Hash {
                column: column.to_string(),
                num_partitions: 16,
            }
        } else {
            PartitionStrategy::Hash {
                column: column.to_string(),
                num_partitions: 8,
            }
        }
    }
}

impl Default for PartitionStrategyRecommender {
    fn default() -> Self {
        Self::new()
    }
}

/// Smart partition router
pub struct PartitionRouter {
    routing_cache: HashMap<String, Vec<String>>,
}

impl PartitionRouter {
    pub fn new() -> Self {
        Self {
            routing_cache: HashMap::new(),
        }
    }

    pub fn route_query(
        &mut self,
        table: &str,
        query_predicates: &[String],
        all_partitions: &[String],
        _strategy: &PartitionStrategy,
    ) -> Vec<String> {
        let cache_key = format!("{}:{:?}", table, query_predicates);
        if let Some(cached) = self.routing_cache.get(&cache_key) {
            return cached.clone();
        }

        let target_partitions = all_partitions.to_vec();
        self.routing_cache.insert(cache_key, target_partitions.clone());
        target_partitions
    }

    pub fn clear_cache(&mut self) {
        self.routing_cache.clear();
    }
}

impl Default for PartitionRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Partition SQL generator
pub struct PartitionSqlGenerator;

impl PartitionSqlGenerator {
    pub fn generate_create_table_sql(
        table_name: &str,
        columns: &[(String, String)],
        strategy: &PartitionStrategy,
    ) -> String {
        let mut sql = format!("CREATE TABLE {} (\n", table_name);

        for (i, (col_name, col_type)) in columns.iter().enumerate() {
            sql.push_str(&format!("    {} {}", col_name, col_type));
            if i < columns.len() - 1 {
                sql.push_str(",\n");
            }
        }

        sql.push_str("\n) ");
        sql.push_str(&Self::strategy_to_sql(strategy));
        sql.push(';');

        sql
    }

    fn strategy_to_sql(strategy: &PartitionStrategy) -> String {
        match strategy {
            PartitionStrategy::Range { column, ranges } => {
                let mut sql = format!("PARTITION BY RANGE ({})", column);
                if !ranges.is_empty() {
                    sql.push_str(" (\n");
                    for (i, range) in ranges.iter().enumerate() {
                        sql.push_str(&format!("    PARTITION {} VALUES LESS THAN (", range.name));
                        if let Some(ref upper) = range.upper_bound {
                            sql.push_str(upper);
                        } else {
                            sql.push_str("MAXVALUE");
                        }
                        sql.push(')');
                        if i < ranges.len() - 1 {
                            sql.push_str(",\n");
                        }
                    }
                    sql.push_str("\n)");
                }
                sql
            }
            PartitionStrategy::Hash { column, num_partitions } => {
                format!("PARTITION BY HASH ({}) PARTITIONS {}", column, num_partitions)
            }
            PartitionStrategy::List { column, lists } => {
                let mut sql = format!("PARTITION BY LIST ({})", column);
                if !lists.is_empty() {
                    sql.push_str(" (\n");
                    for (i, list) in lists.iter().enumerate() {
                        sql.push_str(&format!("    PARTITION {} VALUES IN (", list.name));
                        sql.push_str(&list.values.join(", "));
                        sql.push(')');
                        if i < lists.len() - 1 {
                            sql.push_str(",\n");
                        }
                    }
                    sql.push_str("\n)");
                }
                sql
            }
            PartitionStrategy::Composite { primary, .. } => {
                Self::strategy_to_sql(primary)
            }
        }
    }
}

/// Partition metadata validator
pub struct PartitionValidator;

impl PartitionValidator {
    pub fn validate_strategy(strategy: &PartitionStrategy) -> crate::error::Result<()> {
        use crate::error::DbError;

        match strategy {
            PartitionStrategy::Range { column, ranges } => {
                if column.is_empty() {
                    return Err(DbError::InvalidInput(
                        "Partition column cannot be empty".to_string()
                    ));
                }
                for range in ranges {
                    if range.name.is_empty() {
                        return Err(DbError::InvalidInput(
                            "Partition name cannot be empty".to_string()
                        ));
                    }
                }
            }
            PartitionStrategy::Hash { column, num_partitions } => {
                if column.is_empty() {
                    return Err(DbError::InvalidInput(
                        "Partition column cannot be empty".to_string()
                    ));
                }
                if *num_partitions == 0 {
                    return Err(DbError::InvalidInput(
                        "Number of partitions must be > 0".to_string()
                    ));
                }
            }
            PartitionStrategy::List { column, lists } => {
                if column.is_empty() {
                    return Err(DbError::InvalidInput(
                        "Partition column cannot be empty".to_string()
                    ));
                }
                for list in lists {
                    if list.name.is_empty() {
                        return Err(DbError::InvalidInput(
                            "Partition name cannot be empty".to_string()
                        ));
                    }
                    if list.values.is_empty() {
                        return Err(DbError::InvalidInput(format!(
                            "Partition '{}' must have at least one value",
                            list.name
                        )));
                    }
                }
            }
            PartitionStrategy::Composite { primary, secondary } => {
                Self::validate_strategy(primary)?;
                Self::validate_strategy(secondary)?;
            }
        }
        Ok(())
    }
}
