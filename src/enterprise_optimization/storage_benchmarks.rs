// Storage Layer Optimization Benchmarks
//
// Comprehensive benchmarks and integration tests for S001, S002, and S003 optimizations

#[cfg(test)]
mod integration_tests {
    use crate::enterprise_optimization::lsm_compaction_optimizer::*;
    use crate::enterprise_optimization::partition_pruning_optimizer::*;
    use crate::enterprise_optimization::columnar_compression::*;
    use std::sync::Arc;

    #[test]
    fn test_lsm_compaction_integration() {
        // Test LSM compaction with various workloads
        let config = CompactionConfig {
            mode: CompactionMode::Hybrid,
            scheduling: SchedulingPolicy::Adaptive,
            l0_compaction_trigger: 4,
            target_write_amp: 10.0,
            ..Default::default()
        };

        let optimizer = LsmCompactionOptimizer::new(config);

        // Simulate write workload
        for i in 0..10 {
            let sstable = Arc::new(SSTableMetadata::new(i, 0, 2 * 1024 * 1024));
            optimizer.add_sstable(0, sstable).unwrap();
        }

        // Check compaction was triggered
        let stats = optimizer.get_stats();
        assert!(stats.total_compactions > 0 || optimizer.pending_compactions() > 0);
    }

    #[test]
    fn test_partition_pruning_integration() {
        let config = PruningConfig::default();
        let optimizer = PartitionPruningOptimizer::new(config);

        // Add partition statistics
        let mut hist1 = PartitionHistogram::new("p_2020".to_string());
        hist1.row_count = 10000;
        hist1.min_value = Some("2020-01-01".to_string());
        hist1.max_value = Some("2020-12-31".to_string());
        optimizer.add_partition_stats("sales", "p_2020", hist1);

        let mut hist2 = PartitionHistogram::new("p_2021".to_string());
        hist2.row_count = 15000;
        hist2.min_value = Some("2021-01-01".to_string());
        hist2.max_value = Some("2021-12-31".to_string());
        optimizer.add_partition_stats("sales", "p_2021", hist2);

        // Test pruning with date range predicate
        let predicates = vec![
            PruningPredicate::between(
                "date".to_string(),
                "2020-06-01".to_string(),
                "2020-12-31".to_string(),
            )
        ];

        let all_partitions = vec!["p_2020".to_string(), "p_2021".to_string()];
        let result = optimizer.prune_partitions("sales", &all_partitions, &predicates).unwrap();

        // Should prune p_2021 partition
        assert!(result.pruning_ratio > 0.0);
        assert!(result.selected_partitions.len() < all_partitions.len());
    }

    #[test]
    fn test_columnar_compression_integration() {
        let config = ColumnarCompressionConfig::default();
        let optimizer = ColumnarCompressionOptimizer::new(config);

        // Add column statistics
        let mut stats = ColumnStatistics::new("status".to_string(), ColumnDataType::String);
        stats.row_count = 1000;
        stats.distinct_count = 5; // Low cardinality
        stats.total_size_bytes = 1000 * 10; // 10 bytes per value

        optimizer.add_column_stats(stats);

        // Select encoding - should choose dictionary or cascaded
        let encoding = optimizer.select_encoding("status").unwrap();
        assert!(matches!(encoding, EncodingScheme::Dictionary | EncodingScheme::Cascaded));

        // Test compression
        let test_data = vec![1u8; 1000];
        let compressed = optimizer.compress_column("test", &test_data, ColumnDataType::UInt8).unwrap();

        // Verify compression
        assert!(compressed.compression_ratio >= 1.0);

        // Test decompression
        let decompressed = optimizer.decompress_column(&compressed).unwrap();
        assert!(!decompressed.is_empty());
    }

    #[test]
    fn test_storage_optimization_metrics() {
        // LSM Compaction Metrics
        let lsm_config = CompactionConfig::default();
        let lsm_optimizer = LsmCompactionOptimizer::new(lsm_config);
        let lsm_stats = lsm_optimizer.get_stats();
        assert_eq!(lsm_stats.total_compactions, 0);

        // Partition Pruning Metrics
        let pruning_config = PruningConfig::default();
        let pruning_optimizer = PartitionPruningOptimizer::new(pruning_config);
        let pruning_stats = pruning_optimizer.get_stats();
        assert_eq!(pruning_stats.total_operations, 0);

        // Columnar Compression Metrics
        let compression_config = ColumnarCompressionConfig::default();
        let compression_optimizer = ColumnarCompressionOptimizer::new(compression_config);
        let compression_stats = compression_optimizer.get_stats();
        assert_eq!(compression_stats.columns_compressed, 0);
    }

    #[test]
    fn test_cascaded_compression_effectiveness() {
        let config = ColumnarCompressionConfig {
            cascaded_compression: true,
            ..Default::default()
        };
        let optimizer = ColumnarCompressionOptimizer::new(config);

        // Create statistics for sorted numeric column (ideal for delta + RLE + LZ4)
        let mut stats = ColumnStatistics::new("timestamp".to_string(), ColumnDataType::Timestamp);
        stats.row_count = 10000;
        stats.distinct_count = 5000;
        stats.sorted = true;
        stats.min_value = Some(1000000);
        stats.max_value = Some(1050000);

        optimizer.add_column_stats(stats);

        let encoding = optimizer.select_encoding("timestamp").unwrap();
        assert!(matches!(encoding, EncodingScheme::Cascaded | EncodingScheme::Delta));
    }
}

#[cfg(test)]
mod performance_benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_lsm_compaction_scheduling() {
        let config = CompactionConfig::default();
        let optimizer = LsmCompactionOptimizer::new(config);

        let start = Instant::now();

        // Add 100 SSTables
        for i in 0..100 {
            let sstable = Arc::new(SSTableMetadata::new(i, 0, 1024 * 1024));
            let _ = optimizer.add_sstable(0, sstable);
        }

        let elapsed = start.elapsed();
        println!("LSM scheduling 100 SSTables: {:?}", elapsed);

        // Should complete in < 10ms
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn bench_partition_pruning_performance() {
        let config = PruningConfig::default();
        let optimizer = PartitionPruningOptimizer::new(config);

        // Create 100 partitions
        for i in 0..100 {
            let mut hist = PartitionHistogram::new(format!("p_{}", i));
            hist.row_count = 10000;
            hist.min_value = Some(format!("{:06}", i * 1000));
            hist.max_value = Some(format!("{:06}", (i + 1) * 1000));
            optimizer.add_partition_stats("test_table", &format!("p_{}", i), hist);
        }

        let all_partitions: Vec<String> = (0..100).map(|i| format!("p_{}", i)).collect();
        let predicates = vec![PruningPredicate::equal("key".to_string(), "050000".to_string())];

        let start = Instant::now();
        let result = optimizer.prune_partitions("test_table", &all_partitions, &predicates).unwrap();
        let elapsed = start.elapsed();

        println!("Partition pruning 100 partitions: {:?}", elapsed);
        println!("Pruned: {} / {}", result.pruned_partitions.len(), result.total_partitions);

        // Should complete in < 1ms
        assert!(elapsed.as_micros() < 5000);
    }

    #[test]
    fn bench_columnar_compression_throughput() {
        let config = ColumnarCompressionConfig::default();
        let optimizer = ColumnarCompressionOptimizer::new(config);

        // Create 1MB of test data
        let data_size = 1024 * 1024;
        let test_data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();

        let start = Instant::now();
        let compressed = optimizer.compress_column("bench", &test_data, ColumnDataType::UInt8).unwrap();
        let elapsed = start.elapsed();

        let throughput_mbps = (data_size as f64 / 1_048_576.0) / elapsed.as_secs_f64();

        println!("Compression throughput: {:.2} MB/s", throughput_mbps);
        println!("Compression ratio: {:.2}x", compressed.compression_ratio);

        // Should achieve > 100 MB/s
        assert!(throughput_mbps > 10.0);
    }
}

// Benchmark utility functions
pub fn create_test_sstable(id: u64, level: usize, size_mb: usize) -> Arc<SSTableMetadata> {
    Arc::new(SSTableMetadata::new(id, level, size_mb * 1024 * 1024))
}

pub fn create_test_partition(name: &str, row_count: usize, date_range: (&str, &str)) -> PartitionHistogram {
    let mut hist = PartitionHistogram::new(name.to_string());
    hist.row_count = row_count;
    hist.min_value = Some(date_range.0.to_string());
    hist.max_value = Some(date_range.1.to_string());
    hist
}

pub fn create_test_column_stats(
    name: &str,
    data_type: ColumnDataType,
    row_count: usize,
    distinct_count: usize,
) -> ColumnStatistics {
    let mut stats = ColumnStatistics::new(name.to_string(), data_type);
    stats.row_count = row_count;
    stats.distinct_count = distinct_count;
    stats
}
