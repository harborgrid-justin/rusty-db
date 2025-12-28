// Buffer Pool Performance Benchmarks
// Tests critical buffer pool operations including page pin/unpin,
// eviction policies, and concurrent access patterns

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rusty_db::buffer::{
    eviction::EvictionPolicyType,
    manager::{BufferPoolConfig, BufferPoolManager},
};
use rusty_db::storage::disk::DiskManager;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

fn create_buffer_pool(policy: EvictionPolicyType, num_frames: usize) -> (BufferPoolManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let disk_manager = Arc::new(DiskManager::new(&db_path).unwrap());

    let config = BufferPoolConfig {
        num_frames,
        eviction_policy: policy,
        page_table_partitions: 16,
        enable_per_core_pools: true,
        frames_per_core: num_frames / num_cpus::get(),
        max_flush_batch_size: 64,
        enable_background_flush: false,
        background_flush_interval: Duration::from_secs(1),
        dirty_page_threshold: 0.7,
        enable_stats: true,
        enable_prefetch: false,
        prefetch_threads: 2,
        prefetch_distance: 4,
    };

    let manager = BufferPoolManager::new(disk_manager, config);
    (manager, temp_dir)
}

fn bench_page_pin_unpin(c: &mut Criterion) {
    let mut group = c.benchmark_group("page_pin_unpin");

    let policies = vec![
        EvictionPolicyType::Clock,
        EvictionPolicyType::LRU,
        EvictionPolicyType::TwoQueue,
    ];

    for policy in policies {
        group.bench_with_input(
            BenchmarkId::new("pin_unpin", format!("{:?}", policy)),
            &policy,
            |b, &policy| {
                let (manager, _temp) = create_buffer_pool(policy, 1000);
                b.iter(|| {
                    // Pin and unpin a page
                    if let Ok(mut frame) = manager.pin_page(black_box(1)) {
                        black_box(&mut *frame);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_eviction_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("eviction_policies");

    let policies = vec![
        EvictionPolicyType::Clock,
        EvictionPolicyType::LRU,
        EvictionPolicyType::TwoQueue,
        EvictionPolicyType::LRUK { k: 2 },
        EvictionPolicyType::LIRS,
        EvictionPolicyType::ARC,
    ];

    for policy in policies {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", policy)),
            &policy,
            |b, &policy| {
                let (manager, _temp) = create_buffer_pool(policy, 100);

                b.iter(|| {
                    // Access pattern that triggers eviction
                    for page_id in 0..150 {
                        if let Ok(mut frame) = manager.pin_page(black_box(page_id)) {
                            black_box(&mut *frame);
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    let thread_counts = vec![1, 2, 4, 8];

    for num_threads in thread_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            &num_threads,
            |b, &num_threads| {
                let (manager, _temp) = create_buffer_pool(EvictionPolicyType::Clock, 1000);
                let manager = Arc::new(manager);

                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|i| {
                            let mgr = manager.clone();
                            std::thread::spawn(move || {
                                for j in 0..10 {
                                    let page_id = (i * 10 + j) as u32;
                                    if let Ok(mut frame) = mgr.pin_page(page_id) {
                                        black_box(&mut *frame);
                                    }
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().ok();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_batch_flush(c: &mut Criterion) {
    let (manager, _temp) = create_buffer_pool(EvictionPolicyType::Clock, 1000);

    c.bench_function("batch_flush", |b| {
        b.iter(|| {
            // Pin and mark pages as dirty
            for page_id in 0..64 {
                if let Ok(mut frame) = manager.pin_page(page_id) {
                    frame.mark_dirty();
                    black_box(&mut *frame);
                }
            }

            // Flush would happen here
            black_box(&manager);
        });
    });
}

fn bench_page_table_lookup(c: &mut Criterion) {
    let (manager, _temp) = create_buffer_pool(EvictionPolicyType::Clock, 10000);

    // Pre-populate the buffer pool
    for page_id in 0..5000 {
        if let Ok(frame) = manager.pin_page(page_id) {
            drop(frame);
        }
    }

    c.bench_function("page_table_lookup", |b| {
        b.iter(|| {
            // Random lookups
            for page_id in (0..5000).step_by(100) {
                if let Ok(frame) = manager.pin_page(black_box(page_id)) {
                    black_box(&*frame);
                }
            }
        });
    });
}

criterion_group!(
    benches,
    bench_page_pin_unpin,
    bench_eviction_policies,
    bench_concurrent_access,
    bench_batch_flush,
    bench_page_table_lookup
);
criterion_main!(benches);
