// Example: Using the High-Performance Buffer Pool Manager
//
// This example demonstrates how to use RustyDB's buffer pool manager
// optimized for Windows/MSVC with zero-allocation hot paths.

use rusty_db::buffer::{
    create_default_buffer_pool, create_olap_buffer_pool, create_oltp_buffer_pool,
    BufferPoolBuilder, EvictionPolicyType,
};
use rusty_db::Result;
use std::time::Duration;

fn main() -> Result<()> {
    println!("=== RustyDB Buffer Pool Manager Examples ===\n");

    // Example 1: Simple buffer pool with defaults
    example_1_simple()?;

    // Example 2: OLTP-optimized buffer pool
    example_2_oltp()?;

    // Example 3: OLAP-optimized buffer pool
    example_3_olap()?;

    // Example 4: Custom configuration
    example_4_custom()?;

    // Example 5: Statistics and monitoring
    example_5_stats()?;

    // Example 6: Multiple page operations
    example_6_multi_page()?;

    Ok(())
}

// Example 1: Simple buffer pool with default settings
fn example_1_simple() -> Result<()> {
    println!("Example 1: Simple Buffer Pool");
    println!("------------------------------");

    // Create a simple buffer pool with 100 frames
    let pool = create_default_buffer_pool(100);

    println!(
        "Created buffer pool with {} frames",
        pool.config().num_frames
    );
    println!("Eviction policy: {}\n", pool.eviction_policy_name());

    // Pin a page
    let page_id = 42;
    let guard = pool.pin_page(page_id)?;

    // Read page data
    let data = guard.read_data();
    println!("Pinned page {}, size: {} bytes", page_id, data.data().len());

    // Page is automatically unpinned when guard is dropped
    drop(data);
    drop(guard);

    println!("Page {} unpinned\n", page_id);
    Ok(())
}

// Example 2: OLTP-optimized buffer pool
fn example_2_oltp() -> Result<()> {
    println!("Example 2: OLTP-Optimized Buffer Pool");
    println!("--------------------------------------");

    // Create OLTP-optimized pool (CLOCK + per-core pools)
    let pool = create_oltp_buffer_pool(1000);

    println!("OLTP pool configuration:");
    println!("  Frames: {}", pool.config().num_frames);
    println!("  Policy: {}", pool.eviction_policy_name());
    println!("  Per-core pools: {}", pool.config().enable_per_core_pools);
    println!(
        "  Background flush: {}\n",
        pool.config().enable_background_flush
    );

    // Simulate OLTP workload
    for i in 0..10 {
        let guard = pool.pin_page(i)?;
        // Access data
        let _data = guard.read_data();
    }

    let stats = pool.stats();
    println!("After OLTP workload:");
    println!("  Hit rate: {:.2}%", stats.hit_rate * 100.0);
    println!("  Page reads: {}", stats.page_reads);
    println!();

    Ok(())
}

// Example 3: OLAP-optimized buffer pool
fn example_3_olap() -> Result<()> {
    println!("Example 3: OLAP-Optimized Buffer Pool");
    println!("--------------------------------------");

    // Create OLAP-optimized pool (2Q for scan resistance)
    let pool = create_olap_buffer_pool(5000);

    println!("OLAP pool configuration:");
    println!("  Frames: {}", pool.config().num_frames);
    println!("  Policy: {} (scan-resistant)", pool.eviction_policy_name());
    println!("  Max batch size: {}\n", pool.config().max_flush_batch_size);

    // Simulate sequential scan
    println!("Simulating sequential scan...");
    for i in 0..100 {
        let guard = pool.pin_page(i)?;
        let _data = guard.read_data();
    }

    let stats = pool.stats();
    println!("After sequential scan:");
    println!("  Pages scanned: 100");
    println!("  Page reads: {}", stats.page_reads);
    println!();

    Ok(())
}

// Example 4: Custom configuration
fn example_4_custom() -> Result<()> {
    println!("Example 4: Custom Configuration");
    println!("--------------------------------");

    // Build a custom buffer pool
    let pool = BufferPoolBuilder::new()
        .num_frames(2000)
        .eviction_policy(EvictionPolicyType::LruK(2))
        .per_core_pools(true)
        .frames_per_core(16)
        .max_flush_batch_size(64)
        .background_flush(true)
        .flush_interval(Duration::from_secs(60))
        .dirty_threshold(0.8)
        .build();

    println!("Custom pool created:");
    println!("  Frames: {}", pool.config().num_frames);
    println!("  Policy: {}", pool.eviction_policy_name());
    println!("  Frames per core: {}", pool.config().frames_per_core);
    println!(
        "  Flush threshold: {}%",
        pool.config().dirty_page_threshold * 100.0
    );
    println!();

    Ok(())
}

// Example 5: Statistics and monitoring
fn example_5_stats() -> Result<()> {
    println!("Example 5: Statistics and Monitoring");
    println!("-------------------------------------");

    let pool = create_default_buffer_pool(100);

    // Pin and modify some pages
    for i in 0..20 {
        let guard = pool.pin_page(i)?;
        if i % 2 == 0 {
            // Modify every other page
            let mut data = guard.write_data();
            data.data_mut()[0] = 42;
        }
    }

    // Get comprehensive statistics
    let stats = pool.stats();

    println!("Buffer Pool Statistics:");
    println!("  Total frames: {}", stats.total_frames);
    println!("  Free frames: {}", stats.free_frames);
    println!("  Pinned frames: {}", stats.pinned_frames);
    println!("  Dirty frames: {}", stats.dirty_frames);
    println!();
    println!("Page Table:");
    println!("  Lookups: {}", stats.lookups);
    println!("  Hits: {}", stats.hits);
    println!("  Misses: {}", stats.misses);
    println!("  Hit rate: {:.2}%", stats.hit_rate * 100.0);
    println!();
    println!("I/O Operations:");
    println!("  Page reads: {}", stats.page_reads);
    println!("  Page writes: {}", stats.page_writes);
    println!("  Evictions: {}", stats.evictions);
    println!();
    println!("Performance:");
    println!("  Avg search length: {:.2}", stats.avg_search_length);
    println!("  I/O wait time: {}µs", stats.io_wait_time_us);
    println!();

    Ok(())
}

// Example 6: Multiple page operations
fn example_6_multi_page() -> Result<()> {
    println!("Example 6: Multiple Page Operations");
    println!("------------------------------------");

    let pool = create_default_buffer_pool(50);

    // Pin multiple pages at once
    println!("Pinning 10 pages...");
    let mut guards = Vec::new();
    for i in 0..10 {
        guards.push(pool.pin_page(i)?);
    }

    println!("All pages pinned successfully");

    let stats = pool.stats();
    println!("  Pinned frames: {}", stats.pinned_frames);

    // Modify some pages
    println!("\nModifying pages 0, 2, 4, 6, 8...");
    for (i, guard) in guards.iter().enumerate() {
        if i % 2 == 0 {
            let mut data = guard.write_data();
            data.data_mut()[0] = (i * 10) as u8;
        }
    }

    let stats = pool.stats();
    println!("  Dirty frames: {}", stats.dirty_frames);

    // Flush dirty pages
    println!("\nFlushing dirty pages...");
    drop(guards); // Unpin all pages first
    pool.flush_all()?;

    let stats = pool.stats();
    println!("  Page writes: {}", stats.page_writes);
    println!("  Dirty frames after flush: {}", stats.dirty_frames);
    println!();

    Ok(())
}
