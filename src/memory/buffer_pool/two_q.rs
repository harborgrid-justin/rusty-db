// 2Q Cache Algorithm
//
// Two-queue cache with A1/Am/Aout lists.

use super::common::*;
use serde::{Serialize, Deserialize};

pub struct TwoQCache {
    // Maximum cache size
    capacity: usize,
    // A1in size (typically 25% of capacity)
    a1in_size: usize,
    // A1out size (typically 50% of capacity)
    a1out_size: usize,
    // A1in queue (FIFO for new pages)
    a1in: Mutex<VecDeque<PageId>>,
    // A1out queue (ghost entries)
    a1out: Mutex<VecDeque<PageId>>,
    // Am queue (LRU for frequent pages)
    am: Mutex<VecDeque<PageId>>,
    // Page directory
    directory: PRwLock<HashMap<PageId, TwoQLocation>>,
    // Frame storage
    frames: PRwLock<HashMap<PageId, Arc<BufferFrame>>>,
    // Statistics
    stats: TwoQStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TwoQLocation {
    A1In,
    A1Out,
    Am,
}

#[derive(Debug)]
struct TwoQStats {
    hits_a1in: AtomicU64,
    hits_am: AtomicU64,
    misses: AtomicU64,
    promotions: AtomicU64,
    evictions: AtomicU64,
}

impl TwoQStats {
    fn new() -> Self {
        Self {
            hits_a1in: AtomicU64::new(0),
            hits_am: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            promotions: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }
}

impl TwoQCache {
    // Create new 2Q cache
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            a1in_size: capacity / 4,
            a1out_size: capacity / 2,
            a1in: Mutex::new(VecDeque::new()),
            a1out: Mutex::new(VecDeque::new()),
            am: Mutex::new(VecDeque::new()),
            directory: PRwLock::new(HashMap::new()),
            frames: PRwLock::new(HashMap::new()),
            stats: TwoQStats::new(),
        }
    }

    // Access a page
    pub fn get(&self, page_id: PageId) -> Option<Arc<BufferFrame>> {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            match location {
                TwoQLocation::A1In => {
                    self.stats.hits_a1in.fetch_add(1, Ordering::Relaxed);
                }
                TwoQLocation::Am => {
                    self.stats.hits_am.fetch_add(1, Ordering::Relaxed);
                    self.touch_am(page_id);
                }
                TwoQLocation::A1Out => {
                    // Ghost hit - promote to Am
                    self.stats.promotions.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
            }

            let frames = self.frames.read();
            return frames.get(&page_id).cloned();
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    // Insert a new page
    pub fn insert(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let dir = self.directory.read();

        if let Some(&location) = dir.get(&page_id) {
            drop(dir);

            match location {
                TwoQLocation::A1Out => {
                    // Promote to Am
                    self.remove_from_a1out(page_id);
                    self.ensure_am_space();
                    self.add_to_am(page_id, frame);
                    return;
                }
                _ => {
                    // Already in cache
                    return;
                }
            }
        }
        drop(dir);

        // New page - add to A1in
        self.ensure_a1in_space();
        self.add_to_a1in(page_id, frame);
    }

    fn ensure_a1in_space(&self) {
        let mut a1in = self.a1in.lock();
        if a1in.len() >= self.a1in_size {
            if let Some(evict_page) = a1in.pop_front() {
                drop(a1in);

                // Move to A1out (ghost)
                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                self.add_to_a1out(evict_page);
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn ensure_am_space(&self) {
        let am_capacity = self.capacity - self.a1in_size;
        let mut am = self.am.lock();

        if am.len() >= am_capacity {
            if let Some(evict_page) = am.pop_front() {
                drop(am);

                let mut frames = self.frames.write();
                frames.remove(&evict_page);
                drop(frames);

                let mut dir = self.directory.write();
                dir.remove(&evict_page);
                drop(dir);

                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn add_to_a1in(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut a1in = self.a1in.lock();
        a1in.push_back(page_id);
        drop(a1in);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, TwoQLocation::A1In);
    }

    fn add_to_a1out(&self, page_id: PageId) {
        let mut a1out = self.a1out.lock();
        a1out.push_back(page_id);

        // Limit A1out size
        while a1out.len() > self.a1out_size {
            if let Some(evict) = a1out.pop_front() {
                let mut dir = self.directory.write();
                dir.remove(&evict);
            }
        }
        drop(a1out);

        let mut dir = self.directory.write();
        dir.insert(page_id, TwoQLocation::A1Out);
    }

    fn add_to_am(&self, page_id: PageId, frame: Arc<BufferFrame>) {
        let mut am = self.am.lock();
        am.push_back(page_id);
        drop(am);

        let mut frames = self.frames.write();
        frames.insert(page_id, frame);
        drop(frames);

        let mut dir = self.directory.write();
        dir.insert(page_id, TwoQLocation::Am);
    }

    fn touch_am(&self, page_id: PageId) {
        let mut am = self.am.lock();
        am.retain(|&id| id != page_id);
        am.push_back(page_id);
    }

    fn remove_from_a1out(&self, page_id: PageId) {
        let mut a1out = self.a1out.lock();
        a1out.retain(|&id| id != page_id);
    }

    // Get cache statistics
    pub fn get_stats(&self) -> TwoQStatsSnapshot {
        TwoQStatsSnapshot {
            hits_a1in: self.stats.hits_a1in.load(Ordering::Relaxed),
            hits_am: self.stats.hits_am.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            promotions: self.stats.promotions.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            a1in_size: self.a1in.lock().len(),
            a1out_size: self.a1out.lock().len(),
            am_size: self.am.lock().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoQStatsSnapshot {
    pub hits_a1in: u64,
    pub hits_am: u64,
    pub misses: u64,
    pub promotions: u64,
    pub evictions: u64,
    pub a1in_size: usize,
    pub a1out_size: usize,
    pub am_size: usize,
}
