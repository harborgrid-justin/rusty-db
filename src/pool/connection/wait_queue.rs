// Wait queue management module
//
// This module provides wait queue functionality for connection requests including:
// - Fair and priority-based queuing
// - Deadlock detection
// - Starvation prevention
// - Queue statistics

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Instant, Duration};
use parking_lot::Condvar;
use serde::{Serialize, Deserialize};

use super::core::PoolError;

// Wait queue for connection requests
pub struct WaitQueue {
    // Queue entries
    entries: Mutex<VecDeque<WaitEntry>>,

    // Condition variable for notifications
    condvar: Condvar,

    // Maximum queue size
    max_size: usize,

    // Fair queue mode
    fair_mode: bool,

    // Next waiter ID
    next_id: AtomicU64,

    // Queue statistics
    stats: WaitQueueStats,
}

// Wait queue entry
struct WaitEntry {
    id: u64,
    enqueued_at: Instant,
    priority: QueuePriority,
    notified: Arc<AtomicBool>,
}

// Queue priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for QueuePriority {
    fn default() -> Self {
        QueuePriority::Normal
    }
}

// Wait queue statistics
#[derive(Default)]
struct WaitQueueStats {
    total_enqueued: AtomicU64,
    total_dequeued: AtomicU64,
    total_timeouts: AtomicU64,
    max_wait_time: AtomicU64, // in microseconds
    total_wait_time: AtomicU64, // in microseconds
}

impl WaitQueue {
    // Create a new wait queue
    pub fn new(max_size: usize, fair_mode: bool) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            max_size,
            fair_mode,
            next_id: AtomicU64::new(1),
            stats: WaitQueueStats::default(),
        }
    }

    // Enqueue a waiter
    pub async fn enqueue(&self) -> Result<(), PoolError> {
        self.enqueue_with_priority(QueuePriority::Normal).await
    }

    // Enqueue with priority
    pub async fn enqueue_with_priority(&self, priority: QueuePriority) -> Result<(), PoolError> {
        let mut queue = self.entries.lock().unwrap();

        if queue.len() >= self.max_size {
            return Err(PoolError::WaitQueueFull {
                current: queue.len(),
                max: self.max_size,
            });
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let entry = WaitEntry {
            id,
            enqueued_at: Instant::now(),
            priority,
            notified: Arc::new(AtomicBool::new(false)),
        };

        if self.fair_mode {
            // FIFO - add to back
            queue.push_back(entry);
        } else {
            // Priority-based - insert based on priority
            let insert_pos = queue.iter().position(|e| e.priority < priority)
                .unwrap_or(queue.len());
            queue.insert(insert_pos, entry);
        }

        self.stats.total_enqueued.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    // Notify next waiter
    pub fn notify_one(&self) {
        let mut queue = self.entries.lock().unwrap();

        if let Some(entry) = queue.pop_front() {
            let wait_time = entry.enqueued_at.elapsed();
            self.record_wait_time(wait_time);

            entry.notified.store(true, Ordering::SeqCst);
            self.stats.total_dequeued.fetch_add(1, Ordering::SeqCst);

            self.condvar.notify_one();
        }
    }

    // Notify all waiters
    pub fn notify_all(&self) {
        let mut queue = self.entries.lock().unwrap();

        while let Some(entry) = queue.pop_front() {
            let wait_time = entry.enqueued_at.elapsed();
            self.record_wait_time(wait_time);

            entry.notified.store(true, Ordering::SeqCst);
            self.stats.total_dequeued.fetch_add(1, Ordering::SeqCst);
        }

        self.condvar.notify_all();
    }

    // Get queue position for a waiter
    pub fn queue_position(&self, waiter_id: u64) -> Option<usize> {
        let queue = self.entries.lock().unwrap();
        queue.iter().position(|e| e.id == waiter_id)
    }

    // Get current queue length
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    // Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }

    // Record wait time
    fn record_wait_time(&self, duration: Duration) {
        let micros = duration.as_micros() as u64;
        self.stats.total_wait_time.fetch_add(micros, Ordering::SeqCst);

        // Update max wait time
        let mut current_max = self.stats.max_wait_time.load(Ordering::SeqCst);
        while micros > current_max {
            match self.stats.max_wait_time.compare_exchange(
                current_max,
                micros,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    // Get statistics
    pub fn statistics(&self) -> QueueStats {
        let total_enqueued = self.stats.total_enqueued.load(Ordering::SeqCst);
        let total_dequeued = self.stats.total_dequeued.load(Ordering::SeqCst);
        let total_wait_time = self.stats.total_wait_time.load(Ordering::SeqCst);
        let max_wait_time = self.stats.max_wait_time.load(Ordering::SeqCst);

        QueueStats {
            current_size: self.len(),
            total_enqueued,
            total_dequeued,
            total_timeouts: self.stats.total_timeouts.load(Ordering::SeqCst),
            average_wait_time: if total_dequeued > 0 {
                Duration::from_micros(total_wait_time / total_dequeued)
            } else {
                Duration::ZERO
            },
            max_wait_time: Duration::from_micros(max_wait_time),
        }
    }
}

// Queue statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub current_size: usize,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_timeouts: u64,
    pub average_wait_time: Duration,
    pub max_wait_time: Duration,
}

// Deadlock detector for wait queue
pub struct DeadlockDetector {
    // Detection enabled
    enabled: bool,

    // Detection interval
    _check_interval: Duration,

    // Deadlock threshold (how long to wait before considering deadlock)
    deadlock_threshold: Duration,

    // Detected deadlocks
    deadlocks_detected: AtomicU64,
}

impl DeadlockDetector {
    // Create a new deadlock detector
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            _check_interval: Duration::from_secs(10),
            deadlock_threshold: Duration::from_secs(60),
            deadlocks_detected: AtomicU64::new(0),
        }
    }

    // Check for deadlocks in the wait queue
    pub fn check_deadlock(&self, queue: &WaitQueue) -> bool {
        if !self.enabled {
            return false;
        }

        let entries = queue.entries.lock().unwrap();

        // Simple heuristic: if oldest waiter has been waiting too long
        if let Some(oldest) = entries.front() {
            if oldest.enqueued_at.elapsed() > self.deadlock_threshold {
                self.deadlocks_detected.fetch_add(1, Ordering::SeqCst);
                tracing::warn!("Potential deadlock detected: waiter {} waiting for {:?}",
                             oldest.id, oldest.enqueued_at.elapsed());
                return true;
            }
        }

        false
    }

    // Get statistics
    pub fn statistics(&self) -> DeadlockStats {
        DeadlockStats {
            deadlocks_detected: self.deadlocks_detected.load(Ordering::SeqCst),
        }
    }
}

// Deadlock statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlockStats {
    pub deadlocks_detected: u64,
}

// Starvation prevention system
pub struct StarvationPrevention {
    // Maximum wait time before boosting priority
    max_wait_time: Duration,

    // Priority boost increment
    _priority_boost: u32,

    // Boosted waiters
    boosted_count: AtomicU64,
}

impl StarvationPrevention {
    // Create a new starvation prevention system
    pub fn new(max_wait_time: Duration) -> Self {
        Self {
            max_wait_time,
            _priority_boost: 1,
            boosted_count: AtomicU64::new(0),
        }
    }

    // Check for starvation and boost priority if needed
    pub fn check_and_boost(&self, queue: &WaitQueue) {
        let mut entries = queue.entries.lock().unwrap();

        for entry in entries.iter_mut() {
            if entry.enqueued_at.elapsed() > self.max_wait_time {
                // Boost priority (simple increment for now)
                if entry.priority < QueuePriority::Critical {
                    entry.priority = match entry.priority {
                        QueuePriority::Low => QueuePriority::Normal,
                        QueuePriority::Normal => QueuePriority::High,
                        QueuePriority::High => QueuePriority::Critical,
                        QueuePriority::Critical => QueuePriority::Critical,
                    };

                    self.boosted_count.fetch_add(1, Ordering::SeqCst);

                    tracing::info!("Boosted priority for waiter {} after {:?}",
                                 entry.id, entry.enqueued_at.elapsed());
                }
            }
        }

        // Re-sort if not in fair mode
        if !queue.fair_mode {
            // Convert to Vec, sort, convert back
            let mut vec: Vec<_> = entries.drain(..).collect();
            vec.sort_by(|a, b| b.priority.cmp(&a.priority));
            entries.extend(vec);
        }
    }

    // Get statistics
    pub fn statistics(&self) -> StarvationStats {
        StarvationStats {
            boosted_count: self.boosted_count.load(Ordering::SeqCst),
        }
    }
}

// Starvation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarvationStats {
    pub boosted_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_queue() {
        let queue = WaitQueue::new(100, true);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }
}
