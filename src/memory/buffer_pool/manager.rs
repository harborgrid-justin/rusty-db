// Buffer Pool Manager Public API
//
// Main interface for buffer pool operations.
//
// TODO: CRITICAL - TRIPLE BUFFER POOL DUPLICATION!
// This is BufferPoolManager implementation #3 of 3 with identical names.
//
// Three separate BufferPoolManager implementations exist:
//   1. src/storage/buffer.rs - COW semantics, NUMA, LRU-K eviction
//   2. src/buffer/manager.rs - Lock-free, per-core pools, IOCP, prefetch
//   3. src/memory/buffer_pool/manager.rs (THIS FILE) - Multi-tier, ARC, 2Q, checkpoint
//
// RECOMMENDATION: Consolidate enterprise features into canonical implementation
//   - Migrate multi-tier, ARC, 2Q features to src/buffer/manager.rs
//   - Rename this module to avoid naming conflicts (e.g., EnterpriseBufferPool)
//   - Keep checkpoint and double-write features as optional add-ons
//   - This should become a wrapper/extension, not a standalone manager
//   - Estimated effort: 3-5 days
//
// See: diagrams/02_storage_layer_flow.md - Issue #2.1

use super::arc::*;
use super::checkpoint::*;
use super::common::*;
use super::eviction_policies::*;
use super::multi_tier::*;
use super::prefetcher::*;
use super::statistics::*;
use super::two_q::*;
use super::writer::*;

pub struct BufferPoolManager {
    pool: Arc<MultiTierBufferPool>,
    arc_cache: Arc<AdaptiveReplacementCache>,
    two_q_cache: Arc<TwoQCache>,
    prefetcher: Arc<PagePrefetcher>,
    clock_policy: Arc<ClockSweepPolicy>,
    lru_k_policy: Arc<LruKPolicy>,
    touch_optimizer: Arc<TouchCountOptimizer>,
    cost_aware: Arc<CostAwareReplacement>,
    checkpoint_queue: Arc<CheckpointQueue>,
    incremental_checkpointer: Arc<IncrementalCheckpointer>,
    background_writer: Arc<BackgroundWriter>,
    write_coalescing: Arc<WriteCoalescingBuffer>,
    double_write: Arc<DoubleWriteBuffer>,
    flush_manager: Arc<FlushListManager>,
    stats_tracker: Arc<BufferPoolStatisticsTracker>,
}

impl BufferPoolManager {
    // Create a new buffer pool manager
    pub fn new(config: BufferPoolConfig) -> Self {
        let pool = Arc::new(MultiTierBufferPool::new(config.clone()));
        let capacity = config.total_size / config.page_size;
        let checkpoint_queue = Arc::new(CheckpointQueue::new());

        Self {
            pool: pool.clone(),
            arc_cache: Arc::new(AdaptiveReplacementCache::new(capacity)),
            two_q_cache: Arc::new(TwoQCache::new(capacity)),
            prefetcher: Arc::new(PagePrefetcher::new(8)),
            clock_policy: Arc::new(ClockSweepPolicy::new(capacity, config.page_size)),
            lru_k_policy: Arc::new(LruKPolicy::new(2, 300)),
            touch_optimizer: Arc::new(TouchCountOptimizer::new(10)),
            cost_aware: Arc::new(CostAwareReplacement::new()),
            checkpoint_queue: checkpoint_queue.clone(),
            incremental_checkpointer: Arc::new(IncrementalCheckpointer::new(
                60,
                100,
                checkpoint_queue.clone(),
            )),
            background_writer: Arc::new(BackgroundWriter::new(50, 5, 0.75)),
            write_coalescing: Arc::new(WriteCoalescingBuffer::new(100)),
            double_write: Arc::new(DoubleWriteBuffer::new(128)),
            flush_manager: Arc::new(FlushListManager::new(100)),
            stats_tracker: Arc::new(BufferPoolStatisticsTracker::new()),
        }
    }

    // Pin a page (web API endpoint)
    pub fn api_pin_page(&self, tablespace_id: u32, page_number: u64) -> Option<Arc<BufferFrame>> {
        let page_id = PageId::new(tablespace_id, page_number);
        self.pool.pin_page(page_id, PoolType::Default)
    }

    // Unpin a page (web API endpoint)
    pub fn api_unpin_page(&self, tablespace_id: u32, page_number: u64, dirty: bool) -> bool {
        let page_id = PageId::new(tablespace_id, page_number);
        self.pool.unpin_page(page_id, dirty)
    }

    // Get buffer pool statistics (web API endpoint)
    pub fn api_get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "buffer_pool": self.pool.get_stats(),
            "arc_cache": self.arc_cache.get_stats(),
            "two_q_cache": self.two_q_cache.get_stats(),
            "prefetcher": self.prefetcher.get_stats(),
            "clock_policy": self.clock_policy.get_stats(),
            "lru_k_policy": self.lru_k_policy.get_stats(),
            "touch_optimizer": self.touch_optimizer.get_stats(),
            "cost_aware": self.cost_aware.get_stats(),
            "checkpoint": self.checkpoint_queue.get_stats(),
            "incremental_checkpoint": self.incremental_checkpointer.get_stats(),
            "background_writer": self.background_writer.get_stats(),
            "write_coalescing": self.write_coalescing.get_stats(),
            "double_write": self.double_write.get_stats(),
            "flush_manager": self.flush_manager.get_stats(),
            "comprehensive": self.stats_tracker.get_comprehensive_stats(),
        })
    }

    // Flush all dirty pages (web API endpoint)
    pub fn api_flush_all(&self) -> usize {
        self.pool.flush_all()
    }

    // Perform checkpoint (web API endpoint)
    pub fn api_checkpoint(&self) -> CheckpointResult {
        self.checkpoint_queue.checkpoint()
    }

    // Get memory pressure (web API endpoint)
    pub fn api_get_memory_pressure(&self) -> MemoryPressureSnapshot {
        self.stats_tracker.memory_pressure.snapshot()
    }

    // Export Prometheus metrics (web API endpoint)
    pub fn api_export_prometheus(&self) -> String {
        self.stats_tracker.export_prometheus()
    }

    // Export JSON metrics (web API endpoint)
    pub fn api_export_json(&self) -> String {
        self.stats_tracker.export_json()
    }

    // Start background operations (web API endpoint)
    pub fn api_start_background_operations(&self) {
        self.pool.start_tier_manager();
        self.incremental_checkpointer.start();
        self.background_writer.start();
    }

    // Stop background operations (web API endpoint)
    pub fn api_stop_background_operations(&self) {
        self.pool.stop_tier_manager();
        self.incremental_checkpointer.stop();
        self.background_writer.stop();
    }

    // Get buffer pool capacity (web API endpoint)
    pub fn api_get_capacity(&self) -> usize {
        self.pool.capacity()
    }

    // Get frames in use (web API endpoint)
    pub fn api_get_frames_in_use(&self) -> usize {
        self.pool.frames_in_use()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_creation() {
        let config = BufferPoolConfig::default();
        let pool = MultiTierBufferPool::new(config);
        assert!(pool.capacity() > 0);
    }

    #[test]
    fn test_page_pin_unpin() {
        let config = BufferPoolConfig::default();
        let pool = MultiTierBufferPool::new(config);
        let page_id = PageId::new(0, 1);

        let frame = pool.pin_page(page_id, PoolType::Default);
        assert!(frame.is_some());

        let unpinned = pool.unpin_page(page_id, false);
        assert!(unpinned);
    }

    #[test]
    fn test_arc_cache() {
        let arc = AdaptiveReplacementCache::new(100);
        let page_id = PageId::new(0, 1);
        let frame = Arc::new(BufferFrame::new(8192));

        arc.insert(page_id, frame.clone());
        let retrieved = arc.get(page_id, 8192);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_clock_sweep() {
        let clock = ClockSweepPolicy::new(10, 8192);
        let victim = clock.find_victim();
        assert!(victim.is_some());
    }

    #[test]
    fn test_checkpoint_queue() {
        let queue = CheckpointQueue::new();
        let page_id = PageId::new(0, 1);
        let frame = Arc::new(BufferFrame::new(8192));

        let dirty_page = DirtyPage {
            page_id,
            lsn: 100,
            dirty_time: Instant::now(),
            frame,
        };

        queue.enqueue(dirty_page);
        assert_eq!(queue.dirty_count(), 1);
    }
}
