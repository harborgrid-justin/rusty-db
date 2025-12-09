/// Free Frame Manager - Manages allocation/deallocation of buffer frames

use crate::buffer::page_cache::{FrameId, PerCoreFramePool};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Manages free frames using a lock-free stack or per-core pools
pub struct FreeFrameManager {
    /// Global free list (fallback)
    global_free_list: Mutex<Vec<FrameId>>,

    /// Per-core pools (optional)
    per_core_pools: Option<Vec<Arc<PerCoreFramePool>>>,

    /// Number of CPU cores
    num_cores: usize,

    /// Statistics
    global_allocations: AtomicU64,
    per_core_allocations: AtomicU64,
}

impl FreeFrameManager {
    /// Create a new free frame manager
    pub fn new(
        num_frames: usize,
        enable_per_core_pools: bool,
        frames_per_core: usize,
    ) -> Self {
        let num_cores = num_cpus::get();

        let per_core_pools = if enable_per_core_pools {
            let pools: Vec<_> = (0..num_cores)
                .map(|i| Arc::new(PerCoreFramePool::new(i, frames_per_core)))
                .collect();

            // Distribute initial frames to pools
            let mut frame_id = 0;
            for pool in &pools {
                let mut frames = Vec::with_capacity(frames_per_core);
                for _ in 0..frames_per_core {
                    if frame_id < num_frames as FrameId {
                        frames.push(frame_id);
                        frame_id += 1;
                    }
                }
                pool.add_frames(frames);
            }

            Some(pools)
        } else {
            None
        };

        // Remaining frames go to global list
        let global_frames: Vec<FrameId> = if enable_per_core_pools {
            let start = (num_cores * frames_per_core) as FrameId;
            (start..num_frames as FrameId).collect()
        } else {
            (0..num_frames as FrameId).collect()
        };

        Self {
            global_free_list: Mutex::new(global_frames),
            per_core_pools,
            num_cores,
            global_allocations: AtomicU64::new(0),
            per_core_allocations: AtomicU64::new(0),
        }
    }

    /// Allocate a free frame
    #[inline]
    pub fn allocate(&self) -> Option<FrameId> {
        // Try per-core pool first
        if let Some(ref pools) = self.per_core_pools {
            let core_id = get_current_core_id() % self.num_cores;
            if let Some(frame_id) = pools[core_id].try_allocate() {
                self.per_core_allocations.fetch_add(1, Ordering::Relaxed);
                return Some(frame_id);
            }

            // Try stealing from other cores
            for i in 0..self.num_cores {
                let steal_core = (core_id + i) % self.num_cores;
                if let Some(frame_id) = pools[steal_core].try_allocate() {
                    self.per_core_allocations.fetch_add(1, Ordering::Relaxed);
                    return Some(frame_id);
                }
            }
        }

        // Fall back to global list
        self.global_free_list.lock().pop().map(|frame_id| {
            self.global_allocations.fetch_add(1, Ordering::Relaxed);
            frame_id
        })
    }

    /// Deallocate a frame
    #[inline]
    pub fn deallocate(&self, frame_id: FrameId) {
        // Try to add to per-core pool first
        if let Some(ref pools) = self.per_core_pools {
            let core_id = get_current_core_id() % self.num_cores;
            if pools[core_id].deallocate(frame_id) {
                return;
            }
        }

        // Add to global list
        self.global_free_list.lock().push(frame_id);
    }

    /// Get number of free frames
    #[inline]
    pub fn free_count(&self) -> usize {
        let mut count = self.global_free_list.lock().len();

        if let Some(ref pools) = self.per_core_pools {
            count += pools.iter().map(|p| p.free_count()).sum::<usize>();
        }

        count
    }

    /// Get statistics
    #[cold]
    pub fn stats(&self) -> (u64, u64, usize) {
        (
            self.global_allocations.load(Ordering::Relaxed),
            self.per_core_allocations.load(Ordering::Relaxed),
            self.free_count(),
        )
    }
}

/// Get current CPU core ID (best effort)
#[inline]
pub fn get_current_core_id() -> usize {
    // On Linux, we can use sched_getcpu
    #[cfg(all(target_os = "linux", feature = "libc"))]
    {
        unsafe { libc::sched_getcpu() as usize }
    }

    // On other platforms, use thread ID as proxy
    #[cfg(not(all(target_os = "linux", feature = "libc")))]
    {
        // Use a hash of the thread ID to get a pseudo-random core ID
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        hasher.finish() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_frame_manager() {
        let manager = FreeFrameManager::new(10, false, 0);

        let frame1 = manager.allocate();
        assert!(frame1.is_some());

        let frame2 = manager.allocate();
        assert!(frame2.is_some());

        assert_ne!(frame1, frame2);

        manager.deallocate(frame1.unwrap());
        let frame3 = manager.allocate();
        assert_eq!(frame3, frame1);
    }

    #[test]
    fn test_free_count() {
        let manager = FreeFrameManager::new(10, false, 0);
        assert_eq!(manager.free_count(), 10);

        let _ = manager.allocate();
        assert_eq!(manager.free_count(), 9);

        let _ = manager.allocate();
        assert_eq!(manager.free_count(), 8);
    }
}
