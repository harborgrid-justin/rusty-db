/// Table Partitioning Support
///
/// This module provides comprehensive table partitioning capabilities:
/// - Range partitioning (by date, number ranges)
/// - Hash partitioning (for even distribution)
/// - List partitioning (by discrete values)
/// - Composite partitioning (combination of strategies)
/// - Partition pruning optimization
/// - Dynamic partition management

pub mod types;
pub mod manager;
pub mod pruning;
pub mod operations;
pub mod execution;
pub mod optimizer;

// Re-export main types
pub use types::*;
pub use manager::{PartitionManager, PartitionStatsManager, PartitionMerger, PartitionSplitter};
pub use pruning::{PartitionPruner, advanced};
pub use operations::*;
pub use execution::*;
pub use optimizer::*;

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_create_range_partition() {
        let mut manager = PartitionManager::new();

        let strategy = PartitionStrategy::Range {
            column: "date".to_string(),
            ranges: vec![
                RangePartition {
                    name: "p_2023".to_string(),
                    lower_bound: Some("2023-01-01".to_string()),
                    upper_bound: Some("2024-01-01".to_string()),
                },
                RangePartition {
                    name: "p_2024".to_string(),
                    lower_bound: Some("2024-01-01".to_string()),
                    upper_bound: None,
                },
            ],
        };

        assert!(manager.create_partitioned_table("sales".to_string(), strategy).is_ok());

        let partition = manager.get_partition_for_value("sales", "2023-06-15").unwrap();
        assert_eq!(partition, "p_2023");

        let partition = manager.get_partition_for_value("sales", "2024-06-15").unwrap();
        assert_eq!(partition, "p_2024");
    }

    #[test]
    fn test_hash_partition() {
        let mut manager = PartitionManager::new();

        let strategy = PartitionStrategy::Hash {
            column: "id".to_string(),
            num_partitions: 4,
        };

        assert!(manager.create_partitioned_table("users".to_string(), strategy).is_ok());

        let partition = manager.get_partition_for_value("users", "12345").unwrap();
        assert!(partition.starts_with("partition_"));
    }

    #[test]
    fn test_list_partition() {
        let mut manager = PartitionManager::new();

        let strategy = PartitionStrategy::List {
            column: "region".to_string(),
            lists: vec![
                ListPartition {
                    name: "p_west".to_string(),
                    values: vec!["CA".to_string(), "OR".to_string(), "WA".to_string()],
                },
                ListPartition {
                    name: "p_east".to_string(),
                    values: vec!["NY".to_string(), "MA".to_string(), "FL".to_string()],
                },
            ],
        };

        assert!(manager.create_partitioned_table("stores".to_string(), strategy).is_ok());

        let partition = manager.get_partition_for_value("stores", "CA").unwrap();
        assert_eq!(partition, "p_west");

        let partition = manager.get_partition_for_value("stores", "NY").unwrap();
        assert_eq!(partition, "p_east");
    }

    #[test]
    fn test_add_partition() {
        let mut manager = PartitionManager::new();

        let strategy = PartitionStrategy::Range {
            column: "date".to_string(),
            ranges: vec![],
        };

        manager.create_partitioned_table("events".to_string(), strategy).unwrap();

        let def = PartitionDefinition::Range {
            lower: Some("2023-01-01".to_string()),
            upper: Some("2024-01-01".to_string()),
        };

        assert!(manager.add_partition("events", "p_2023".to_string(), def).is_ok());

        let partitions = manager.list_partitions("events").unwrap();
        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0], "p_2023");
    }

    #[test]
    fn test_partition_pruning_range() {
        let metadata = PartitionMetadata {
            table_name: "sales".to_string(),
            strategy: PartitionStrategy::Range {
                column: "date".to_string(),
                ranges: vec![
                    RangePartition {
                        name: "p_jan".to_string(),
                        lower_bound: Some("2023-01-01".to_string()),
                        upper_bound: Some("2023-02-01".to_string()),
                    },
                    RangePartition {
                        name: "p_feb".to_string(),
                        lower_bound: Some("2023-02-01".to_string()),
                        upper_bound: Some("2023-03-01".to_string()),
                    },
                ],
            },
            created_at: SystemTime::now(),
            partition_count: 2,
        };

        let predicate = QueryPredicate {
            column: "date".to_string(),
            operator: PredicateOperator::Equal,
            value: "2023-01-15".to_string(),
        };

        let pruned = PartitionPruner::prune_partitions(&metadata, &predicate);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0], "p_jan");
    }

    #[test]
    fn test_partition_stats() {
        let mut stats_mgr = PartitionStatsManager::new();

        stats_mgr.update_stats(
            "sales".to_string(),
            "p_2023".to_string(),
            1000,
            1024000,
        );

        let stats = stats_mgr.get_stats("sales", "p_2023");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().row_count, 1000);
    }
}
