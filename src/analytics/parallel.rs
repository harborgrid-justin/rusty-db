// Parallel Query Execution
//
// This module provides support for parallel query processing:
//
// - **Data Partitioning**: Split data for parallel processing
// - **Parallelism Estimation**: Determine optimal worker count
// - **Hash Partitioning**: Partition by hash for joins

// =============================================================================
// Parallel Query Executor
// =============================================================================

/// Parallel query executor for analytics workloads.
///
/// Manages parallel execution of query operations across multiple workers.
pub struct ParallelQueryExecutor {
    /// Number of worker threads
    num_workers: usize,

    /// Maximum parallelism allowed
    max_parallelism: usize,
}

impl ParallelQueryExecutor {
    /// Create a new parallel executor with the given worker count.
    pub fn new(num_workers: usize) -> Self {
        Self {
            num_workers,
            max_parallelism: num_workers * 2,
        }
    }

    /// Create an executor using all available CPU cores.
    pub fn with_all_cores() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        Self::new(cores)
    }

    /// Estimate the optimal parallelism for an operation.
    ///
    /// Returns a value between 1 and `max_parallelism`.
    pub fn estimate_parallelism(&self, cardinality: u64, operation: &str) -> usize {
        let base_parallelism = match operation {
            "scan" => (cardinality / 10000).min(self.max_parallelism as u64) as usize,
            "join" => (cardinality / 50000).min(self.max_parallelism as u64) as usize,
            "aggregate" => (cardinality / 20000).min(self.max_parallelism as u64) as usize,
            "sort" => (cardinality / 30000).min(self.max_parallelism as u64) as usize,
            _ => 1,
        };

        base_parallelism.max(1).min(self.num_workers)
    }

    /// Partition data using round-robin distribution.
    pub fn partition_data(
        &self,
        data: Vec<Vec<String>>,
        numpartitions: usize,
    ) -> Vec<Vec<Vec<String>>> {
        let mut partitions = vec![Vec::new(); num_partitions];

        for (i, row) in data.into_iter().enumerate() {
            partitions[i % num_partitions].push(row);
        }

        partitions
    }

    /// Partition data using hash partitioning on a key column.
    ///
    /// This ensures rows with the same key go to the same partition,
    /// which is useful for parallel joins and aggregations.
    pub fn hash_partition(
        &self,
        data: Vec<Vec<String>>,
        key_index: usize,
        num_partitions: usize,
    ) -> Vec<Vec<Vec<String>>> {
        let mut partitions = vec![Vec::new(); num_partitions];

        for row in data {
            if let Some(key) = row.get(key_index) {
                let hash = self.hash_string(key);
                let partition = (hash as usize) % num_partitions;
                partitions[partition].push(row);
            }
        }

        partitions
    }

    /// Partition data by range on a key column.
    ///
    /// Assumes data is sorted by the key column.
    pub fn range_partition(
        &self,
        data: Vec<Vec<String>>,
        num_partitions: usize,
    ) -> Vec<Vec<Vec<String>>> {
        let chunk_size = (data.len() + num_partitions - 1) / num_partitions;

        data.chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Simple hash function for string keys.
    fn hash_string(&self, s: &str) -> u64 {
        s.bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    }

    /// Get the number of workers.
    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    /// Get the maximum parallelism.
    pub fn max_parallelism(&self) -> usize {
        self.max_parallelism
    }

    /// Set the maximum parallelism.
    pub fn set_max_parallelism(&mut self, max: usize) {
        self.max_parallelism = max;
    }
}

impl Default for ParallelQueryExecutor {
    fn default() -> Self {
        Self::new(4)
    }
}

// =============================================================================
// Partition Statistics
// =============================================================================

/// Statistics about data partitions.
#[derive(Debug, Clone)]
pub struct PartitionStats {
    /// Number of partitions
    pub num_partitions: usize,

    /// Rows per partition
    pub rows_per_partition: Vec<usize>,

    /// Minimum partition size
    pub min_size: usize,

    /// Maximum partition size
    pub max_size: usize,

    /// Average partition size
    pub avg_size: f64,

    /// Skew ratio (max/avg)
    pub skew_ratio: f64,
}

impl PartitionStats {
    /// Compute statistics for partitions.
    pub fn compute(partitions: &[Vec<Vec<String>>]) -> Self {
        let rows_per_partition: Vec<usize> = partitions.iter().map(|p| p.len()).collect();

        let min_size = rows_per_partition.iter().copied().min().unwrap_or(0);
        let max_size = rows_per_partition.iter().copied().max().unwrap_or(0);
        let total: usize = rows_per_partition.iter().sum();
        let avg_size = if partitions.is_empty() {
            0.0
        } else {
            total as f64 / partitions.len() as f64
        };

        let skew_ratio = if avg_size > 0.0 {
            max_size as f64 / avg_size
        } else {
            1.0
        };

        Self {
            num_partitions: partitions.len(),
            rows_per_partition,
            min_size,
            max_size,
            avg_size,
            skew_ratio,
        }
    }

    /// Check if partitions are well-balanced.
    pub fn is_balanced(&self, threshold: f64) -> bool {
        self.skew_ratio <= threshold
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(n: usize) -> Vec<Vec<String>> {
        (0..n)
            .map(|i| vec![i.to_string(), format!("value_{}", i)])
            .collect()
    }

    #[test]
    fn test_parallelism_estimation() {
        let executor = ParallelQueryExecutor::new(4);

        // Small data should use 1 worker
        assert_eq!(executor.estimate_parallelism(1000, "scan"), 1);

        // Larger data should use more workers
        assert!(executor.estimate_parallelism(100000, "scan") > 1);
    }

    #[test]
    fn test_round_robin_partition() {
        let executor = ParallelQueryExecutor::new(4);
        let data = make_data(10);

        let partitions = executor.partition_data(data, 3);

        assert_eq!(partitions.len(), 3);
        assert_eq!(partitions[0].len(), 4); // 0, 3, 6, 9
        assert_eq!(partitions[1].len(), 3); // 1, 4, 7
        assert_eq!(partitions[2].len(), 3); // 2, 5, 8
    }

    #[test]
    fn test_hash_partition() {
        let executor = ParallelQueryExecutor::new(4);
        let data = make_data(100);

        let partitions = executor.hash_partition(data, 0, 4);

        assert_eq!(partitions.len(), 4);

        // All partitions should have some data
        for partition in &partitions {
            assert!(!partition.is_empty());
        }

        // Total should equal original
        let total: usize = partitions.iter().map(|p| p.len()).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn test_range_partition() {
        let executor = ParallelQueryExecutor::new(4);
        let data = make_data(10);

        let partitions = executor.range_partition(data, 3);

        assert_eq!(partitions.len(), 3);

        // First partition should have smaller values
        assert_eq!(partitions[0][0][0], "0");
    }

    #[test]
    fn test_partition_stats() {
        let partitions = vec![
            vec![vec!["1".to_string()], vec!["2".to_string()]],
            vec![vec!["3".to_string()]],
            vec![
                vec!["4".to_string()],
                vec!["5".to_string()],
                vec!["6".to_string()],
            ],
        ];

        let stats = PartitionStats::compute(&partitions);

        assert_eq!(stats.num_partitions, 3);
        assert_eq!(stats.min_size, 1);
        assert_eq!(stats.max_size, 3);
        assert!((stats.avg_size - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_balance_check() {
        let balanced = vec![
            vec![vec!["1".to_string()], vec!["2".to_string()]],
            vec![vec!["3".to_string()], vec!["4".to_string()]],
        ];

        let stats = PartitionStats::compute(&balanced);
        assert!(stats.is_balanced(1.5));
    }
}
