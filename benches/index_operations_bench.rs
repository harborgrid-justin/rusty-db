// Index Operations Performance Benchmarks
// Tests critical index operations including insertions, lookups,
// range scans, and different index types

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rusty_db::index::{
    btree::{BTree, BTreeConfig},
    hash_index::HashIndex,
    IndexManager, IndexType,
};

fn bench_btree_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_insert");

    let sizes = vec![100, 1000, 10000];

    for size in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let config = BTreeConfig::default();
                let mut btree: BTree<i64, String> = BTree::new(config);

                for i in 0..size {
                    btree.insert(black_box(i), format!("value_{}", i)).ok();
                }

                black_box(btree);
            });
        });
    }

    group.finish();
}

fn bench_btree_lookup(c: &mut Criterion) {
    let config = BTreeConfig::default();
    let mut btree: BTree<i64, String> = BTree::new(config);

    // Pre-populate the tree
    for i in 0..10000 {
        btree.insert(i, format!("value_{}", i)).ok();
    }

    c.bench_function("btree_lookup", |b| {
        b.iter(|| {
            for i in (0..10000).step_by(100) {
                black_box(btree.get(&i));
            }
        });
    });
}

fn bench_btree_range_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_range_scan");

    let config = BTreeConfig::default();
    let mut btree: BTree<i64, String> = BTree::new(config);

    // Pre-populate the tree
    for i in 0..10000 {
        btree.insert(i, format!("value_{}", i)).ok();
    }

    let range_sizes = vec![10, 100, 1000];

    for range_size in range_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(range_size),
            &range_size,
            |b, &range_size| {
                b.iter(|| {
                    let start = 5000;
                    let end = start + range_size;
                    black_box(btree.range_scan(&start, &end));
                });
            },
        );
    }

    group.finish();
}

fn bench_hash_index_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_index");

    group.bench_function("insert", |b| {
        b.iter(|| {
            let mut hash_index = HashIndex::new(1000);

            for i in 0..1000 {
                hash_index
                    .insert(black_box(i), vec![i as u32])
                    .ok();
            }

            black_box(hash_index);
        });
    });

    group.bench_function("lookup", |b| {
        let mut hash_index = HashIndex::new(1000);

        // Pre-populate
        for i in 0..1000 {
            hash_index.insert(i, vec![i as u32]).ok();
        }

        b.iter(|| {
            for i in (0..1000).step_by(10) {
                black_box(hash_index.get(&i));
            }
        });
    });

    group.finish();
}

fn bench_index_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_manager");

    group.bench_function("create_index", |b| {
        b.iter(|| {
            let manager = IndexManager::new();

            for i in 0..10 {
                manager
                    .create_index(
                        format!("table_{}", i),
                        format!("idx_{}", i),
                        vec![format!("col_{}", i)],
                        IndexType::BTree,
                    )
                    .ok();
            }

            black_box(manager);
        });
    });

    group.bench_function("get_index", |b| {
        let manager = IndexManager::new();

        // Pre-create indexes
        for i in 0..10 {
            manager
                .create_index(
                    format!("table_{}", i),
                    format!("idx_{}", i),
                    vec![format!("col_{}", i)],
                    IndexType::BTree,
                )
                .ok();
        }

        b.iter(|| {
            for i in 0..10 {
                black_box(manager.get_index(&format!("table_{}", i), &format!("idx_{}", i)));
            }
        });
    });

    group.finish();
}

fn bench_concurrent_index_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_index_access");

    let thread_counts = vec![1, 2, 4, 8];

    for num_threads in thread_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            &num_threads,
            |b, &num_threads| {
                let config = BTreeConfig::default();
                let btree: BTree<i64, String> = BTree::new(config);

                // Pre-populate
                for i in 0..10000 {
                    btree.insert(i, format!("value_{}", i)).ok();
                }

                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|thread_id| {
                            std::thread::spawn(move || {
                                for i in 0..100 {
                                    let key = (thread_id * 100 + i) as i64;
                                    black_box(key);
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

fn bench_index_update_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_update_delete");

    group.bench_function("delete", |b| {
        b.iter(|| {
            let config = BTreeConfig::default();
            let mut btree: BTree<i64, String> = BTree::new(config);

            // Insert
            for i in 0..1000 {
                btree.insert(i, format!("value_{}", i)).ok();
            }

            // Delete
            for i in (0..1000).step_by(2) {
                btree.delete(&i).ok();
            }

            black_box(btree);
        });
    });

    group.bench_function("update", |b| {
        let config = BTreeConfig::default();
        let mut btree: BTree<i64, String> = BTree::new(config);

        // Insert
        for i in 0..1000 {
            btree.insert(i, format!("value_{}", i)).ok();
        }

        b.iter(|| {
            // Update (delete + insert)
            for i in 0..100 {
                btree.delete(&i).ok();
                btree.insert(i, format!("new_value_{}", i)).ok();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_btree_insert,
    bench_btree_lookup,
    bench_btree_range_scan,
    bench_hash_index_operations,
    bench_index_manager,
    bench_concurrent_index_access,
    bench_index_update_delete
);
criterion_main!(benches);
