// Buffer Pool Management Demo
//
// This example demonstrates the comprehensive buffer pool management system
// including multi-tier pools, caching, replacement policies, and dirty page management.

use rusty_db::memory::buffer_pool::{
    BufferPoolManager,
    BufferPoolConfig,
    PageId,
    PoolType,
};

fn main() {
    println!("=== RustyDB Buffer Pool Management Demo ===\n");

    // Create a buffer pool with custom configuration
    let config = BufferPoolConfig {
        total_size: 256 * 1024 * 1024, // 256MB
        page_size: 8192,
        hot_tier_ratio: 0.2,
        warm_tier_ratio: 0.5,
        numa_aware: false,
        keep_pool_size: 16 * 1024 * 1024,
        recycle_pool_size: 8 * 1024 * 1024,
        promotion_threshold: 10,
        demotion_threshold_secs: 300,
        ..Default::default()
    };

    println!("1. Creating Buffer Pool Manager");
    println!("   - Total size: {}MB", config.total_size / 1024 / 1024);
    println!("   - Page size: {} bytes", config.page_size);
    println!("   - Hot tier: {}%", (config.hot_tier_ratio * 100.0) as u32);
    println!("   - Warm tier: {}%", (config.warm_tier_ratio * 100.0) as u32);
    println!("");

    let manager = BufferPoolManager::new(config);

    // Start background operations
    println!("2. Starting Background Operations");
    manager.api_start_background_operations();
    println!("   - Tier manager started");
    println!("   - Incremental checkpointer started");
    println!("   - Background writer started");
    println!("");

    // Pin some pages
    println!("3. Pinning Pages");
    for i in 0..10 {
        if let Some(frame) = manager.api_pin_page(0, i) {
            println!("   - Pinned page {} (tablespace 0)", i);
        }
    }
    println!("");

    // Get statistics
    println!("4. Buffer Pool Statistics");
    let stats = manager.api_get_stats();
    println!("   {}", serde_json::to_string_pretty(&stats).unwrap());
    println!("");

    // Get capacity and usage
    println!("5. Capacity and Usage");
    println!("   - Total capacity: {} bytes", manager.api_get_capacity());
    println!("   - Frames in use: {}", manager.api_get_frames_in_use());
    println!("");

    // Check memory pressure
    println!("6. Memory Pressure");
    let pressure = manager.api_get_memory_pressure();
    println!("   - Current usage: {} bytes", pressure.current_usage);
    println!("   - Pressure level: {:.2}%", pressure.pressure_level * 100.0);
    println!("   - Under pressure: {}", pressure.under_pressure);
    println!("");

    // Export Prometheus metrics
    println!("7. Prometheus Metrics Export");
    let prometheus = manager.api_export_prometheus();
    println!("{}", prometheus.lines().take(10).collect::<Vec<_>>().join("\n"));
    println!("   ... (truncated)");
    println!("");

    // Perform checkpoint
    println!("8. Performing Checkpoint");
    let checkpoint_result = manager.api_checkpoint();
    println!("   - Pages flushed: {}", checkpoint_result.pages_flushed);
    println!("   - Checkpoint LSN: {}", checkpoint_result.checkpoint_lsn);
    println!("");

    // Unpin pages
    println!("9. Unpinning Pages");
    for i in 0..10 {
        manager.api_unpin_page(0, i, false);
        println!("   - Unpinned page {} (not dirty)", i);
    }
    println!("");

    // Stop background operations
    println!("10. Stopping Background Operations");
    manager.api_stop_background_operations();
    println!("   - All background threads stopped");
    println!("");

    println!("=== Demo Complete ===");
}
