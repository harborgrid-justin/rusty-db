// Benchmarks for Index/SIMD Optimizations (I001, I002, I003)
//
// Comprehensive performance testing for:
// - B-Tree split optimization
// - SIMD vectorized filtering
// - Bitmap index compression

#![feature(test)]
extern crate test;

use rusty_db::index::btree_optimized::*;
use rusty_db::index::bitmap_compressed::*;
use rusty_db::simd::advanced_ops::*;
use rusty_db::simd::SelectionVector;
use test::Bencher;

// ============================================================================
// I001: B-Tree Split Optimization Benchmarks
// ============================================================================

#[bench]
fn bench_split_anticipation(b: &mut Bencher) {
    let mut predictor = SplitPredictor::new();

    b.iter(|| {
        for i in 0..1000 {
            predictor.record_insert(i, 50, 64);
        }
        predictor.reset();
    });
}

#[bench]
fn bench_prefix_compression(b: &mut Bencher) {
    let strings: Vec<String> = (0..1000)
        .map(|i| format!("user_account_{:08}", i))
        .collect();

    b.iter(|| {
        let _prefix = PrefixAnalyzer::find_common_prefix(&strings);
    });
}

#[bench]
fn bench_prefix_compress_decompress(b: &mut Bencher) {
    let strings: Vec<String> = (0..100)
        .map(|i| format!("user_account_{:08}", i))
        .collect();

    b.iter(|| {
        let (prefix, compressed) = PrefixAnalyzer::compress(strings.clone());
        // Decompress
        for comp in &compressed {
            test::black_box(comp.to_string());
        }
        test::black_box(prefix);
    });
}

#[bench]
fn bench_suffix_truncation(b: &mut Bencher) {
    let keys = vec![
        "user_account_12345678",
        "user_account_12345679",
        "user_account_12345680",
    ];

    b.iter(|| {
        for i in 1..keys.len() - 1 {
            let truncated = SuffixTruncator::truncate_string(
                keys[i],
                Some(keys[i - 1]),
                Some(keys[i + 1]),
            );
            test::black_box(truncated);
        }
    });
}

#[bench]
fn bench_bulk_loader_calculation(b: &mut Bencher) {
    let loader: BulkLoader<i32, String> = BulkLoader::new(64, 0.9);

    b.iter(|| {
        for size in [100, 1000, 10000, 100000] {
            let nodes = loader.nodes_needed(size);
            test::black_box(nodes);
        }
    });
}

// ============================================================================
// I002: SIMD Vectorized Filtering Benchmarks
// ============================================================================

#[bench]
fn bench_simd_string_compare(b: &mut Bencher) {
    let left: Vec<String> = (0..1000).map(|i| format!("string_{}", i)).collect();
    let right: Vec<String> = (0..1000).map(|i| format!("string_{}", i)).collect();

    b.iter(|| {
        let mut selection = SelectionVector::with_capacity(1000);
        SimdStringCompare::compare_equal(&left, &right, &mut selection).unwrap();
        test::black_box(selection);
    });
}

#[bench]
fn bench_simd_hash_i64_batch(b: &mut Bencher) {
    let keys: Vec<i64> = (0..10000).collect();
    let mut hashes = vec![0u64; 10000];

    b.iter(|| {
        SimdHashJoin::hash_i64_batch(&keys, &mut hashes);
        test::black_box(&hashes);
    });
}

#[bench]
fn bench_simd_aggregate_selected(b: &mut Bencher) {
    let data: Vec<i64> = (0..10000).collect();
    let mut selection = SelectionVector::with_capacity(1000);

    // Select every 10th element
    for i in (0..10000).step_by(10) {
        selection.add(i).unwrap();
    }

    b.iter(|| {
        let sum = SimdAggregateWithSelection::sum_i64_selected(&data, &selection);
        test::black_box(sum);
    });
}

#[bench]
fn bench_bitpacked_selection_vector(b: &mut Bencher) {
    let mut bitmap = BitpackedSelectionVector::new(10000);

    b.iter(|| {
        bitmap.clear();
        for i in (0..10000).step_by(100) {
            bitmap.set(i).unwrap();
        }
        let count = bitmap.count();
        test::black_box(count);
    });
}

#[bench]
fn bench_selection_vector_to_bitmap_conversion(b: &mut Bencher) {
    let mut selection = SelectionVector::with_capacity(1000);
    for i in (0..10000).step_by(10) {
        selection.add(i).unwrap();
    }

    b.iter(|| {
        let bitmap = SelectionVectorConverter::to_bitmap(&selection, 10000);
        test::black_box(bitmap);
    });
}

// ============================================================================
// I003: Bitmap Index Compression Benchmarks
// ============================================================================

#[bench]
fn bench_wah_compression(b: &mut Bencher) {
    // Create sparse bitmap (10% density)
    let mut bitmap = vec![0u64; 1000];
    for i in (0..1000).step_by(10) {
        bitmap[i] = 0xFFFFFFFFFFFFFFFF;
    }

    b.iter(|| {
        let wah = WahBitmap::from_bitmap(&bitmap);
        test::black_box(wah);
    });
}

#[bench]
fn bench_wah_decompression(b: &mut Bencher) {
    let mut bitmap = vec![0u64; 1000];
    for i in (0..1000).step_by(10) {
        bitmap[i] = 0xFFFFFFFFFFFFFFFF;
    }
    let wah = WahBitmap::from_bitmap(&bitmap);

    b.iter(|| {
        let decompressed = wah.to_bitmap();
        test::black_box(decompressed);
    });
}

#[bench]
fn bench_wah_and_operation(b: &mut Bencher) {
    let mut bitmap1 = vec![0u64; 1000];
    let mut bitmap2 = vec![0u64; 1000];

    for i in (0..1000).step_by(10) {
        bitmap1[i] = 0xFFFFFFFFFFFFFFFF;
    }
    for i in (5..1000).step_by(10) {
        bitmap2[i] = 0xFFFFFFFFFFFFFFFF;
    }

    let wah1 = WahBitmap::from_bitmap(&bitmap1);
    let wah2 = WahBitmap::from_bitmap(&bitmap2);

    b.iter(|| {
        let result = wah1.and(&wah2);
        test::black_box(result);
    });
}

#[bench]
fn bench_roaring_bitmap_add(b: &mut Bencher) {
    b.iter(|| {
        let mut roaring = RoaringBitmap::new();
        for i in 0..10000 {
            roaring.add(i);
        }
        test::black_box(roaring);
    });
}

#[bench]
fn bench_roaring_bitmap_contains(b: &mut Bencher) {
    let mut roaring = RoaringBitmap::new();
    for i in 0..10000 {
        roaring.add(i);
    }

    b.iter(|| {
        let mut count = 0;
        for i in 0..10000 {
            if roaring.contains(i) {
                count += 1;
            }
        }
        test::black_box(count);
    });
}

#[bench]
fn bench_roaring_bitmap_and(b: &mut Bencher) {
    let mut r1 = RoaringBitmap::new();
    let mut r2 = RoaringBitmap::new();

    for i in 0..10000 {
        r1.add(i);
    }
    for i in 5000..15000 {
        r2.add(i);
    }

    b.iter(|| {
        let result = r1.and(&r2);
        test::black_box(result);
    });
}

#[bench]
fn bench_roaring_bitmap_or(b: &mut Bencher) {
    let mut r1 = RoaringBitmap::new();
    let mut r2 = RoaringBitmap::new();

    for i in 0..10000 {
        r1.add(i);
    }
    for i in 5000..15000 {
        r2.add(i);
    }

    b.iter(|| {
        let result = r1.or(&r2);
        test::black_box(result);
    });
}

#[bench]
fn bench_simd_bitmap_and(b: &mut Bencher) {
    let bitmap1 = vec![0xAAAAAAAAAAAAAAAAu64; 1000];
    let bitmap2 = vec![0x5555555555555555u64; 1000];
    let mut result = vec![0u64; 1000];

    b.iter(|| {
        #[cfg(target_arch = "x86_64")]
        {
            SimdBitmapOps::and_avx2(&bitmap1, &bitmap2, &mut result);
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            SimdBitmapOps::and_scalar(&bitmap1, &bitmap2, &mut result);
        }
        test::black_box(&result);
    });
}

#[bench]
fn bench_simd_bitmap_or(b: &mut Bencher) {
    let bitmap1 = vec![0xAAAAAAAAAAAAAAAAu64; 1000];
    let bitmap2 = vec![0x5555555555555555u64; 1000];
    let mut result = vec![0u64; 1000];

    b.iter(|| {
        #[cfg(target_arch = "x86_64")]
        {
            SimdBitmapOps::or_avx2(&bitmap1, &bitmap2, &mut result);
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            SimdBitmapOps::or_scalar(&bitmap1, &bitmap2, &mut result);
        }
        test::black_box(&result);
    });
}

// ============================================================================
// Combined Benchmark: Real-world Scenario
// ============================================================================

#[bench]
fn bench_combined_index_query(b: &mut Bencher) {
    // Setup: Create roaring bitmap index
    let mut index = RoaringBitmap::new();
    for i in 0..100000 {
        if i % 10 == 0 {
            index.add(i);
        }
    }

    // Query: Find matching rows and aggregate
    b.iter(|| {
        let matches = index.to_vec();
        let mut selection = SelectionVector::with_capacity(matches.len());

        for m in matches {
            selection.add(m as usize).unwrap();
        }

        let count = SimdAggregateWithSelection::count_selected(&selection);
        test::black_box(count);
    });
}
