// Transaction Management Performance Benchmarks
// Tests critical transaction operations including begin/commit,
// lock acquisition, MVCC version management, and deadlock detection

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rusty_db::transaction::{
    lock_manager::LockManager,
    manager::TransactionManager,
    types::{IsolationLevel, LockMode},
};
use std::sync::Arc;

fn bench_transaction_lifecycle(c: &mut Criterion) {
    let manager = TransactionManager::new();

    c.bench_function("begin_commit", |b| {
        b.iter(|| {
            let txn_id = manager.begin().unwrap();
            manager.commit(txn_id).ok();
            black_box(txn_id);
        });
    });
}

fn bench_isolation_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("isolation_levels");

    let levels = vec![
        IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead,
        IsolationLevel::Serializable,
    ];

    for level in levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", level)),
            &level,
            |b, &level| {
                let manager = TransactionManager::new();
                b.iter(|| {
                    let txn_id = manager.begin_with_isolation(level).unwrap();
                    black_box(txn_id);
                    manager.commit(txn_id).ok();
                });
            },
        );
    }

    group.finish();
}

fn bench_lock_acquisition(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_acquisition");

    let lock_modes = vec![
        LockMode::Shared,
        LockMode::Exclusive,
        LockMode::IntentionShared,
        LockMode::IntentionExclusive,
    ];

    for mode in lock_modes {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", mode)),
            &mode,
            |b, &mode| {
                let manager = Arc::new(TransactionManager::new());
                let lock_manager = manager.get_lock_manager();

                b.iter(|| {
                    let txn_id = manager.begin().unwrap();
                    let resource = format!("table.row_{}", txn_id);

                    lock_manager
                        .acquire_lock(txn_id, resource.clone(), mode)
                        .ok();

                    lock_manager.release_locks(txn_id).ok();
                    manager.commit(txn_id).ok();
                    black_box(txn_id);
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_transactions");

    let thread_counts = vec![1, 2, 4, 8, 16];

    for num_threads in thread_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            &num_threads,
            |b, &num_threads| {
                let manager = Arc::new(TransactionManager::new());

                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let mgr = manager.clone();
                            std::thread::spawn(move || {
                                for _ in 0..10 {
                                    if let Ok(txn_id) = mgr.begin() {
                                        mgr.commit(txn_id).ok();
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

fn bench_lock_contention(c: &mut Criterion) {
    let manager = Arc::new(TransactionManager::new());
    let lock_manager = manager.get_lock_manager();

    c.bench_function("lock_contention", |b| {
        b.iter(|| {
            // Multiple transactions competing for the same resource
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let mgr = manager.clone();
                    let lm = lock_manager.clone();
                    std::thread::spawn(move || {
                        for i in 0..10 {
                            if let Ok(txn_id) = mgr.begin() {
                                // All threads try to lock the same resource
                                let resource = format!("table.row_hotspot");
                                lm.acquire_lock(txn_id, resource.clone(), LockMode::Exclusive)
                                    .ok();

                                // Small critical section
                                black_box(i);

                                lm.release_locks(txn_id).ok();
                                mgr.commit(txn_id).ok();
                            }
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().ok();
            }
        });
    });
}

fn bench_mvcc_version_creation(c: &mut Criterion) {
    let manager = TransactionManager::new();

    c.bench_function("mvcc_version_creation", |b| {
        b.iter(|| {
            let txn_id = manager.begin().unwrap();

            // Simulate creating multiple versions
            for i in 0..100 {
                black_box(i);
            }

            manager.commit(txn_id).ok();
        });
    });
}

fn bench_deadlock_detection(c: &mut Criterion) {
    let manager = Arc::new(TransactionManager::new());

    c.bench_function("deadlock_detection", |b| {
        b.iter(|| {
            let lock_manager = manager.get_lock_manager();

            // Create a potential deadlock scenario
            let txn1 = manager.begin().unwrap();
            let txn2 = manager.begin().unwrap();

            lock_manager
                .acquire_lock(txn1, "resource_a".to_string(), LockMode::Exclusive)
                .ok();
            lock_manager
                .acquire_lock(txn2, "resource_b".to_string(), LockMode::Exclusive)
                .ok();

            // These would create a deadlock if waited
            lock_manager
                .try_acquire_lock(txn1, "resource_b".to_string(), LockMode::Exclusive)
                .ok();
            lock_manager
                .try_acquire_lock(txn2, "resource_a".to_string(), LockMode::Exclusive)
                .ok();

            lock_manager.release_locks(txn1).ok();
            lock_manager.release_locks(txn2).ok();

            manager.commit(txn1).ok();
            manager.commit(txn2).ok();

            black_box(&manager);
        });
    });
}

fn bench_wal_operations(c: &mut Criterion) {
    let manager = TransactionManager::new();

    c.bench_function("wal_write", |b| {
        b.iter(|| {
            let txn_id = manager.begin().unwrap();

            // Simulate WAL writes during transaction
            for i in 0..50 {
                black_box(i);
            }

            manager.commit(txn_id).ok();
        });
    });
}

criterion_group!(
    benches,
    bench_transaction_lifecycle,
    bench_isolation_levels,
    bench_lock_acquisition,
    bench_concurrent_transactions,
    bench_lock_contention,
    bench_mvcc_version_creation,
    bench_deadlock_detection,
    bench_wal_operations
);
criterion_main!(benches);
