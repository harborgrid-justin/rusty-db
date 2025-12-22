// Comprehensive Memory Management Integration Tests
//
// This module provides integration tests for all memory management optimizations
// to validate performance improvements and correctness.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use std::thread;

    use crate::enterprise_optimization::slab_tuner::*;
    use crate::enterprise_optimization::pressure_forecaster::*;
    use crate::enterprise_optimization::transaction_arena::*;
    use crate::enterprise_optimization::large_object_optimizer::*;
    use crate::memory::allocator::MemoryPressureManager;

    // ========================================================================
    // M001: Slab Allocator Tuning Tests
    // ========================================================================

    #[test]
    fn test_m001_slab_tuning_basic() {
        let allocator = TunedSlabAllocator::new(4);

        // Track common allocation patterns
        let tracker = allocator.pattern_tracker();
        tracker.track(128);  // Page header
        tracker.track(256);  // Small row
        tracker.track(512);  // Index node
        tracker.track(1024); // Large row

        assert_eq!(tracker.total_tracked(), 4);

        let frequencies = tracker.get_frequencies();
        assert!(frequencies.contains_key(&AllocationPattern::PageHeader));
        assert!(frequencies.contains_key(&AllocationPattern::SmallRow));
    }

    #[test]
    fn test_m001_allocation_pattern_frequencies() {
        let allocator = TunedSlabAllocator::new(2);
        let tracker = allocator.pattern_tracker();

        // Simulate high-frequency lock entry allocations
        for _ in 0..1000 {
            tracker.track(64); // Lock entry
        }

        // Simulate medium-frequency row allocations
        for _ in 0..500 {
            tracker.track(256); // Small row
        }

        let frequencies = tracker.get_frequencies();
        let lock_freq = frequencies.get(&AllocationPattern::LockEntry).unwrap();
        let row_freq = frequencies.get(&AllocationPattern::SmallRow).unwrap();

        assert_eq!(*lock_freq, 1000);
        assert_eq!(*row_freq, 500);
    }

    #[test]
    fn test_m001_hot_object_magazine() {
        let mut magazine = HotObjectMagazine::new(AllocationPattern::SmallRow);

        // Magazine should be optimized for small rows
        assert_eq!(magazine.capacity, AllocationPattern::SmallRow.magazine_capacity());

        // Fill magazine
        for i in 0..10 {
            assert!(magazine.free((i * 1000) as usize));
        }

        // Allocate from magazine
        assert!(magazine.allocate().is_some());
        assert!(!magazine.is_empty());
    }

    #[test]
    fn test_m001_per_cpu_cache() {
        let cache = PerCpuSlabCache::new(0, None, 64);

        // First allocation should miss
        assert!(cache.try_allocate(0).is_none());

        // Free to cache
        cache.free(0, 0x1000);

        // Second allocation should hit
        assert_eq!(cache.try_allocate(0), Some(0x1000));

        let stats = cache.stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_m001_overhead_reduction() {
        let allocator = TunedSlabAllocator::new(4);

        // Simulate many allocations
        for _ in 0..1000 {
            allocator.pattern_tracker().track(256);
        }

        let stats = allocator.tuning_stats();

        // With proper tuning, we should see high fast-path rates
        // which translate to overhead reduction
        let reduction = allocator.estimated_overhead_reduction();
        assert!(reduction >= 0.0 && reduction <= 0.20); // 0-20% reduction
    }

    // ========================================================================
    // M002: Memory Pressure Forecasting Tests
    // ========================================================================

    #[test]
    fn test_m002_pressure_forecast_basic() {
        let pm = Arc::new(MemoryPressureManager::new(1_000_000_000));
        let config = EarlyWarningConfig::default();
        let forecaster = PressureForecaster::new(pm, config);

        // Not enough samples initially
        assert!(forecaster.generate_forecast().is_none());

        // Add samples
        for i in 1..=15 {
            forecaster.record_sample(i * 50_000_000, 1_000_000_000);
            thread::sleep(Duration::from_millis(10));
        }

        // Should now be able to generate forecast
        let forecast = forecaster.generate_forecast();
        assert!(forecast.is_some());

        let f = forecast.unwrap();
        assert!(f.current_usage > 0.0);
        assert!(f.confidence > 0.0);
    }

    #[test]
    fn test_m002_trend_analysis() {
        let pm = Arc::new(MemoryPressureManager::new(1_000_000_000));
        let mut config = EarlyWarningConfig::default();
        config.min_samples_for_forecast = 5;
        let forecaster = PressureForecaster::new(pm, config);

        // Add increasing usage samples
        for i in 1..=10 {
            forecaster.record_sample(i * 80_000_000, 1_000_000_000);
            thread::sleep(Duration::from_millis(10));
        }

        let forecast = forecaster.generate_forecast().unwrap();

        // Should detect increasing trend
        assert!(
            forecast.trend == MemoryTrend::Increasing || forecast.trend == MemoryTrend::Critical
        );

        // Predicted usage should be higher than current
        assert!(forecast.predicted_60s >= forecast.current_usage);
    }

    #[test]
    fn test_m002_early_warning_thresholds() {
        let pm = Arc::new(MemoryPressureManager::new(1_000_000_000));
        let config = EarlyWarningConfig {
            warning_threshold: 0.70,
            high_threshold: 0.80,
            critical_threshold: 0.90,
            emergency_threshold: 0.95,
            ..EarlyWarningConfig::default()
        };
        let forecaster = PressureForecaster::new(pm, config.clone());

        // Add samples approaching warning threshold
        for i in 1..=15 {
            let usage = 600_000_000 + (i * 10_000_000);
            forecaster.record_sample(usage, 1_000_000_000);
            thread::sleep(Duration::from_millis(10));
        }

        let forecast = forecaster.generate_forecast().unwrap();

        // Should recommend monitoring at 70%+
        if forecast.current_usage >= config.warning_threshold {
            assert!(
                forecast.recommended_action != RecommendedAction::None,
                "Should recommend action at {}% usage",
                forecast.current_usage * 100.0
            );
        }
    }

    #[test]
    fn test_m002_forecast_statistics() {
        let pm = Arc::new(MemoryPressureManager::new(1_000_000_000));
        let mut config = EarlyWarningConfig::default();
        config.min_samples_for_forecast = 3;
        let forecaster = PressureForecaster::new(pm, config);

        // Generate multiple forecasts
        for i in 1..=10 {
            forecaster.record_sample(i * 50_000_000, 1_000_000_000);
            thread::sleep(Duration::from_millis(10));
        }

        for _ in 0..5 {
            forecaster.generate_forecast();
        }

        let stats = forecaster.stats();
        assert_eq!(stats.forecasts_generated, 5);
        assert!(stats.sample_count >= 3);
    }

    // ========================================================================
    // M003: Transaction Arena Tests
    // ========================================================================

    #[test]
    fn test_m003_transaction_arena_basic() {
        let manager = TransactionArenaManager::new();

        // Create arena for small transaction
        let arena = manager.create_arena(1, Some(50_000)).unwrap();
        assert_eq!(arena.txn_id(), 1);
        assert_eq!(arena.profile(), TransactionSizeProfile::Small);

        // Allocate memory
        let _ptr1 = arena.allocate(512).unwrap();
        let _ptr2 = arena.allocate(1024).unwrap();

        let stats = arena.stats();
        assert_eq!(stats.allocations, 2);
        assert!(stats.current_usage > 0);

        // Commit
        manager.commit_arena(1).unwrap();

        let mgr_stats = manager.stats();
        assert_eq!(mgr_stats.total_commits, 1);
        assert!(mgr_stats.fragmentation_reduction_bytes > 0);
    }

    #[test]
    fn test_m003_transaction_size_profiles() {
        let manager = TransactionArenaManager::new();

        // Create arenas with different size hints
        let tiny = manager.create_arena(1, Some(5_000)).unwrap();
        let small = manager.create_arena(2, Some(50_000)).unwrap();
        let medium = manager.create_arena(3, Some(500_000)).unwrap();
        let large = manager.create_arena(4, Some(5_000_000)).unwrap();

        assert_eq!(tiny.profile(), TransactionSizeProfile::Tiny);
        assert_eq!(small.profile(), TransactionSizeProfile::Small);
        assert_eq!(medium.profile(), TransactionSizeProfile::Medium);
        assert_eq!(large.profile(), TransactionSizeProfile::Large);

        // Cleanup
        for id in 1..=4 {
            manager.destroy_arena(id).ok();
        }
    }

    #[test]
    fn test_m003_arena_rollback() {
        let manager = TransactionArenaManager::new();
        let arena = manager.create_arena(100, None).unwrap();

        // Allocate some memory
        let _ptr1 = arena.allocate(1024).unwrap();
        let _ptr2 = arena.allocate(2048).unwrap();

        let stats_before = arena.stats();
        assert!(stats_before.current_usage > 0);

        // Rollback - should reset arena
        arena.rollback();

        let stats_after = arena.stats();
        assert_eq!(stats_after.current_usage, 0);
        assert_eq!(stats_after.rollback_count, 1);
    }

    #[test]
    fn test_m003_fragmentation_reduction() {
        let manager = TransactionArenaManager::new();

        // Create and commit many transactions
        for i in 1..=100 {
            let arena = manager.create_arena(i, Some(10_000)).unwrap();
            arena.allocate(512).ok();
            arena.allocate(1024).ok();
            manager.commit_arena(i).unwrap();
        }

        let stats = manager.stats();
        assert_eq!(stats.total_commits, 100);

        // Bulk free should reduce fragmentation by ~15%
        let expected_reduction_pct = 10.0; // At least 10%
        assert!(
            stats.fragmentation_reduction_percent >= expected_reduction_pct,
            "Expected at least {}% fragmentation reduction, got {:.2}%",
            expected_reduction_pct,
            stats.fragmentation_reduction_percent
        );
    }

    #[test]
    fn test_m003_stale_arena_cleanup() {
        let manager = TransactionArenaManager::new();

        // Create several arenas
        for i in 1..=10 {
            manager.create_arena(i, None).unwrap();
        }

        assert_eq!(manager.stats().active_arenas, 10);

        // Wait a bit
        thread::sleep(Duration::from_millis(100));

        // Cleanup with short max age
        let removed = manager.cleanup_stale_arenas(Duration::from_millis(50));
        assert!(removed > 0);
    }

    // ========================================================================
    // M004: Large Object Optimizer Tests
    // ========================================================================

    #[test]
    fn test_m004_free_region_coalescing() {
        let region1 = FreeRegion {
            address: 0x1000,
            size: 0x1000,
            huge_pages: false,
        };

        let region2 = FreeRegion {
            address: 0x2000,
            size: 0x1000,
            huge_pages: false,
        };

        // Regions should be coalesceable
        assert!(region1.can_coalesce_with(&region2));

        let coalesced = region1.coalesce_with(&region2);
        assert_eq!(coalesced.address, 0x1000);
        assert_eq!(coalesced.size, 0x2000);
        assert_eq!(coalesced.end_address(), 0x3000);
    }

    #[test]
    fn test_m004_optimizer_basic() {
        let optimizer = LargeObjectOptimizer::new(Some(2 * 1024 * 1024));

        // Allocate large object
        let size = 1024 * 1024; // 1MB
        let ptr = optimizer.allocate(size).unwrap();

        // Deallocate - should add to free list
        optimizer.deallocate(ptr, size).unwrap();

        let stats = optimizer.stats();
        assert_eq!(stats.allocations, 1);
        assert_eq!(stats.deallocations, 1);

        let info = optimizer.free_list_info();
        assert_eq!(info.total_free_bytes, size);
    }

    #[test]
    fn test_m004_coalescing_stats() {
        let optimizer = LargeObjectOptimizer::new(Some(2 * 1024 * 1024));

        // Allocate several adjacent regions (simulated)
        let size = 1024 * 1024;
        let ptr1 = optimizer.allocate(size).unwrap();
        let ptr2 = optimizer.allocate(size).unwrap();
        let ptr3 = optimizer.allocate(size).unwrap();

        // Deallocate all
        optimizer.deallocate(ptr1, size).unwrap();
        optimizer.deallocate(ptr2, size).unwrap();
        optimizer.deallocate(ptr3, size).unwrap();

        let stats = optimizer.stats();

        // Coalescing should have occurred
        assert!(stats.coalesces > 0 || stats.free_regions <= 3);
        assert!(stats.overhead_reduction_bytes > 0);
    }

    #[test]
    fn test_m004_allocation_strategies() {
        let optimizer = LargeObjectOptimizer::new(None);

        // Default should be best-fit
        assert_eq!(optimizer.strategy(), AllocationStrategy::BestFit);

        // Change strategy
        optimizer.set_strategy(AllocationStrategy::FirstFit);
        assert_eq!(optimizer.strategy(), AllocationStrategy::FirstFit);

        optimizer.set_strategy(AllocationStrategy::WorstFit);
        assert_eq!(optimizer.strategy(), AllocationStrategy::WorstFit);
    }

    #[test]
    fn test_m004_free_list_compaction() {
        let optimizer = LargeObjectOptimizer::new(None);

        // Allocate and deallocate to create fragmentation
        for _ in 0..10 {
            let ptr = optimizer.allocate(512 * 1024).unwrap();
            optimizer.deallocate(ptr, 512 * 1024).unwrap();
        }

        let regions_before = optimizer.free_list_info().region_count;

        // Compact free list
        let regions_after = optimizer.compact_free_list();

        // Should reduce fragmentation
        assert!(regions_after <= regions_before);
    }

    #[test]
    fn test_m004_overhead_reduction() {
        let optimizer = LargeObjectOptimizer::new(None);

        // Allocate many objects to build up overhead savings
        let mut ptrs = Vec::new();
        for _ in 0..50 {
            let ptr = optimizer.allocate(256 * 1024).unwrap();
            ptrs.push(ptr);
        }

        // Deallocate all
        for ptr in ptrs {
            optimizer.deallocate(ptr, 256 * 1024).unwrap();
        }

        let stats = optimizer.stats();

        // Should have some overhead reduction from coalescing
        assert!(stats.overhead_reduction_bytes > 0);
        assert!(stats.overhead_reduction_percent >= 0.0);
    }

    // ========================================================================
    // Integration Tests - All Optimizations Together
    // ========================================================================

    #[test]
    fn test_integration_memory_optimizations() {
        // Initialize all optimizers
        let slab_allocator = TunedSlabAllocator::new(4);
        let pm = Arc::new(MemoryPressureManager::new(10 * 1024 * 1024 * 1024));
        let forecaster = PressureForecaster::new(
            Arc::clone(&pm),
            EarlyWarningConfig::default(),
        );
        let arena_manager = TransactionArenaManager::new();
        let large_obj_optimizer = LargeObjectOptimizer::new(None);

        // Simulate workload
        for txn_id in 1..=100 {
            // Create transaction arena
            let arena = arena_manager.create_arena(txn_id, Some(50_000)).unwrap();

            // Simulate allocations
            slab_allocator.pattern_tracker().track(256);
            slab_allocator.pattern_tracker().track(512);

            // Record memory usage for forecasting
            forecaster.record_sample(txn_id * 10_000_000, 10 * 1024 * 1024 * 1024);

            // Simulate large object allocation
            if txn_id % 10 == 0 {
                let ptr = large_obj_optimizer.allocate(512 * 1024).unwrap();
                large_obj_optimizer.deallocate(ptr, 512 * 1024).ok();
            }

            // Commit transaction
            arena_manager.commit_arena(txn_id).unwrap();
        }

        // Validate results
        let slab_stats = slab_allocator.tuning_stats();
        let forecast = forecaster.generate_forecast();
        let arena_stats = arena_manager.stats();
        let large_stats = large_obj_optimizer.stats();

        // All optimizations should show activity
        assert!(slab_allocator.pattern_tracker().total_tracked() > 0);
        assert!(forecast.is_some());
        assert_eq!(arena_stats.total_commits, 100);
        assert!(large_stats.allocations > 0);

        // Memory efficiency improvements should be measurable
        assert!(arena_stats.fragmentation_reduction_percent > 0.0);
        assert!(large_stats.overhead_reduction_bytes > 0);
    }

    #[test]
    fn test_integration_stress_test() {
        // Stress test with concurrent operations
        let arena_manager = Arc::new(TransactionArenaManager::new());
        let large_obj_optimizer = Arc::new(LargeObjectOptimizer::new(None));

        let handles: Vec<_> = (0..4)
            .map(|thread_id| {
                let arena_mgr = Arc::clone(&arena_manager);
                let large_opt = Arc::clone(&large_obj_optimizer);

                thread::spawn(move || {
                    for i in 0..25 {
                        let txn_id = (thread_id * 100 + i) as u64;

                        // Create arena
                        let arena = arena_mgr.create_arena(txn_id, Some(10_000)).unwrap();

                        // Simulate work
                        arena.allocate(512).ok();
                        arena.allocate(1024).ok();

                        // Large allocation
                        if i % 5 == 0 {
                            let ptr = large_opt.allocate(256 * 1024).unwrap();
                            large_opt.deallocate(ptr, 256 * 1024).ok();
                        }

                        // Commit
                        arena_mgr.commit_arena(txn_id).ok();
                    }
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify no corruption or leaks
        let stats = arena_manager.stats();
        assert_eq!(stats.total_commits, 100); // 4 threads * 25 txns each
    }
}
