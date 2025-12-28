# RustyDB v0.6.0 Performance Tuning Guide

**Release**: v0.6.0 - $856M Enterprise Server Release
**Date**: December 2025
**Classification**: Enterprise Tuning Documentation

---

## Table of Contents

1. [Buffer Pool Tuning](#buffer-pool-tuning)
2. [Memory Management Tuning](#memory-management-tuning)
3. [Transaction Layer Tuning](#transaction-layer-tuning)
4. [Query Optimizer Tuning](#query-optimizer-tuning)
5. [Connection Pool Tuning](#connection-pool-tuning)
6. [Concurrency Tuning](#concurrency-tuning)
7. [I/O and Storage Tuning](#io-and-storage-tuning)
8. [Workload-Specific Configurations](#workload-specific-configurations)

---

## Buffer Pool Tuning

### Basic Configuration

```rust
use rusty_db::buffer::BufferPoolConfig;

let config = BufferPoolConfig {
    // Core settings
    num_frames: 10_000,           // Number of pages (default: 1000)
    page_size: 4096,              // Page size in bytes (default: 4KB)

    // Eviction policy
    eviction_policy: EvictionPolicyType::Arc,  // Arc, Lru, Lirs, Clock

    // Concurrency
    shard_count: 64,              // Lock-free page table shards
    ..Default::default()
};
```

### Parameter Guide

#### `num_frames`
**Description**: Number of pages to cache in memory
**Range**: 1,000 - 1,000,000
**Default**: 1,000 (4 MB with 4KB pages)

**Tuning Recommendations**:
```
Small deployments (< 16 GB RAM):
  num_frames = 10_000              // ~40 MB

Medium deployments (16-64 GB RAM):
  num_frames = 100_000             // ~400 MB

Large deployments (64-256 GB RAM):
  num_frames = 1_000_000           // ~4 GB

Enterprise deployments (> 256 GB RAM):
  num_frames = 10_000_000          // ~40 GB
```

**Formula**: `num_frames = (available_memory * 0.25) / page_size`

#### `eviction_policy`
**Description**: Algorithm for page eviction
**Options**: `Arc`, `Lirs`, `Lru`, `Clock`, `TwoQ`, `LruK`

**Workload Recommendations**:
```
OLTP (mixed reads/writes, short transactions):
  eviction_policy = Arc            // Adaptive, self-tuning

OLAP (sequential scans, analytics):
  eviction_policy = Lirs           // Excellent scan resistance

General purpose:
  eviction_policy = Arc            // Best overall performance

Legacy compatibility:
  eviction_policy = Lru            // Simple, predictable
```

**Performance Comparison**:
```
                   Hit Rate  Memory    CPU    Scan Resistance
Arc (default)         85%    1.2x      Low    Good
Lirs                  91%    1.2x      Low    Excellent
Lru                   78%    1.0x      Minimal Fair
Clock                 76%    1.0x      Minimal Fair
TwoQ                  82%    1.5x      Low    Good
```

### Enhanced ARC Configuration

```rust
use rusty_db::enterprise_optimization::arc_enhanced::{
    EnhancedArcEvictionPolicy, EnhancedArcConfig
};

let arc_config = EnhancedArcConfig {
    // Adaptive ghost list sizing
    adaptive_ghost_lists: true,
    min_ghost_ratio: 0.5,          // Min ghost list size (50% of cache)
    max_ghost_ratio: 2.0,          // Max ghost list size (200% of cache)

    // Scan detection and isolation
    scan_detection: true,
    scan_window_size: 32,          // Pattern detection window
    scan_threshold: 0.7,           // 70% sequential = scan

    // PID controller for p parameter
    pid_kp: 0.1,                   // Proportional gain
    pid_ki: 0.01,                  // Integral gain
    pid_kd: 0.05,                  // Derivative gain
};

let arc = EnhancedArcEvictionPolicy::with_config(num_frames, arc_config);
```

### Prefetching Configuration

```rust
use rusty_db::enterprise_optimization::prefetch_enhanced::{
    EnhancedPrefetchEngine, EnhancedPrefetchConfig
};

let prefetch_config = EnhancedPrefetchConfig {
    enabled: true,

    // Adaptive depth control
    initial_depth: 8,              // Starting prefetch depth
    min_depth: 2,                  // Minimum (for HDD)
    max_depth: 32,                 // Maximum (for SSD)
    adaptive_depth: true,

    // I/O latency thresholds
    low_latency_threshold_us: 50,  // SSD threshold (<50μs)
    high_latency_threshold_us: 500, // HDD threshold (>500μs)

    // Throttling
    pressure_threshold: 0.85,      // Throttle at 85% buffer pool usage

    // Pattern detection
    pattern_window_size: 32,       // Detection window
    min_confidence: 0.7,           // Prefetch threshold
};
```

**Tuning for Storage Type**:
```
NVMe SSD (ultra-fast):
  initial_depth = 16
  max_depth = 64
  low_latency_threshold_us = 20

SATA SSD (fast):
  initial_depth = 8
  max_depth = 32
  low_latency_threshold_us = 50

HDD (slow):
  initial_depth = 4
  max_depth = 16
  high_latency_threshold_us = 1000
```

### Dirty Page Flushing

```rust
use rusty_db::enterprise_optimization::dirty_page_flusher::{
    AdvancedDirtyPageFlusher, DirtyPageFlusherConfig
};

let flusher_config = DirtyPageFlusherConfig {
    enabled: true,

    // Flush timing
    flush_interval: Duration::from_secs(5),      // Flush every 5s
    dirty_threshold: 0.7,                        // Flush at 70% dirty

    // Batching
    max_batch_size: 64,                          // 64 pages per batch
    write_combine_distance: 10,                  // Combine within 10 pages

    // Advanced features
    fuzzy_checkpoint: true,                      // Non-blocking checkpoints
    adaptive_rate: true,                         // Adaptive rate control
    target_bandwidth_mbps: 100.0,                // Target 100 MB/s
    priority_flushing: true,                     // Hot pages first
    hot_page_threshold: 5,                       // Hot after 5 modifications

    // Checkpoint interval
    checkpoint_interval: Duration::from_secs(60), // Every 60 seconds
};
```

**Tuning for Workload**:
```
Write-heavy OLTP:
  flush_interval = 2s
  dirty_threshold = 0.6
  max_batch_size = 128
  target_bandwidth_mbps = 200.0

Read-heavy OLTP:
  flush_interval = 10s
  dirty_threshold = 0.8
  max_batch_size = 32
  target_bandwidth_mbps = 50.0

OLAP/Batch loads:
  flush_interval = 30s
  dirty_threshold = 0.9
  max_batch_size = 256
  target_bandwidth_mbps = 500.0
```

---

## Memory Management Tuning

### Slab Allocator Configuration

```rust
use rusty_db::enterprise_optimization::slab_tuner::TunedSlabAllocator;

// Initialize with CPU count
let num_cpus = num_cpus::get();
let allocator = TunedSlabAllocator::new(num_cpus);

// Size classes are pre-configured but can be customized
// Default classes:
// - 64, 128, 192, 256, 384, 512, 1024, 2048, 4096 bytes
```

**Magazine Sizing**:
```
Lock-heavy workload (many small locks):
  lock_entry_magazine_size = 256    // Default: 128

Row-heavy workload (OLTP):
  small_row_magazine_size = 128     // Default: 96

Index-heavy workload (B-tree):
  index_node_magazine_size = 32     // Default: 16
```

### Memory Pressure Configuration

```rust
use rusty_db::enterprise_optimization::pressure_forecaster::{
    PressureForecaster, EarlyWarningConfig
};

let warning_config = EarlyWarningConfig {
    // Thresholds
    warning_threshold: 0.70,         // Warning at 70%
    high_pressure_threshold: 0.80,   // High at 80%
    critical_threshold: 0.90,        // Critical at 90%
    emergency_threshold: 0.95,       // Emergency at 95%

    // Forecasting
    history_size: 60,                // 60 samples for prediction
    forecast_horizons: vec![30, 60, 120], // 30s, 60s, 120s forecasts
    min_confidence: 0.7,             // Minimum confidence threshold
};
```

**Tuning for Environment**:
```
Development/Test (relaxed):
  warning_threshold = 0.80
  critical_threshold = 0.95

Production (moderate):
  warning_threshold = 0.70
  critical_threshold = 0.90

Mission-critical (aggressive):
  warning_threshold = 0.60
  critical_threshold = 0.85
```

### Transaction Arena Configuration

```rust
use rusty_db::enterprise_optimization::transaction_arena::{
    TransactionArenaManager, TransactionSizeProfile
};

let arena_mgr = TransactionArenaManager::new();

// Size profiles (auto-selected based on estimated size)
TransactionSizeProfile::Tiny      // <10KB: 4KB initial, 64KB limit
TransactionSizeProfile::Small     // 10-100KB: 32KB initial, 512KB limit
TransactionSizeProfile::Medium    // 100KB-1MB: 256KB initial, 4MB limit
TransactionSizeProfile::Large     // 1-10MB: 2MB initial, 32MB limit
TransactionSizeProfile::Huge      // >10MB: 16MB initial, 256MB limit
```

**Tuning Tips**:
```
OLTP (small transactions):
  - Most transactions will be Tiny/Small
  - No configuration needed (auto-selected)

OLAP (large transactions):
  - Pre-allocate with size hint:
    arena_mgr.create_arena(txn_id, Some(2_000_000))?;  // 2MB hint
  - Profile will be selected based on hint

Bulk loads:
  - Use Huge profile
  - Consider disabling arena for very large loads (>100MB)
```

### Large Object Allocator

```rust
use rusty_db::enterprise_optimization::large_object_optimizer::{
    LargeObjectOptimizer, AllocationStrategy
};

let optimizer = LargeObjectOptimizer::new(
    Some(2 * 1024 * 1024)  // 2MB threshold for "large"
);

// Allocation strategies
AllocationStrategy::BestFit     // Smallest region that fits (default)
AllocationStrategy::FirstFit    // First region that fits (faster)
AllocationStrategy::WorstFit    // Largest region (anti-fragmentation)
```

**Strategy Selection**:
```
Low fragmentation priority:
  strategy = BestFit

High performance priority:
  strategy = FirstFit

Anti-fragmentation (long-running):
  strategy = WorstFit
```

---

## Transaction Layer Tuning

### MVCC Configuration

```rust
use rusty_db::enterprise_optimization::mvcc_optimized::OptimizedMVCCManager;

let mvcc = OptimizedMVCCManager::new(
    1000  // max_versions_per_key
);
```

**Tuning `max_versions_per_key`**:
```
Short transactions, low concurrency:
  max_versions_per_key = 100

Medium transactions, moderate concurrency:
  max_versions_per_key = 1000  // Default

Long-running transactions, high concurrency:
  max_versions_per_key = 10000

Analytical queries (time-travel):
  max_versions_per_key = 100000
```

**Impact**:
- **Lower values**: Less memory, more frequent compaction
- **Higher values**: More memory, better performance for long transactions

### Lock Manager Configuration

```rust
use rusty_db::enterprise_optimization::lock_manager_sharded::ShardedLockManager;

// Shard count: 64 (default)
// Recommended: 4-8x CPU core count
let num_cores = num_cpus::get();
let shard_count = (num_cores * 4).next_power_of_two();

let lock_manager = ShardedLockManager::new(shard_count);
```

**Shard Count Tuning**:
```
Low concurrency (< 10 concurrent txns):
  shard_count = 16

Medium concurrency (10-50 concurrent txns):
  shard_count = 64  // Default

High concurrency (50-200 concurrent txns):
  shard_count = 128

Extreme concurrency (> 200 concurrent txns):
  shard_count = 256
```

### WAL Configuration

```rust
use rusty_db::enterprise_optimization::wal_optimized::{
    StripedWALManager, WALConfig
};

let wal = StripedWALManager::new(
    wal_base_path,
    target_latency_ms: 5.0,        // Target 5ms latency
    max_commit_delay_ms: 100,      // Max 100ms delay
)?;

// PID controller parameters (advanced)
let pid_config = PIDConfig {
    kp: 0.5,                       // Proportional gain
    ki: 0.1,                       // Integral gain
    kd: 0.05,                      // Derivative gain
    min_batch_size: 1,
    max_batch_size: 1000,
};
```

**Latency vs Throughput Tradeoff**:
```
Low latency priority (OLTP):
  target_latency_ms = 1.0
  max_commit_delay_ms = 10
  stripe_count = 8  // Default

High throughput priority (batch):
  target_latency_ms = 10.0
  max_commit_delay_ms = 1000
  stripe_count = 16

Balanced:
  target_latency_ms = 5.0
  max_commit_delay_ms = 100
  stripe_count = 8
```

**Storage-Specific Tuning**:
```
NVMe SSD:
  target_latency_ms = 1.0
  max_batch_size = 500

SATA SSD:
  target_latency_ms = 5.0
  max_batch_size = 1000

HDD:
  target_latency_ms = 20.0
  max_batch_size = 2000
```

### Deadlock Detection

```rust
use rusty_db::enterprise_optimization::deadlock_detector::{
    OptimizedDeadlockDetector, DeadlockConfig
};

let config = DeadlockConfig {
    detection_epoch_threshold: 100,  // Detect every 100 graph updates
    initial_backoff_ms: 100,         // Initial timeout: 100ms
    max_backoff_ms: 10000,           // Max timeout: 10s
    backoff_multiplier: 2.0,         // Exponential factor
};
```

**Tuning for Contention Level**:
```
Low contention:
  detection_epoch_threshold = 1000  // Infrequent detection

Medium contention:
  detection_epoch_threshold = 100   // Default

High contention:
  detection_epoch_threshold = 10    // Frequent detection

Very high contention:
  detection_epoch_threshold = 1     // Every update (expensive!)
```

---

## Query Optimizer Tuning

### Hardware-Aware Cost Model

```rust
use rusty_db::enterprise_optimization::hardware_cost_calibration::{
    CalibratedCostModel, HardwareProfile
};

// Auto-detect hardware (recommended)
let hardware = HardwareProfile::auto_detect();
let cost_model = CalibratedCostModel::new(hardware);

// Or manually configure
let hardware = HardwareProfile {
    cpu_speed_ghz: 3.5,
    core_count: 32,
    memory_bandwidth_gbps: 51.2,
    memory_latency_ns: 100,
    disk_seq_iops: 200_000,
    disk_random_iops: 50_000,
    disk_throughput_mbps: 3500,
    ..Default::default()
};
```

**Calibration Interval**:
```
// Recalibrate every N executions
cost_model.set_calibration_interval(100);  // Default

Development/Testing:
  calibration_interval = 10      // Frequent recalibration

Production (stable workload):
  calibration_interval = 1000    // Infrequent recalibration

Production (variable workload):
  calibration_interval = 100     // Default
```

### Adaptive Execution Configuration

```rust
use rusty_db::enterprise_optimization::adaptive_execution::{
    AdaptiveExecutionEngine, AdaptiveConfig
};

let config = AdaptiveConfig {
    // Sampling
    sample_percentage: 0.10,       // Sample 10% of data

    // Plan switching threshold
    switch_threshold: 10.0,        // Switch if estimate is 10x off

    // Parallel degree
    min_parallel_degree: 1,
    max_parallel_degree: 32,       // Max 32 threads

    // Memory grants
    memory_grant_buffer: 1.2,      // 20% buffer
    max_memory_percentage: 0.25,   // Max 25% of total memory
};
```

**Tuning for Workload**:
```
OLTP (predictable, fast queries):
  sample_percentage = 0.05         // Less sampling
  switch_threshold = 20.0          // Higher threshold
  max_parallel_degree = 8          // Limited parallelism

OLAP (variable, complex queries):
  sample_percentage = 0.10         // Default sampling
  switch_threshold = 5.0           // Lower threshold
  max_parallel_degree = 32         // Full parallelism

Mixed:
  sample_percentage = 0.10
  switch_threshold = 10.0
  max_parallel_degree = 16
```

### Plan Baseline Configuration

```rust
use rusty_db::enterprise_optimization::plan_stability::{
    EnhancedBaselineManager, BaselineConfig
};

let config = BaselineConfig {
    // Capture criteria
    min_quality_score: 0.6,        // Minimum quality to capture
    auto_capture: true,            // Auto-capture good plans

    // Validation
    validate_before_capture: true,

    // Regression detection
    cost_regression_threshold: 1.5,      // 50% worse cost
    time_regression_threshold: 1.3,      // 30% worse time
    quality_regression_threshold: 0.8,   // 20% worse quality

    // Evolution
    allow_evolution: true,         // Allow better plans to replace
    max_plans_per_baseline: 10,    // Keep top 10 plans
};
```

**Tuning for Stability vs Performance**:
```
Stability priority (production):
  min_quality_score = 0.7
  cost_regression_threshold = 1.2    // Stricter
  allow_evolution = false            // No automatic changes

Performance priority (development):
  min_quality_score = 0.5
  cost_regression_threshold = 2.0    // Relaxed
  allow_evolution = true             // Allow improvements

Balanced:
  min_quality_score = 0.6            // Default
  cost_regression_threshold = 1.5
  allow_evolution = true
```

---

## Connection Pool Tuning

### Basic Pool Configuration

```rust
use rusty_db::pool::connection::ConnectionPoolConfig;

let config = ConnectionPoolConfig {
    min_connections: 10,
    max_connections: 100,
    connection_timeout_secs: 30,
    idle_timeout_secs: 300,          // 5 minutes
    max_lifetime_secs: 1800,         // 30 minutes
};
```

**Sizing Formula**:
```
OLTP workload:
  max_connections = num_cores * 2 to num_cores * 4
  min_connections = max_connections / 2

OLAP workload:
  max_connections = num_cores * 0.5 to num_cores * 1
  min_connections = 5

Mixed workload:
  max_connections = num_cores * 2
  min_connections = num_cores
```

### Session Multiplexing

```rust
use rusty_db::enterprise_optimization::session_multiplexer::{
    SessionMultiplexer, MultiplexerConfig
};

let config = MultiplexerConfig {
    max_connections: 1000,
    max_sessions: 10_000,
    session_timeout: Duration::from_secs(300),
    enable_affinity: true,
    session_ratio: 10,               // 10 sessions per connection
    ..Default::default()
};
```

**Session Ratio Tuning**:
```
OLTP (short, frequent queries):
  session_ratio = 20               // Aggressive multiplexing

OLAP (long queries):
  session_ratio = 2                // Conservative multiplexing

Mixed:
  session_ratio = 10               // Default

Connection-limited:
  session_ratio = 50               // Maximum multiplexing
```

### Adaptive Pool Sizing

```rust
use rusty_db::enterprise_optimization::adaptive_pool_sizing::{
    AdaptivePoolSizer, AdaptivePoolConfig, ScalingPolicy
};

let config = AdaptivePoolConfig {
    min_size: 10,
    max_size: 500,
    target_utilization: 0.70,        // Target 70% utilization

    // Scaling policy
    policy: ScalingPolicy::Balanced,

    // Thresholds
    scale_up_threshold: 0.80,        // Scale up at 80%
    scale_down_threshold: 0.40,      // Scale down at 40%

    // Rate limiting
    min_scale_change: 1,
    max_scale_change: 50,
    cooldown: Duration::from_secs(60),

    // Prediction
    enable_prediction: true,
    prediction_window: Duration::from_secs(300),
};
```

**Policy Selection**:
```
Aggressive (fast-growing workload):
  policy = ScalingPolicy::Aggressive
  scale_up_threshold = 0.70
  scale_down_threshold = 0.30
  cooldown = 30s

Conservative (stable workload):
  policy = ScalingPolicy::Conservative
  scale_up_threshold = 0.90
  scale_down_threshold = 0.50
  cooldown = 120s

Balanced (default):
  policy = ScalingPolicy::Balanced
  scale_up_threshold = 0.80
  scale_down_threshold = 0.40
  cooldown = 60s
```

### Connection Health Checking

```rust
use rusty_db::enterprise_optimization::connection_health::{
    AdaptiveHealthChecker, AdaptiveHealthConfig
};

let config = AdaptiveHealthConfig {
    base_interval: Duration::from_secs(30),
    min_interval: Duration::from_secs(5),
    max_interval: Duration::from_secs(300),
    healthy_multiplier: 3.0,         // Check healthy connections 3x less
    degraded_multiplier: 0.25,       // Check degraded 4x more
    enable_prediction: true,
};
```

**Tuning for Environment**:
```
Reliable network:
  base_interval = 60s
  healthy_multiplier = 5.0

Unreliable network:
  base_interval = 15s
  healthy_multiplier = 2.0

Cloud/virtualized:
  base_interval = 30s               // Default
  healthy_multiplier = 3.0
```

### Per-User Connection Limits

```rust
use rusty_db::enterprise_optimization::connection_limits::{
    ConnectionLimitManager, ConnectionLimit, LimitPolicy, Priority
};

// Configure limits per user/tenant
let limit = ConnectionLimit {
    id: "premium_user".to_string(),
    max_connections: 50,
    policy: LimitPolicy::Soft,
    burst_allowance: 15,             // Allow up to 65 in burst
    priority: Priority::High,
    ..Default::default()
};

manager.set_limit("premium_user", limit);
```

**Policy Selection**:
```
Strict enforcement:
  policy = LimitPolicy::Hard

Bursty workload:
  policy = LimitPolicy::Soft
  burst_allowance = 25% of max

Queue-based:
  policy = LimitPolicy::Quota

Guaranteed minimums:
  policy = LimitPolicy::Reserved
  reserved_connections = 10
```

---

## Concurrency Tuning

### Lock-Free Skip List

```rust
use rusty_db::enterprise_optimization::optimized_skiplist::OptimizedSkipList;

// Max height auto-adjusts based on size:
// <1K: 4 levels, 1K-10K: 8 levels, 10K-100K: 16 levels, >100K: 32 levels

// Configuration is automatic, but can force max height:
let skiplist = OptimizedSkipList::with_max_height(16);
```

### Work-Stealing Configuration

```rust
use rusty_db::enterprise_optimization::optimized_work_stealing::{
    WorkStealingScheduler, WorkStealingConfig
};

let config = WorkStealingConfig {
    initial_buffer_size: 64,         // Initial deque size
    numa_aware: true,                // Enable NUMA awareness
    adaptive_stealing: true,         // Adaptive policy
    steal_batch_size: 4,             // Steal 4 tasks at once
};
```

**Tuning for Hardware**:
```
Single NUMA node:
  numa_aware = false

Multi-socket (multiple NUMA):
  numa_aware = true
  steal_batch_size = 8             // Batch to amortize cross-NUMA cost

Many-core (>32 cores):
  initial_buffer_size = 128
  steal_batch_size = 8
```

### Epoch-Based Reclamation

```rust
use rusty_db::enterprise_optimization::optimized_epoch::{
    OptimizedEpochManager, EpochConfig
};

let config = EpochConfig {
    initial_interval_us: 1000,       // 1ms initial interval
    min_interval_us: 100,            // 100μs minimum
    max_interval_us: 10_000,         // 10ms maximum
    batch_size: 128,                 // Reclaim 128 objects per batch
    gc_threshold: 1000,              // Trigger GC at 1000 deferred objects
};
```

---

## I/O and Storage Tuning

### Direct I/O Configuration

```rust
use rusty_db::io::IoConfig;

let config = IoConfig {
    use_direct_io: true,             // Bypass OS cache
    io_depth: 128,                   // Queue depth for async I/O
    use_aio: true,                   // Use Linux AIO
    ..Default::default()
};
```

**Storage Type Recommendations**:
```
NVMe SSD:
  use_direct_io = true
  io_depth = 256

SATA SSD:
  use_direct_io = true
  io_depth = 128

HDD:
  use_direct_io = false            // OS cache helps
  io_depth = 32
```

### Read-Ahead Configuration

```
Prefetch settings (covered in Buffer Pool Tuning):
- Set initial_depth based on storage speed
- Enable adaptive_depth for automatic tuning
```

---

## Workload-Specific Configurations

### High-Throughput OLTP

```rust
// Configuration for e-commerce, high-frequency trading, etc.

// Buffer Pool
num_frames = 1_000_000             // 4 GB
eviction_policy = Arc

// Memory
max_versions_per_key = 1000        // Medium retention
transaction_size_profile = Small

// Transactions
shard_count = 128                  // High concurrency
wal_target_latency_ms = 1.0        // Low latency
stripe_count = 16                  // High parallelism

// Connection Pool
max_connections = 1000
session_ratio = 20                 // Aggressive multiplexing
adaptive_policy = Aggressive

// Query Optimizer
max_parallel_degree = 8            // Limited parallelism
switch_threshold = 20.0            // Stable plans
```

### Analytics Workload

```rust
// Configuration for data warehouse, reporting, BI

// Buffer Pool
num_frames = 10_000_000            // 40 GB
eviction_policy = Lirs             // Scan resistance
prefetch_max_depth = 64            // Aggressive prefetching

// Memory
max_versions_per_key = 10000       // Long queries
transaction_size_profile = Huge

// Transactions
shard_count = 32                   // Lower concurrency
wal_target_latency_ms = 10.0       // Higher latency OK

// Connection Pool
max_connections = 100
session_ratio = 2                  // Long-running queries

// Query Optimizer
max_parallel_degree = 32           // Full parallelism
switch_threshold = 5.0             // Adaptive plans
enable_prediction = true
```

### Mixed Workload

```rust
// Balanced configuration for general-purpose databases

// Buffer Pool
num_frames = 2_000_000             // 8 GB
eviction_policy = Arc
prefetch_max_depth = 32

// Memory
max_versions_per_key = 1000
transaction_size_profile = Medium

// Transactions
shard_count = 64
wal_target_latency_ms = 5.0
stripe_count = 8

// Connection Pool
max_connections = 500
session_ratio = 10
adaptive_policy = Balanced

// Query Optimizer
max_parallel_degree = 16
switch_threshold = 10.0
```

### Multi-Tenant SaaS

```rust
// Configuration for SaaS platforms with many tenants

// Connection Pool (critical for multi-tenancy)
max_connections = 2000
max_sessions = 20_000
session_ratio = 10

// Per-tenant limits
free_tier_limit = 5                // 5 connections
premium_tier_limit = 50            // 50 connections
enterprise_tier_limit = 200        // 200 with reservation

// Resource isolation
enable_per_user_limits = true
enable_affinity = true

// Other settings (standard)
num_frames = 5_000_000             // 20 GB
eviction_policy = Arc
shard_count = 128
```

---

## Monitoring and Validation

### Key Metrics to Track

After tuning, monitor these metrics to validate improvements:

**Buffer Pool**:
```
target: hit_ratio > 90%
target: eviction_rate < 1000/sec
target: page_fault_rate < 100/sec
```

**Memory**:
```
target: allocation_overhead < 5%
target: fragmentation_ratio < 0.20
target: pressure_events = 0
```

**Transactions**:
```
target: TPS > baseline * 1.5
target: lock_contention < 5%
target: deadlock_rate < 0.1%
```

**Queries**:
```
target: avg_execution_time < baseline * 0.75
target: plan_regression_rate < 5%
target: cache_hit_rate > 85%
```

**Connections**:
```
target: pool_utilization = 60-80%
target: wait_queue_length < 10
target: connection_churn < 10/sec
```

### Tuning Iteration Process

1. **Baseline**: Measure current performance
2. **Change**: Modify one parameter at a time
3. **Measure**: Run workload for 30+ minutes
4. **Validate**: Check if metrics improved
5. **Repeat**: Continue with next parameter

### Load Testing

Always validate tuning with realistic load tests:

```bash
# Run benchmark suite
cargo bench --release

# Stress test
cargo test --release -- --nocapture --test-threads=32

# Production simulation
./scripts/simulate_production_load.sh
```

---

## Conclusion

This tuning guide provides comprehensive parameter recommendations for all RustyDB v0.6.0 performance components. Always:

1. **Start with defaults** for general-purpose workloads
2. **Measure before tuning** to establish baseline
3. **Change one parameter at a time** for clear attribution
4. **Monitor continuously** to detect regressions
5. **Document changes** for repeatability

For specific performance issues, see:
- **MEMORY_TUNING.md** for memory-specific guidance
- **QUERY_OPTIMIZATION.md** for query tuning
- **BEST_PRACTICES.md** for general recommendations

---

**Document Version**: 1.0
**Last Updated**: December 2025
**Release**: v0.6.0
