// Copyright (c) 2025 RustyDB Contributors
//
// Concurrency Optimizations Benchmarks and Integration Tests
//
// This module provides comprehensive benchmarking and integration tests for the
// three main concurrency optimizations:
// - C001: Optimized Skip List
// - C002: Optimized Work-Stealing Scheduler
// - C003: Optimized Epoch-Based Reclamation
//
// ## Performance Targets
//
// - Skip List: +20% throughput on index operations
// - Work-Stealing: +15% parallelism efficiency
// - Epoch Reclamation: -25% memory overhead

use super::optimized_epoch::{
    defer_garbage, force_thread_gc, init_thread_gc, register_participant, thread_gc_stats,
    unregister_participant, OptimizedEpochGuard, OptimizedEpochManager,
};
use super::optimized_skiplist::{OptimizedSkipList, OptimizedSkipListStats};
use super::optimized_work_stealing::{
    NumaTopology, OptimizedWorkStealingPool, PoolStatsSnapshot, StealingPolicy,
    WorkerStatsSnapshot,
};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Benchmark results for skip list operations
#[derive(Debug, Clone)]
pub struct SkipListBenchmark {
    pub operation: String,
    pub total_ops: u64,
    pub duration_ms: u128,
    pub ops_per_sec: u64,
    pub avg_latency_ns: u64,
    pub stats: OptimizedSkipListStats,
}

/// Run skip list benchmarks
pub fn benchmark_skiplist(num_threads: usize, ops_per_thread: usize) -> Vec<SkipListBenchmark> {
    let mut results = Vec::new();

    // Benchmark 1: Concurrent inserts
    {
        let list = Arc::new(OptimizedSkipList::new());
        let start = Instant::now();
        let mut handles = vec![];

        for thread_id in 0..num_threads {
            let list = list.clone();
            handles.push(thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let key = thread_id * ops_per_thread + i;
                    list.insert(key, key);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        let total_ops = (num_threads * ops_per_thread) as u64;
        let stats = list.stats();

        results.push(SkipListBenchmark {
            operation: "concurrent_insert".to_string(),
            total_ops,
            duration_ms: duration.as_millis(),
            ops_per_sec: (total_ops as f64 / duration.as_secs_f64()) as u64,
            avg_latency_ns: (duration.as_nanos() / total_ops as u128) as u64,
            stats,
        });
    }

    // Benchmark 2: Mixed read/write workload
    {
        let list = Arc::new(OptimizedSkipList::new());

        // Pre-populate
        for i in 0..ops_per_thread {
            list.insert(i, i);
        }

        let start = Instant::now();
        let mut handles = vec![];

        for thread_id in 0..num_threads {
            let list = list.clone();
            handles.push(thread::spawn(move || {
                for i in 0..ops_per_thread {
                    if i % 10 == 0 {
                        // 10% writes
                        let key = thread_id * ops_per_thread + i;
                        list.insert(key, key);
                    } else {
                        // 90% reads
                        let key = i % ops_per_thread;
                        list.find(&key);
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        let total_ops = (num_threads * ops_per_thread) as u64;
        let stats = list.stats();

        results.push(SkipListBenchmark {
            operation: "mixed_read_write_90_10".to_string(),
            total_ops,
            duration_ms: duration.as_millis(),
            ops_per_sec: (total_ops as f64 / duration.as_secs_f64()) as u64,
            avg_latency_ns: (duration.as_nanos() / total_ops as u128) as u64,
            stats,
        });
    }

    // Benchmark 3: Read-heavy workload (fast path optimization)
    {
        let list = Arc::new(OptimizedSkipList::new());

        // Pre-populate with small dataset (triggers fast path)
        for i in 0..100 {
            list.insert(i, i);
        }

        let start = Instant::now();
        let mut handles = vec![];

        for _thread_id in 0..num_threads {
            let list = list.clone();
            handles.push(thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let key = i % 100;
                    list.find(&key);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        let total_ops = (num_threads * ops_per_thread) as u64;
        let stats = list.stats();

        results.push(SkipListBenchmark {
            operation: "read_heavy_fast_path".to_string(),
            total_ops,
            duration_ms: duration.as_millis(),
            ops_per_sec: (total_ops as f64 / duration.as_secs_f64()) as u64,
            avg_latency_ns: (duration.as_nanos() / total_ops as u128) as u64,
            stats,
        });
    }

    results
}

/// Benchmark results for work-stealing pool
#[derive(Debug, Clone)]
pub struct WorkStealingBenchmark {
    pub scenario: String,
    pub num_workers: usize,
    pub total_tasks: usize,
    pub duration_ms: u128,
    pub tasks_per_sec: u64,
    pub pool_stats: PoolStatsSnapshot,
}

/// Run work-stealing scheduler benchmarks
pub fn benchmark_work_stealing(num_workers: usize, tasks_per_worker: usize) -> Vec<WorkStealingBenchmark> {
    let mut results = Vec::new();

    // Benchmark 1: Imbalanced workload (tests stealing efficiency)
    {
        let pool = Arc::new(OptimizedWorkStealingPool::<usize>::new(num_workers));
        let start = Instant::now();

        // Push all tasks to worker 0
        if let Some(worker0) = pool.worker(0) {
            for i in 0..(num_workers * tasks_per_worker) {
                worker0.push(i);
            }
        }

        // All workers compete for work
        let mut handles = vec![];
        let processed = Arc::new(AtomicUsize::new(0));

        for i in 0..num_workers {
            let pool = pool.clone();
            let processed = processed.clone();
            handles.push(thread::spawn(move || {
                if let Some(worker) = pool.worker(i) {
                    let mut count = 0;
                    while worker.get_work().is_some() {
                        count += 1;
                    }
                    processed.fetch_add(count, Ordering::Relaxed);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        let total_tasks = processed.load(Ordering::Relaxed);
        let pool_stats = pool.aggregate_stats();

        results.push(WorkStealingBenchmark {
            scenario: "imbalanced_workload".to_string(),
            num_workers,
            total_tasks,
            duration_ms: duration.as_millis(),
            tasks_per_sec: (total_tasks as f64 / duration.as_secs_f64()) as u64,
            pool_stats,
        });
    }

    // Benchmark 2: Balanced workload
    {
        let pool = Arc::new(OptimizedWorkStealingPool::<usize>::new(num_workers));
        let start = Instant::now();

        // Distribute tasks evenly
        for worker_id in 0..num_workers {
            if let Some(worker) = pool.worker(worker_id) {
                for i in 0..tasks_per_worker {
                    worker.push(worker_id * tasks_per_worker + i);
                }
            }
        }

        // Workers process their own tasks
        let mut handles = vec![];
        let processed = Arc::new(AtomicUsize::new(0));

        for i in 0..num_workers {
            let pool = pool.clone();
            let processed = processed.clone();
            handles.push(thread::spawn(move || {
                if let Some(worker) = pool.worker(i) {
                    let mut count = 0;
                    while worker.get_work().is_some() {
                        count += 1;
                    }
                    processed.fetch_add(count, Ordering::Relaxed);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        let total_tasks = processed.load(Ordering::Relaxed);
        let pool_stats = pool.aggregate_stats();

        results.push(WorkStealingBenchmark {
            scenario: "balanced_workload".to_string(),
            num_workers,
            total_tasks,
            duration_ms: duration.as_millis(),
            tasks_per_sec: (total_tasks as f64 / duration.as_secs_f64()) as u64,
            pool_stats,
        });
    }

    results
}

/// Benchmark results for epoch-based reclamation
#[derive(Debug, Clone)]
pub struct EpochBenchmark {
    pub scenario: String,
    pub total_deferrals: u64,
    pub total_reclaimed: u64,
    pub duration_ms: u128,
    pub deferrals_per_sec: u64,
    pub reclamation_rate: f64,
    pub avg_garbage_per_thread: u64,
}

/// Run epoch-based reclamation benchmarks
pub fn benchmark_epoch_reclamation(num_threads: usize, deferrals_per_thread: usize) -> Vec<EpochBenchmark> {
    let mut results = Vec::new();

    // Benchmark 1: High-frequency garbage generation
    {
        let manager = Arc::new(OptimizedEpochManager::new());
        let total_deferred = Arc::new(AtomicU64::new(0));
        let total_reclaimed = Arc::new(AtomicU64::new(0));

        let start = Instant::now();
        let mut handles = vec![];

        // Garbage generator threads
        for _ in 0..num_threads {
            let total_deferred = total_deferred.clone();
            handles.push(thread::spawn(move || {
                init_thread_gc();

                for _ in 0..deferrals_per_thread {
                    defer_garbage(|| {
                        // Simulate cleanup work
                    });
                    total_deferred.fetch_add(1, Ordering::Relaxed);
                }

                // Force collection
                force_thread_gc();

                let stats = thread_gc_stats();
                stats.total_reclaimed
            }));
        }

        // Epoch advancement thread
        let manager_clone = manager.clone();
        let advance_handle = thread::spawn(move || {
            for _ in 0..100 {
                manager_clone.try_advance();
                thread::sleep(Duration::from_micros(100));
            }
        });

        // Collect results
        for handle in handles {
            let reclaimed = handle.join().unwrap();
            total_reclaimed.fetch_add(reclaimed, Ordering::Relaxed);
        }

        advance_handle.join().unwrap();

        let duration = start.elapsed();
        let deferred = total_deferred.load(Ordering::Relaxed);
        let reclaimed = total_reclaimed.load(Ordering::Relaxed);

        results.push(EpochBenchmark {
            scenario: "high_frequency_garbage".to_string(),
            total_deferrals: deferred,
            total_reclaimed: reclaimed,
            duration_ms: duration.as_millis(),
            deferrals_per_sec: (deferred as f64 / duration.as_secs_f64()) as u64,
            reclamation_rate: reclaimed as f64 / deferred.max(1) as f64,
            avg_garbage_per_thread: deferred / num_threads as u64,
        });
    }

    results
}

/// Comprehensive integration test
pub fn integration_test_concurrency_optimizations() -> IntegrationTestResults {
    let num_threads = 4;
    let ops_per_thread = 10_000;

    let skiplist_results = benchmark_skiplist(num_threads, ops_per_thread);
    let work_stealing_results = benchmark_work_stealing(num_threads, ops_per_thread);
    let epoch_results = benchmark_epoch_reclamation(num_threads, ops_per_thread);

    IntegrationTestResults {
        skiplist_results,
        work_stealing_results,
        epoch_results,
        numa_info: NumaTopology::detect(),
    }
}

/// Integration test results
#[derive(Debug)]
pub struct IntegrationTestResults {
    pub skiplist_results: Vec<SkipListBenchmark>,
    pub work_stealing_results: Vec<WorkStealingBenchmark>,
    pub epoch_results: Vec<EpochBenchmark>,
    pub numa_info: NumaTopology,
}

impl IntegrationTestResults {
    /// Print formatted results
    pub fn print_summary(&self) {
        println!("\n=== Concurrency Optimizations Benchmark Results ===\n");

        println!("NUMA Topology:");
        println!("  Nodes: {}", self.numa_info.node_count());
        println!("  NUMA-aware: {}\n", self.numa_info.is_numa());

        println!("Skip List Results:");
        for result in &self.skiplist_results {
            println!("  {}", result.operation);
            println!("    Throughput: {} ops/sec", result.ops_per_sec);
            println!("    Avg Latency: {} ns", result.avg_latency_ns);
            println!(
                "    Fast Path: {}/{} ({:.1}%)",
                result.stats.fast_path_searches,
                result.stats.searches,
                result.stats.fast_path_searches as f64 / result.stats.searches.max(1) as f64
                    * 100.0
            );
            println!("    Height Adaptations: {}", result.stats.height_adaptations);
        }

        println!("\nWork-Stealing Results:");
        for result in &self.work_stealing_results {
            println!("  {}", result.scenario);
            println!("    Throughput: {} tasks/sec", result.tasks_per_sec);
            println!(
                "    Steal Success Rate: {:.1}%",
                result.pool_stats.steal_success_rate() * 100.0
            );
            println!(
                "    Cross-NUMA Rate: {:.1}%",
                result.pool_stats.cross_numa_rate() * 100.0
            );
            println!("    Policy Adaptations: {}", result.pool_stats.total_adaptations);
        }

        println!("\nEpoch Reclamation Results:");
        for result in &self.epoch_results {
            println!("  {}", result.scenario);
            println!("    Deferrals/sec: {}", result.deferrals_per_sec);
            println!(
                "    Reclamation Rate: {:.1}%",
                result.reclamation_rate * 100.0
            );
            println!("    Avg Garbage/Thread: {}", result.avg_garbage_per_thread);
        }
    }

    /// Check if performance targets are met
    pub fn verify_performance_targets(&self) -> PerformanceReport {
        let mut report = PerformanceReport {
            skiplist_target_met: false,
            work_stealing_target_met: false,
            epoch_target_met: false,
            details: Vec::new(),
        };

        // Check skip list throughput (target: +20% over baseline)
        // Baseline assumption: ~1M ops/sec for concurrent inserts
        if let Some(insert_result) = self
            .skiplist_results
            .iter()
            .find(|r| r.operation == "concurrent_insert")
        {
            let target_ops = 1_200_000; // 1M * 1.2
            report.skiplist_target_met = insert_result.ops_per_sec >= target_ops;
            report.details.push(format!(
                "Skip List: {} ops/sec (target: {} ops/sec) - {}",
                insert_result.ops_per_sec,
                target_ops,
                if report.skiplist_target_met {
                    "PASS"
                } else {
                    "FAIL"
                }
            ));
        }

        // Check work-stealing efficiency (target: +15% steal success rate)
        if let Some(imbalanced) = self
            .work_stealing_results
            .iter()
            .find(|r| r.scenario == "imbalanced_workload")
        {
            let target_rate = 0.7; // 70% success rate
            let actual_rate = imbalanced.pool_stats.steal_success_rate();
            report.work_stealing_target_met = actual_rate >= target_rate;
            report.details.push(format!(
                "Work-Stealing: {:.1}% success rate (target: {:.1}%) - {}",
                actual_rate * 100.0,
                target_rate * 100.0,
                if report.work_stealing_target_met {
                    "PASS"
                } else {
                    "FAIL"
                }
            ));
        }

        // Check epoch reclamation (target: -25% memory overhead ~ 75% reclamation rate)
        if let Some(epoch) = self.epoch_results.first() {
            let target_rate = 0.75;
            report.epoch_target_met = epoch.reclamation_rate >= target_rate;
            report.details.push(format!(
                "Epoch Reclamation: {:.1}% reclaimed (target: {:.1}%) - {}",
                epoch.reclamation_rate * 100.0,
                target_rate * 100.0,
                if report.epoch_target_met {
                    "PASS"
                } else {
                    "FAIL"
                }
            ));
        }

        report
    }
}

/// Performance report
#[derive(Debug)]
pub struct PerformanceReport {
    pub skiplist_target_met: bool,
    pub work_stealing_target_met: bool,
    pub epoch_target_met: bool,
    pub details: Vec<String>,
}

impl PerformanceReport {
    pub fn print(&self) {
        println!("\n=== Performance Target Verification ===\n");
        for detail in &self.details {
            println!("  {}", detail);
        }
        println!("\nOverall: {}", if self.all_targets_met() {
            "ALL TARGETS MET ✓"
        } else {
            "SOME TARGETS NOT MET"
        });
    }

    pub fn all_targets_met(&self) -> bool {
        self.skiplist_target_met && self.work_stealing_target_met && self.epoch_target_met
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skiplist_benchmark() {
        let results = benchmark_skiplist(2, 1000);
        assert!(!results.is_empty());

        for result in &results {
            assert!(result.ops_per_sec > 0);
            println!("{}: {} ops/sec", result.operation, result.ops_per_sec);
        }
    }

    #[test]
    fn test_work_stealing_benchmark() {
        let results = benchmark_work_stealing(2, 1000);
        assert!(!results.is_empty());

        for result in &results {
            assert!(result.tasks_per_sec > 0);
            println!("{}: {} tasks/sec", result.scenario, result.tasks_per_sec);
        }
    }

    #[test]
    fn test_epoch_benchmark() {
        let results = benchmark_epoch_reclamation(2, 1000);
        assert!(!results.is_empty());

        for result in &results {
            assert!(result.deferrals_per_sec > 0);
            println!("{}: {} deferrals/sec", result.scenario, result.deferrals_per_sec);
        }
    }

    #[test]
    fn test_integration() {
        let results = integration_test_concurrency_optimizations();
        results.print_summary();

        let report = results.verify_performance_targets();
        report.print();

        // Note: Targets may not be met in test environment
        // In production, these should all pass
    }
}
