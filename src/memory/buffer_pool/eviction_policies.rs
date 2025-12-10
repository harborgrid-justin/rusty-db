// Buffer Replacement Policies
//
// Clock-Sweep, LRU-K, Touch Count Optimizer, and Cost-Aware replacement.

use super::common::*;
use serde::{Serialize, Deserialize};

// Clock-Sweep (Second-Chance) algorithm implementation
pub struct ClockSweepPolicy {
    // Clock hand position
    hand: AtomicUsize,
    // Buffer frames
    frames: Vec<Arc<BufferFrame>>,
    // Reference bits
    reference_bits: Vec<AtomicBool>,
    // Statistics
    stats: ClockStats,
}

#[derive(Debug)]
struct ClockStats {
    sweeps: AtomicU64,
    evictions: AtomicU64,
    second_chances: AtomicU64,
}

impl ClockSweepPolicy {
    pub fn new(capacity: usize, page_size: usize) -> Self {
        let mut frames = Vec::new();
        let mut reference_bits = Vec::new();

        for _ in 0..capacity {
            frames.push(Arc::new(BufferFrame::new(page_size)));
            reference_bits.push(AtomicBool::new(false));
        }

        Self {
            hand: AtomicUsize::new(0),
            frames,
            reference_bits,
            stats: ClockStats {
                sweeps: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                second_chances: AtomicU64::new(0),
            },
        }
    }

    // Find victim page for eviction
    pub fn find_victim(&self) -> Option<usize> {
        let capacity = self.frames.len();
        let mut current_hand = self.hand.load(Ordering::Relaxed);

        loop {
            self.stats.sweeps.fetch_add(1, Ordering::Relaxed);

            // Check if frame is pinned
            if self.frames[current_hand].pin_count() > 0 {
                current_hand = (current_hand + 1) % capacity;
                continue;
            }

            // Check reference bit
            let had_reference = self.reference_bits[current_hand].swap(false, Ordering::Relaxed);

            if !had_reference {
                // Found victim
                self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                self.hand.store((current_hand + 1) % capacity, Ordering::Relaxed);
                return Some(current_hand);
            } else {
                // Give second chance
                self.stats.second_chances.fetch_add(1, Ordering::Relaxed);
                current_hand = (current_hand + 1) % capacity;
            }
        }
    }

    // Set reference bit for a frame
    pub fn set_reference(&self, frame_idx: usize) {
        if frame_idx < self.reference_bits.len() {
            self.reference_bits[frame_idx].store(true, Ordering::Relaxed);
        }
    }

    // Get frame at index
    pub fn get_frame(&self, idx: usize) -> Option<Arc<BufferFrame>> {
        self.frames.get(idx).cloned()
    }

    // Get statistics
    pub fn get_stats(&self) -> ClockStatsSnapshot {
        ClockStatsSnapshot {
            sweeps: self.stats.sweeps.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            second_chances: self.stats.second_chances.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockStatsSnapshot {
    pub sweeps: u64,
    pub evictions: u64,
    pub second_chances: u64,
}

// LRU-K (K = 2) implementation - tracks K most recent accesses
pub struct LruKPolicy {
    // K value (typically 2)
    k: usize,
    // Access history for each page
    history: PRwLock<HashMap<PageId, VecDeque<Instant>>>,
    // Correlation period for history
    corr_period: Duration,
    // Statistics
    stats: LruKStats,
}

#[derive(Debug)]
struct LruKStats {
    accesses: AtomicU64,
    evictions: AtomicU64,
    history_promotions: AtomicU64,
}

impl LruKPolicy {
    pub fn new(k: usize, corr_period_secs: u64) -> Self {
        Self {
            k,
            history: PRwLock::new(HashMap::new()),
            corr_period: Duration::from_secs(corr_period_secs),
            stats: LruKStats {
                accesses: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                history_promotions: AtomicU64::new(0),
            },
        }
    }

    // Record page access
    pub fn access(&self, page_id: PageId) {
        self.stats.accesses.fetch_add(1, Ordering::Relaxed);

        let mut history = self.history.write();
        let page_history = history.entry(page_id).or_insert_with(VecDeque::new);

        page_history.push_back(Instant::now());

        // Keep only K most recent accesses
        if page_history.len() > self.k {
            page_history.pop_front();
            self.stats.history_promotions.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Calculate backward K-distance for a page
    pub fn backward_k_distance(&self, page_id: PageId) -> Option<Duration> {
        let history = self.history.read();
        if let Some(page_history) = history.get(&page_id) {
            if page_history.len() >= self.k {
                // K-th most recent access
                if let Some(&kth_access) = page_history.get(page_history.len() - self.k) {
                    return Some(kth_access.elapsed());
                }
            } else if let Some(&first_access) = page_history.front() {
                // Not enough history, use oldest access
                return Some(first_access.elapsed());
            }
        }
        None
    }

    // Find victim page (largest backward K-distance)
    pub fn find_victim(&self, candidates: &[PageId]) -> Option<PageId> {
        let mut max_distance = Duration::ZERO;
        let mut victim = None;

        for &page_id in candidates {
            if let Some(distance) = self.backward_k_distance(page_id) {
                if distance > max_distance {
                    max_distance = distance;
                    victim = Some(page_id);
                }
            } else {
                // No history = infinite distance, best victim
                return Some(page_id);
            }
        }

        if victim.is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }

        victim
    }

    // Clean old history entries
    pub fn clean_old_history(&self) {
        let mut history = self.history.write();
        let cutoff = Instant::now() - self.corr_period;

        history.retain(|_, page_history| {
            page_history.retain(|&access_time| access_time > cutoff);
            !page_history.is_empty()
        });
    }

    // Get statistics
    pub fn get_stats(&self) -> LruKStatsSnapshot {
        LruKStatsSnapshot {
            accesses: self.stats.accesses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            history_promotions: self.stats.history_promotions.load(Ordering::Relaxed),
            history_entries: self.history.read().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LruKStatsSnapshot {
    pub accesses: u64,
    pub evictions: u64,
    pub history_promotions: u64,
    pub history_entries: usize,
}

// Touch count optimization for hot pages
pub struct TouchCountOptimizer {
    // Touch counts per page
    touch_counts: PRwLock<HashMap<PageId, AtomicU64>>,
    // Hot threshold
    hot_threshold: u64,
    // Statistics
    stats: TouchCountStats,
}

#[derive(Debug)]
struct TouchCountStats {
    total_touches: AtomicU64,
    _hot_pages: AtomicU64,
    _warm_pages: AtomicU64,
    _cold_pages: AtomicU64,
}

impl TouchCountOptimizer {
    pub fn new(hot_threshold: u64) -> Self {
        Self {
            touch_counts: PRwLock::new(HashMap::new()),
            hot_threshold,
            stats: TouchCountStats {
                total_touches: AtomicU64::new(0),
                _hot_pages: AtomicU64::new(0),
                _warm_pages: AtomicU64::new(0),
                _cold_pages: AtomicU64::new(0),
            },
        }
    }

    // Record page touch
    pub fn touch(&self, page_id: PageId) {
        self.stats.total_touches.fetch_add(1, Ordering::Relaxed);

        let counts = self.touch_counts.read();
        if let Some(count) = counts.get(&page_id) {
            count.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(counts);
            let mut counts = self.touch_counts.write();
            counts.entry(page_id).or_insert_with(|| AtomicU64::new(1));
        }
    }

    // Get touch count for a page
    pub fn get_count(&self, page_id: PageId) -> u64 {
        let counts = self.touch_counts.read();
        counts.get(&page_id)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    // Determine page temperature
    pub fn temperature(&self, page_id: PageId) -> BufferTier {
        let count = self.get_count(page_id);

        if count >= self.hot_threshold {
            BufferTier::Hot
        } else if count >= self.hot_threshold / 2 {
            BufferTier::Warm
        } else {
            BufferTier::Cold
        }
    }

    // Reset touch count for a page
    pub fn reset(&self, page_id: PageId) {
        let counts = self.touch_counts.read();
        if let Some(count) = counts.get(&page_id) {
            count.store(0, Ordering::Relaxed);
        }
    }

    // Decay all touch counts (age out old activity)
    pub fn decay_all(&self, factor: f64) {
        let counts = self.touch_counts.read();
        for (_, count) in counts.iter() {
            let current = count.load(Ordering::Relaxed);
            let new_value = (current as f64 * factor) as u64;
            count.store(new_value, Ordering::Relaxed);
        }
    }

    // Get statistics
    pub fn get_stats(&self) -> TouchCountStatsSnapshot {
        let counts = self.touch_counts.read();
        let mut hot = 0u64;
        let mut warm = 0u64;
        let mut cold = 0u64;

        for (_, count) in counts.iter() {
            let c = count.load(Ordering::Relaxed);
            if c >= self.hot_threshold {
                hot += 1;
            } else if c >= self.hot_threshold / 2 {
                warm += 1;
            } else {
                cold += 1;
            }
        }

        TouchCountStatsSnapshot {
            total_touches: self.stats.total_touches.load(Ordering::Relaxed),
            hot_pages: hot,
            warm_pages: warm,
            cold_pages: cold,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchCountStatsSnapshot {
    pub total_touches: u64,
    pub hot_pages: u64,
    pub warm_pages: u64,
    pub cold_pages: u64,
}

// Alias for TouchCountStatsSnapshot for compatibility
pub type TouchOptimizerStatsSnapshot = TouchCountStatsSnapshot;

// Cost-aware replacement policy
pub struct CostAwareReplacement {
    // Cost per page (based on load time, etc.)
    page_costs: PRwLock<HashMap<PageId, f64>>,
    // Access frequency
    access_freq: PRwLock<HashMap<PageId, AtomicU64>>,
    // Statistics
    stats: CostAwareStats,
}

#[derive(Debug)]
struct CostAwareStats {
    cost_calculations: AtomicU64,
    evictions: AtomicU64,
}

impl CostAwareReplacement {
    pub fn new() -> Self {
        Self {
            page_costs: PRwLock::new(HashMap::new()),
            access_freq: PRwLock::new(HashMap::new()),
            stats: CostAwareStats {
                cost_calculations: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
            },
        }
    }

    // Set page load cost
    pub fn set_cost(&self, page_id: PageId, cost: f64) {
        let mut costs = self.page_costs.write();
        costs.insert(page_id, cost);
    }

    // Record page access
    pub fn access(&self, page_id: PageId) {
        let freq = self.access_freq.read();
        if let Some(count) = freq.get(&page_id) {
            count.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(freq);
            let mut freq = self.access_freq.write();
            freq.entry(page_id).or_insert_with(|| AtomicU64::new(1));
        }
    }

    // Calculate replacement value (higher = keep, lower = evict)
    pub fn replacement_value(&self, page_id: PageId) -> f64 {
        self.stats.cost_calculations.fetch_add(1, Ordering::Relaxed);

        let costs = self.page_costs.read();
        let freq = self.access_freq.read();

        let cost = costs.get(&page_id).copied().unwrap_or(1.0);
        let frequency = freq.get(&page_id)
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(1) as f64;

        // Value = Cost * Frequency (expensive, frequently accessed pages stay)
        cost * frequency
    }

    // Find victim page (lowest replacement value)
    pub fn find_victim(&self, candidates: &[PageId]) -> Option<PageId> {
        let mut min_value = f64::MAX;
        let mut victim = None;

        for &page_id in candidates {
            let value = self.replacement_value(page_id);
            if value < min_value {
                min_value = value;
                victim = Some(page_id);
            }
        }

        if victim.is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }

        victim
    }

    // Get statistics
    pub fn get_stats(&self) -> CostAwareStatsSnapshot {
        CostAwareStatsSnapshot {
            cost_calculations: self.stats.cost_calculations.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            tracked_pages: self.page_costs.read().len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAwareStatsSnapshot {
    pub cost_calculations: u64,
    pub evictions: u64,
    pub tracked_pages: usize,
}
