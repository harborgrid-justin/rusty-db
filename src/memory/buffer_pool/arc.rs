// Adaptive Replacement Cache (ARC)
//
// Self-tuning cache balancing recency and frequency.

use super::common::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::collections::{HashMap, VecDeque};
use parking_lot::{Mutex, RwLock as PRwLock};

// ============================================================================
// SECTION 2: PAGE CACHE MANAGEMENT (600+ lines)
// ============================================================================

// Adaptive Replacement Cache (ARC) implementation
//
// ARC maintains four lists:
// - T1: Recently accessed pages (once)
// - T2: Frequently accessed pages (multiple times)
// - B1: Ghost entries for recently evicted from T1
// - B2: Ghost entries for recently evicted from T2
pub struct AdaptiveReplacementCache {
    // Target size for T1
    p: AtomicUsize,
    // Maximum cache size
    c: usize,
    // T1: Recent cache (frequency = 1)
    t1: Mutex<VecDeque<PageId>>,
    // T2: Frequent cache (frequency > 1)
    t2: Mutex<VecDeque<PageId>>,
    // B1: Ghost entries for T1
    b1: Mutex<VecDeque<PageId>>,
    // B2: Ghost entries for T2
    b2: Mutex<VecDeque<PageId>>,
    // Page directory mapping PageId to location
    directory: PRwLock<HashMap<PageId, CacheLocation>>,
    // Frame storage
    frames: PRwLock<HashMap<PageId, Arc<BufferFrame>>>,
    // Statistics
    stats: ArcStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheLocation {
    T1,
    T2,
    B1,
    B2,
}

#[derive(Debug)]
struct ArcStats {
    hits_t1: AtomicU64,
    hits_t2: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    ghost_hits_b1: AtomicU64,
    ghost_hits_b2: AtomicU64,
}

impl ArcStats {
    fn new() -> Self {
        Self {
            hits_t1: AtomicU64::new(0),
            hits_t2: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            ghost_hits_b1: AtomicU64::new(0),
            ghost_hits_b2: AtomicU64::new(0),
        }
    }
}

impl AdaptiveReplacementCache {
    // Create new ARC cache with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            p: AtomicUsize::new(0),
            c: capacity,
            t1: Mutex::new(VecDeque::new()),
            t2: Mutex::new(VecDeque::new()),
            b1: Mutex::new(VecDeque::new()),
            b2: Mutex::new(VecDeque::new()),
            directory: PRwLock::new(HashMap::new()),
            frames: PRwLock::new(HashMap::new()),
            stats: ArcStats::new(),
        }
    }

    // Access a page in the cache
    pub fn get(&self, page_id: PageId, _page_size: usize) -> Option<Arc<BufferFrame>> {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            match location {
                CacheLocation::T1 => {
                    self.stats.hits_t1.fetch_add(1, Ordering::Relaxed);
                    // Move from T1 to T2 (accessed more than once)
                    self.move_t1_to_t2(page_id);
                }
                CacheLocation::T2 => {
                    self.stats.hits_t2.fetch_add(1, Ordering::Relaxed);
                    // Move to MRU position in T2
                    self.touch_t2(page_id);
                }
                CacheLocation::B1 => {
                    self.stats.ghost_hits_b1.fetch_add(1, Ordering::Relaxed);
                    // Increase p (favor recency)
                    self.adapt_on_b1_hit();
                    return None; // Ghost entry, need to load from disk
                }
                CacheLocation::B2 => {
                    self.stats.ghost_hits_b2.fetch_add(1, Ordering::Relaxed);
                    // Decrease p (favor frequency)
                    self.adapt_on_b2_hit();
                    return None; // Ghost entry, need to load from disk
                }
            }

            let frames = self.frames.read();
            return frames.get(&page_id).cloned();
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    // Insert a new page into the cache
    pub fn insert(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            // Handle ghost hits
            match location {
                CacheLocation::B1 => {
                    self.adapt_on_b1_hit();
                    self.replace(page_id, CacheLocation::B1);
                    self.remove_from_b1(page_id);
                    self.add_to_t2(page_id, frame);
                    return;
                }
                CacheLocation::B2 => {
                    self.adapt_on_b2_hit();
                    self.replace(page_id, CacheLocation::B2);
                    self.remove_from_b2(page_id);
                    self.add_to_t2(page_id, frame);
                    return;
                }
                _ => {
                    // Already in cache, just update
                    return;
                }
            }
        }
        drop(dir);

        // New page, add to T1
        let t1_len = self.t1.lock().len();
        let t2_len = self.t2.lock().len();

        if t1_len + t2_len >= self.c {
            self.replace(page_id, CacheLocation::T1);
        }

        self.add_to_t1(page_id, frame);
    }

    // ARC replacement algorithm
    fn replace(&self, _page_id: PageId, hit_location: CacheLocation) {
        let p = self.p.load(Ordering::Relaxed);
        let t1_len = self.t1.lock().len();

        let evict_from_t1 = if t1_len > 0 {
            if t1_len > p || (hit_location == CacheLocation::B2 && t1_len == p) {
                true
            } else {
                false
            }
        } else {
            false
        };

        if evict_from_t1 {
            // Evict from T1
            let mut t1 = self.t1.lock();
            if let Some(evict_page) = t1.pop_front() {
                drop(t1);

                // Move to B1
                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                self.add_to_b1(evict_page);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            // Evict from T2
            let mut t2 = self.t2.lock();
            if let Some(evict_page) = t2.pop_front() {
                drop(t2);

                // Move to B2
                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                self.add_to_b2(evict_page);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    // Adapt parameter p on B1 hit
    fn adapt_on_b1_hit(&self) {
        let b1_len = self.b1.lock().len();
        let b2_len = self.b2.lock().len();

        let delta = if b1_len >= b2_len {
            1
        } else {
            b2_len / b1_len
        };

        let current_p = self.p.load(Ordering::Relaxed);
        let new_p = std::cmp::min(current_p + delta, self.c);
        self.p.store(new_p, Ordering::Relaxed);
    }

    // Adapt parameter p on B2 hit
    fn adapt_on_b2_hit(&self) {
        let b1_len = self.b1.lock().len();
        let b2_len = self.b2.lock().len();

        let delta = if b2_len >= b1_len {
            1
        } else {
            b1_len / b2_len
        };

        let current_p = self.p.load(Ordering::Relaxed);
        let new_p = current_p.saturating_sub(delta);
        self.p.store(new_p, Ordering::Relaxed);
    }

    // Helper methods for list management
    fn add_to_t1(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut t1 = self.t1.lock();
        t1.push_back(page_id);
        drop(t1);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::T1);
    }

    fn add_to_t2(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut t2 = self.t2.lock();
        t2.push_back(page_id);
        drop(t2);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::T2);
    }

    fn add_to_b1(&self, page_id: PageId) {
        let mut b1 = self.b1.lock();
        b1.push_back(page_id);

        // Limit B1 size
        while b1.len() > self.c {
            if let Some(evict) = b1.pop_front() {
                let mut dir = self.directory.write();
                dir.remove(&evict);
            }
        }
        drop(b1);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::B1);
    }

    fn add_to_b2(&self, page_id: PageId) {
        let mut b2 = self.b2.lock();
        b2.push_back(page_id);

        // Limit B2 size
        while b2.len() > self.c {
            if let Some(evict) = b2.pop_front() {
                let mut dir = self.directory.write();
                dir.remove(&evict);
            }
        }
        drop(b2);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::B2);
    }

    fn move_t1_to_t2(&self, page_id: PageId) {
        let mut t1 = self.t1.lock();
        t1.retain(|&id| id != page_id);
        drop(t1);

        let mut t2 = self.t2.lock();
        t2.push_back(page_id);
        drop(t2);

        let mut dir = self.directory.write();
        dir.insert(page_id, CacheLocation::T2);
    }

    fn touch_t2(&self, page_id: PageId) {
        let mut t2 = self.t2.lock();
        t2.retain(|&id| id != page_id);
        t2.push_back(page_id);
    }

    fn remove_from_b1(&self, page_id: PageId) {
        let mut b1 = self.b1.lock();
        b1.retain(|&id| id != page_id);
    }

    fn remove_from_b2(&self, page_id: PageId) {
        let mut b2 = self.b2.lock();
        b2.retain(|&id| id != page_id);
    }

    // Get cache statistics
    pub fn get_stats(&self) -> ArcStatsSnapshot {
        ArcStatsSnapshot {
            hits_t1: self.stats.hits_t1.load(Ordering::Relaxed),
            hits_t2: self.stats.hits_t2.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            ghost_hits_b1: self.stats.ghost_hits_b1.load(Ordering::Relaxed),
            ghost_hits_b2: self.stats.ghost_hits_b2.load(Ordering::Relaxed),
            t1_size: self.t1.lock().len(),
            t2_size: self.t2.lock().len(),
            b1_size: self.b1.lock().len(),
            b2_size: self.b2.lock().len(),
            p_value: self.p.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcStatsSnapshot {
    pub hits_t1: u64,
    pub hits_t2: u64,
    pub misses: u64,
    pub evictions: u64,
    pub ghost_hits_b1: u64,
    pub ghost_hits_b2: u64,
    pub t1_size: usize,
    pub t2_size: usize,
    pub b1_size: usize,
    pub b2_size: usize,
    pub p_value: usize,
}
