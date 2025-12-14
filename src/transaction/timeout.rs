// Transaction timeout management.
//
// This module provides timeout tracking for transactions to prevent
// resource leaks from long-running or abandoned transactions.
//
// # Example
//
// ```rust,ignore
// let tm = TimeoutManager::new(Duration::from_secs(60));
// tm.set_timeout(txn_id::from_secs(30));
// if tm.is_timed_out(txn_id) {
//     // Abort the transaction
// }
// ```

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use parking_lot::RwLock;

use crate::common::TransactionId;

/// Timeout manager for transactions.
///
/// Tracks deadlines for transactions and provides methods
/// to check for and handle timeouts.
pub struct TimeoutManager {
    /// Map of transaction ID -> deadline.
    timeouts: Arc<RwLock<HashMap<TransactionId, TimeoutEntry>>>,
    /// Default timeout for new transactions.
    default_timeout: Duration,
}

/// Entry in the timeout table.
#[derive(Debug, Clone)]
struct TimeoutEntry {
    /// When the timeout was set.
    #[allow(dead_code)]
    start_time: SystemTime,
    /// Deadline for the transaction.
    deadline: SystemTime,
    /// The timeout duration.
    duration: Duration,
}

impl TimeoutManager {
    /// Creates a new timeout manager with the specified default timeout.
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            timeouts: Arc::new(RwLock::new(HashMap::new())),
            default_timeout,
        }
    }

    /// Sets a timeout for a transaction.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - The transaction ID.
    /// * `timeout` - The timeout duration.
    pub fn set_timeout(&self, txn_id: TransactionId, timeout: Duration) {
        let now = SystemTime::now();
        let entry = TimeoutEntry {
            start_time: now,
            deadline: now + timeout,
            duration: timeout,
        };
        self.timeouts.write().insert(txn_id, entry);
    }

    /// Sets the default timeout for a transaction.
    pub fn set_default_timeout(&self, txn_id: TransactionId) {
        self.set_timeout(txn_id, self.default_timeout);
    }

    /// Checks if a transaction has timed out.
    pub fn is_timed_out(&self, txn_id: TransactionId) -> bool {
        let timeouts = self.timeouts.read();
        if let Some(entry) = timeouts.get(&txn_id) {
            SystemTime::now() > entry.deadline
        } else {
            false
        }
    }

    /// Gets the remaining time before timeout, if any.
    pub fn remaining_time(&self, txn_id: TransactionId) -> Option<Duration> {
        let timeouts = self.timeouts.read();
        if let Some(entry) = timeouts.get(&txn_id) {
            let now = SystemTime::now();
            if now < entry.deadline {
                entry.deadline.duration_since(now).ok()
            } else {
                Some(Duration::ZERO)
            }
        } else {
            None
        }
    }

    /// Resets the timeout for a transaction (extends deadline).
    pub fn reset_timeout(&self, txn_id: TransactionId) {
        let mut timeouts = self.timeouts.write();
        if let Some(entry) = timeouts.get_mut(&txn_id) {
            let now = SystemTime::now();
            entry.deadline = now + entry.duration;
        }
    }

    /// Clears the timeout for a transaction.
    pub fn clear_timeout(&self, txn_id: TransactionId) {
        self.timeouts.write().remove(&txn_id);
    }

    /// Gets all timed-out transactions.
    ///
    /// # Returns
    ///
    /// Vector of transaction IDs that have exceeded their deadline.
    pub fn get_timed_out_transactions(&self) -> Vec<TransactionId> {
        let now = SystemTime::now();
        let timeouts = self.timeouts.read();

        timeouts
            .iter()
            .filter(|(_, entry)| now > entry.deadline)
            .map(|(&txn_id, _)| txn_id)
            .collect()
    }

    /// Gets the number of tracked transactions.
    pub fn tracked_count(&self) -> usize {
        self.timeouts.read().len()
    }

    /// Gets the default timeout duration.
    pub fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    /// Clears all timeout entries.
    pub fn clear_all(&self) {
        self.timeouts.write().clear();
    }
}

impl Default for TimeoutManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

impl fmt::Debug for TimeoutManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimeoutManager")
            .field("default_timeout", &self.default_timeout)
            .field("tracked_count", &self.tracked_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_set_and_check_timeout() {
        let tm = TimeoutManager::new(Duration::from_secs(60));

        tm.set_timeout(1, Duration::from_secs(60));
        assert!(!tm.is_timed_out(1));

        // Non-existent transaction is not timed out
        assert!(!tm.is_timed_out(999));
    }

    #[test]
    fn test_immediate_timeout() {
        let tm = TimeoutManager::new(Duration::from_millis(1));

        tm.set_timeout(1, Duration::from_millis(1));

        // Wait a bit
        thread::sleep(Duration::from_millis(10));

        assert!(tm.is_timed_out(1));
    }

    #[test]
    fn test_get_timed_out_transactions() {
        let tm = TimeoutManager::new(Duration::from_millis(1));

        tm.set_timeout(1, Duration::from_millis(1));
        tm.set_timeout(2, Duration::from_secs(60));

        thread::sleep(Duration::from_millis(10));

        let timed_out = tm.get_timed_out_transactions();
        assert!(timed_out.contains(&1));
        assert!(!timed_out.contains(&2));
    }

    #[test]
    fn test_clear_timeout() {
        let tm = TimeoutManager::new(Duration::from_secs(60));

        tm.set_timeout(1, Duration::from_secs(60));
        assert_eq!(tm.tracked_count(), 1);

        tm.clear_timeout(1);
        assert_eq!(tm.tracked_count(), 0);
    }

    #[test]
    fn test_remaining_time() {
        let tm = TimeoutManager::new(Duration::from_secs(60));

        tm.set_timeout(1, Duration::from_secs(60));

        let remaining = tm.remaining_time(1);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > Duration::from_secs(50));
    }
}
