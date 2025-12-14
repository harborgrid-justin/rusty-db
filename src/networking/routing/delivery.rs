// Message delivery guarantees
//
// This module implements different delivery semantics including at-most-once,
// at-least-once, and exactly-once delivery with idempotency support.

use crate::error::Result;
use crate::networking::routing::serialization::RequestId;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Delivery guarantee level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    /// At-most-once: Send and forget, no retries
    AtMostOnce,
    /// At-least-once: Retry until acknowledged
    AtLeastOnce,
    /// Exactly-once: Use idempotency keys to prevent duplicates
    ExactlyOnce,
}

/// Idempotency key for exactly-once delivery
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    /// Create a new idempotency key
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Generate from request ID
    pub fn from_request_id(request_id: &RequestId) -> Self {
        Self(format!("req-{}", request_id.0))
    }
}

/// Delivery tracker for managing message delivery guarantees
pub struct DeliveryTracker {
    /// Inner state
    inner: Arc<RwLock<DeliveryTrackerInner>>,
    /// Default delivery guarantee
    default_guarantee: DeliveryGuarantee,
    /// Idempotency window duration
    idempotency_window: Duration,
}

struct DeliveryTrackerInner {
    /// Pending messages awaiting acknowledgment (for at-least-once)
    pending_messages: HashMap<RequestId, PendingMessage>,

    /// Processed idempotency keys (for exactly-once)
    processed_keys: HashMap<IdempotencyKey, ProcessedMessage>,

    /// Set of recently seen request IDs for deduplication
    seen_requests: HashSet<RequestId>,
}

#[derive(Debug, Clone)]
struct PendingMessage {
    /// When the message was first sent
    first_sent_at: SystemTime,
    /// When the message was last sent
    last_sent_at: SystemTime,
    /// Number of send attempts
    attempt_count: u32,
    /// Maximum number of retries
    max_retries: u32,
    /// Timeout for this message
    timeout: Duration,
}

#[derive(Debug, Clone)]
struct ProcessedMessage {
    /// When the message was processed
    processed_at: SystemTime,
    /// Response (if any)
    response: Option<Vec<u8>>,
}

impl DeliveryTracker {
    /// Create a new delivery tracker
    pub fn new(default_guarantee: DeliveryGuarantee) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DeliveryTrackerInner {
                pending_messages: HashMap::new(),
                processed_keys: HashMap::new(),
                seen_requests: HashSet::new(),
            })),
            default_guarantee,
            idempotency_window: Duration::from_secs(3600), // 1 hour default
        }
    }

    /// Set the idempotency window duration
    pub fn with_idempotency_window(mut self, window: Duration) -> Self {
        self.idempotency_window = window;
        self
    }

    /// Register a message before sending
    pub fn register_message(
        &self,
        request_id: RequestId,
        guarantee: Option<DeliveryGuarantee>,
        max_retries: u32,
        timeout: Duration,
    ) -> Result<()> {
        let guarantee = guarantee.unwrap_or(self.default_guarantee);

        match guarantee {
            DeliveryGuarantee::AtMostOnce => {
                // No tracking needed for at-most-once
                Ok(())
            }
            DeliveryGuarantee::AtLeastOnce | DeliveryGuarantee::ExactlyOnce => {
                let mut inner = self.inner.write();
                let now = SystemTime::now();

                inner.pending_messages.insert(
                    request_id,
                    PendingMessage {
                        first_sent_at: now,
                        last_sent_at: now,
                        attempt_count: 1,
                        max_retries,
                        timeout,
                    },
                );

                Ok(())
            }
        }
    }

    /// Check if a message should be retried
    pub fn should_retry(&self, request_id: &RequestId) -> bool {
        let inner = self.inner.read();

        if let Some(pending) = inner.pending_messages.get(request_id) {
            let now = SystemTime::now();
            let elapsed = now
                .duration_since(pending.first_sent_at)
                .unwrap_or(Duration::ZERO);

            // Check if timeout exceeded
            if elapsed > pending.timeout {
                return false;
            }

            // Check if max retries exceeded
            if pending.attempt_count >= pending.max_retries {
                return false;
            }

            true
        } else {
            false
        }
    }

    /// Record a retry attempt
    pub fn record_retry(&self, request_id: &RequestId) {
        let mut inner = self.inner.write();

        if let Some(pending) = inner.pending_messages.get_mut(request_id) {
            pending.last_sent_at = SystemTime::now();
            pending.attempt_count += 1;
        }
    }

    /// Mark a message as acknowledged
    pub fn mark_acknowledged(&self, request_id: &RequestId) {
        let mut inner = self.inner.write();
        inner.pending_messages.remove(request_id);
        inner.seen_requests.insert(request_id.clone());
    }

    /// Check if a request has been seen before (for deduplication)
    pub fn is_duplicate(&self, request_id: &RequestId) -> bool {
        let inner = self.inner.read();
        inner.seen_requests.contains(request_id)
    }

    /// Check if an idempotency key has been processed
    pub fn is_idempotency_key_processed(&self, key: &IdempotencyKey) -> Option<Vec<u8>> {
        let inner = self.inner.read();

        if let Some(processed) = inner.processed_keys.get(key) {
            // Check if still within idempotency window
            let now = SystemTime::now();
            if let Ok(elapsed) = now.duration_since(processed.processed_at) {
                if elapsed <= self.idempotency_window {
                    return processed.response.clone();
                }
            }
        }

        None
    }

    /// Mark an idempotency key as processed
    pub fn mark_idempotency_key_processed(&self, key: IdempotencyKey, response: Option<Vec<u8>>) {
        let mut inner = self.inner.write();

        inner.processed_keys.insert(
            key,
            ProcessedMessage {
                processed_at: SystemTime::now(),
                response,
            },
        );
    }

    /// Get all pending messages that need retry
    pub fn get_pending_retries(&self) -> Vec<RequestId> {
        let inner = self.inner.read();
        let now = SystemTime::now();

        inner
            .pending_messages
            .iter()
            .filter_map(|(request_id, pending)| {
                // Check if enough time has passed since last attempt
                let elapsed = now
                    .duration_since(pending.last_sent_at)
                    .unwrap_or(Duration::ZERO);

                // Retry after exponential backoff
                let backoff = Duration::from_millis(100 * 2u64.pow(pending.attempt_count));

                if elapsed >= backoff && pending.attempt_count < pending.max_retries {
                    Some(request_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        let mut inner = self.inner.write();
        let now = SystemTime::now();

        // Remove expired pending messages
        inner.pending_messages.retain(|_, pending| {
            let elapsed = now
                .duration_since(pending.first_sent_at)
                .unwrap_or(Duration::ZERO);
            elapsed <= pending.timeout
        });

        // Remove expired idempotency keys
        inner.processed_keys.retain(|_, processed| {
            let elapsed = now
                .duration_since(processed.processed_at)
                .unwrap_or(Duration::ZERO);
            elapsed <= self.idempotency_window
        });

        // Limit seen_requests size (keep only recent ones)
        if inner.seen_requests.len() > 10000 {
            inner.seen_requests.clear();
        }
    }

    /// Get statistics about delivery tracking
    pub fn get_stats(&self) -> DeliveryStats {
        let inner = self.inner.read();

        DeliveryStats {
            pending_messages: inner.pending_messages.len(),
            processed_keys: inner.processed_keys.len(),
            seen_requests: inner.seen_requests.len(),
        }
    }
}

/// Statistics about delivery tracking
#[derive(Debug, Clone)]
pub struct DeliveryStats {
    pub pending_messages: usize,
    pub processed_keys: usize,
    pub seen_requests: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_most_once() {
        let tracker = DeliveryTracker::new(DeliveryGuarantee::AtMostOnce);
        let request_id = RequestId::new();

        tracker
            .register_message(request_id.clone(), None, 3, Duration::from_secs(10))
            .unwrap();

        // At-most-once should not track retries
        assert!(!tracker.should_retry(&request_id));
    }

    #[test]
    fn test_at_least_once() {
        let tracker = DeliveryTracker::new(DeliveryGuarantee::AtLeastOnce);
        let request_id = RequestId::new();

        tracker
            .register_message(
                request_id.clone(),
                Some(DeliveryGuarantee::AtLeastOnce),
                3,
                Duration::from_secs(10),
            )
            .unwrap();

        assert!(tracker.should_retry(&request_id));

        tracker.mark_acknowledged(&request_id);
        assert!(!tracker.should_retry(&request_id));
    }

    #[test]
    fn test_exactly_once() {
        let tracker = DeliveryTracker::new(DeliveryGuarantee::ExactlyOnce);
        let key = IdempotencyKey::new("test-key");

        assert!(tracker.is_idempotency_key_processed(&key).is_none());

        let response = vec![1, 2, 3, 4];
        tracker.mark_idempotency_key_processed(key.clone(), Some(response.clone()));

        assert_eq!(tracker.is_idempotency_key_processed(&key), Some(response));
    }

    #[test]
    fn test_duplicate_detection() {
        let tracker = DeliveryTracker::new(DeliveryGuarantee::AtLeastOnce);
        let request_id = RequestId::new();

        assert!(!tracker.is_duplicate(&request_id));

        tracker.mark_acknowledged(&request_id);
        assert!(tracker.is_duplicate(&request_id));
    }
}
