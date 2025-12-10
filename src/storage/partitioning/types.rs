// Core types for table partitioning

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// Partitioning strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PartitionStrategy {
    // Range partitioning - partition by value ranges
    Range {
        column: String,
        ranges: Vec<RangePartition>,
    },
    // Hash partitioning - distribute evenly using hash function
    Hash {
        column: String,
        num_partitions: usize,
    },
    // List partitioning - partition by discrete values
    List {
        column: String,
        lists: Vec<ListPartition>,
    },
    // Composite partitioning - combination of strategies
    Composite {
        primary: Box<PartitionStrategy>,
        secondary: Box<PartitionStrategy>,
    },
}

// Range partition definition
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangePartition {
    pub name: String,
    pub lower_bound: Option<String>, // None for first partition
    pub upper_bound: Option<String>, // None for last partition
}

// List partition definition
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListPartition {
    pub name: String,
    pub values: Vec<String>,
}

// Partition metadata
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMetadata {
    pub table_name: String,
    pub strategy: PartitionStrategy,
    pub created_at: SystemTime,
    pub partition_count: usize,
}

// Partition definition for adding new partitions
#[derive(Debug, Clone)]
pub enum PartitionDefinition {
    Range {
        lower: Option<String>,
        upper: Option<String>,
    },
    List {
        values: Vec<String>,
    },
}

// Query predicate for partition pruning
#[derive(Debug, Clone)]
pub struct QueryPredicate {
    pub column: String,
    pub operator: PredicateOperator,
    pub value: String,
}

// Predicate operators
#[derive(Debug, Clone)]
pub enum PredicateOperator {
    Equal,
    GreaterThan,
    LessThan,
    Between { upper: String },
}

// Partition statistics for optimization
#[derive(Debug, Clone)]
pub struct PartitionStatistics {
    pub partition_name: String,
    pub row_count: usize,
    pub data_size: usize, // In bytes
    pub min_value: String,
    pub max_value: String,
    pub last_modified: SystemTime,
}
