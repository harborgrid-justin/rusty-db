// # Channel-Based Pooling
//
// Channel-based connection pooling using mpsc channels for request queuing.
// Provides fair scheduling, timeout handling, and backpressure management.

use super::PoolConfig;
use crate::common::NodeId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex, Semaphore};

/// Channel-based connection pool
pub struct ChannelPool {
    /// Node identifier
    node_id: NodeId,

    /// Configuration
    config: PoolConfig,

    /// Request channel sender
    request_tx: mpsc::UnboundedSender<ChannelRequest>,

    /// Request counter
    request_counter: Arc<AtomicU64>,

    /// Pool worker handles
    workers: Vec<tokio::task::JoinHandle<()>>,
}

impl ChannelPool {
    /// Create a new channel-based pool
    pub fn new(node_id: NodeId, config: PoolConfig) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();

        let mut pool = Self {
            node_id: node_id.clone(),
            config: config.clone(),
            request_tx,
            request_counter: Arc::new(AtomicU64::new(0)),
            workers: Vec::new(),
        };

        // Start worker tasks
        pool.start_workers(request_rx);

        pool
    }

    /// Submit a request to the pool
    pub async fn submit_request(
        &self,
        request: RequestPayload,
        priority: RequestPriority,
    ) -> Result<ResponsePayload> {
        let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);

        let (response_tx, response_rx) = oneshot::channel();

        let channel_request = ChannelRequest {
            request_id,
            payload: request,
            priority,
            response_tx,
            submitted_at: Instant::now(),
        };

        // Send request to queue
        self.request_tx
            .send(channel_request)
            .map_err(|_| DbError::Internal("Request channel closed".to_string()))?;

        // Wait for response with timeout
        tokio::time::timeout(self.config.acquire_timeout, response_rx)
            .await
            .map_err(|_| {
                DbError::Timeout(format!(
                    "Request {} timed out after {:?}",
                    request_id, self.config.acquire_timeout
                ))
            })?
            .map_err(|_| DbError::Internal("Response channel closed".to_string()))?
    }

    /// Submit request with custom timeout
    pub async fn submit_request_timeout(
        &self,
        request: RequestPayload,
        priority: RequestPriority,
        timeout: Duration,
    ) -> Result<ResponsePayload> {
        let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed);

        let (response_tx, response_rx) = oneshot::channel();

        let channel_request = ChannelRequest {
            request_id,
            payload: request,
            priority,
            response_tx,
            submitted_at: Instant::now(),
        };

        self.request_tx
            .send(channel_request)
            .map_err(|_| DbError::Internal("Request channel closed".to_string()))?;

        tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| {
                DbError::Timeout(format!(
                    "Request {} timed out after {:?}",
                    request_id, timeout
                ))
            })?
            .map_err(|_| DbError::Internal("Response channel closed".to_string()))?
    }

    /// Start worker tasks to process requests
    fn start_workers(&mut self, request_rx: mpsc::UnboundedReceiver<ChannelRequest>) {
        let worker_count = self.config.max_connections;
        let semaphore = Arc::new(Semaphore::new(worker_count));

        // Wrap receiver in Arc<Mutex<...>> to share among workers
        let shared_rx = Arc::new(Mutex::new(request_rx));

        for worker_id in 0..worker_count {
            let semaphore = Arc::clone(&semaphore);
            let node_id = self.node_id.clone();
            let request_rx = Arc::clone(&shared_rx);

            let handle = tokio::spawn(async move {
                loop {
                    // Acquire permit to process request
                    let permit = match semaphore.acquire().await {
                        Ok(p) => p,
                        Err(_) => break, // Semaphore closed
                    };

                    // Wait for next request (lock the receiver to receive)
                    let request = {
                        let mut rx = request_rx.lock().await;
                        match rx.recv().await {
                            Some(r) => r,
                            None => break, // Channel closed
                        }
                    }; // Lock is released here

                    // Process request
                    let response =
                        Self::process_request(worker_id, &node_id, request.payload).await;

                    // Send response
                    let _ = request.response_tx.send(response);

                    // Release permit
                    drop(permit);
                }
            });

            self.workers.push(handle);
        }
    }

    /// Process a single request
    async fn process_request(
        _worker_id: usize,
        _node_id: &NodeId,
        payload: RequestPayload,
    ) -> Result<ResponsePayload> {
        // This is a placeholder - in a real implementation, this would:
        // 1. Get a connection from the pool
        // 2. Execute the request on the connection
        // 3. Return the response
        // 4. Release the connection back to the pool

        // For now, just echo the request
        Ok(ResponsePayload {
            data: payload.data,
            metadata: vec![("processed".to_string(), "true".to_string())],
        })
    }

    /// Shutdown the pool
    pub async fn shutdown(&mut self) -> Result<()> {
        // Close the request channel
        drop(self.request_tx.clone());

        // Wait for all workers to finish
        for handle in self.workers.drain(..) {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Get the node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get pending request count
    pub fn pending_requests(&self) -> usize {
        // This would require additional tracking in a real implementation
        0
    }
}

/// A request submitted to the channel pool
pub struct ChannelRequest {
    /// Unique request identifier
    pub request_id: u64,

    /// Request payload
    pub payload: RequestPayload,

    /// Request priority
    pub priority: RequestPriority,

    /// Channel to send response
    pub response_tx: oneshot::Sender<Result<ResponsePayload>>,

    /// Time when request was submitted
    pub submitted_at: Instant,
}

impl ChannelRequest {
    /// Get wait time since submission
    pub fn wait_time(&self) -> Duration {
        self.submitted_at.elapsed()
    }
}

/// Request priority for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RequestPriority {
    /// Low priority - background tasks
    Low = 0,

    /// Normal priority - regular queries
    Normal = 1,

    /// High priority - interactive queries
    High = 2,

    /// Critical priority - system operations
    Critical = 3,
}

impl Default for RequestPriority {
    fn default() -> Self {
        RequestPriority::Normal
    }
}

/// Request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPayload {
    /// Request data (could be SQL query, command, etc.)
    pub data: Vec<u8>,

    /// Request metadata
    pub metadata: Vec<(String, String)>,
}

impl RequestPayload {
    /// Create a new request payload
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            metadata: Vec::new(),
        }
    }

    /// Add metadata to the request
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.push((key, value));
        self
    }
}

/// Response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePayload {
    /// Response data
    pub data: Vec<u8>,

    /// Response metadata
    pub metadata: Vec<(String, String)>,
}

impl ResponsePayload {
    /// Create a new response payload
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            metadata: Vec::new(),
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.push((key, value));
        self
    }
}

/// Request channel for submitting requests
pub struct RequestChannel {
    /// Channel pool reference
    pool: Arc<ChannelPool>,

    /// Default priority for requests
    default_priority: RequestPriority,

    /// Default timeout
    default_timeout: Duration,
}

impl RequestChannel {
    /// Create a new request channel
    pub fn new(pool: Arc<ChannelPool>) -> Self {
        Self {
            pool,
            default_priority: RequestPriority::Normal,
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Set default priority
    pub fn with_priority(mut self, priority: RequestPriority) -> Self {
        self.default_priority = priority;
        self
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Submit a request using defaults
    pub async fn submit(&self, payload: RequestPayload) -> Result<ResponsePayload> {
        self.pool
            .submit_request_timeout(payload, self.default_priority, self.default_timeout)
            .await
    }

    /// Submit a request with custom priority
    pub async fn submit_with_priority(
        &self,
        payload: RequestPayload,
        priority: RequestPriority,
    ) -> Result<ResponsePayload> {
        self.pool
            .submit_request_timeout(payload, priority, self.default_timeout)
            .await
    }

    /// Submit a request with custom timeout
    pub async fn submit_with_timeout(
        &self,
        payload: RequestPayload,
        timeout: Duration,
    ) -> Result<ResponsePayload> {
        self.pool
            .submit_request_timeout(payload, self.default_priority, timeout)
            .await
    }
}

/// Fair scheduler for request processing
pub struct FairScheduler {
    /// Priority queues
    queues: [mpsc::UnboundedSender<ChannelRequest>; 4],

    /// Request distribution counters
    counters: [AtomicU64; 4],
}

impl FairScheduler {
    /// Create a new fair scheduler
    pub fn new() -> (Self, [mpsc::UnboundedReceiver<ChannelRequest>; 4]) {
        let (tx0, rx0) = mpsc::unbounded_channel();
        let (tx1, rx1) = mpsc::unbounded_channel();
        let (tx2, rx2) = mpsc::unbounded_channel();
        let (tx3, rx3) = mpsc::unbounded_channel();

        let scheduler = Self {
            queues: [tx0, tx1, tx2, tx3],
            counters: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        };

        (scheduler, [rx0, rx1, rx2, rx3])
    }

    /// Enqueue a request
    pub fn enqueue(&self, request: ChannelRequest) -> Result<()> {
        let priority_idx = request.priority as usize;

        self.queues[priority_idx]
            .send(request)
            .map_err(|_| DbError::Internal("Priority queue closed".to_string()))?;

        self.counters[priority_idx].fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get queue statistics
    pub fn stats(&self) -> [u64; 4] {
        [
            self.counters[0].load(Ordering::Relaxed),
            self.counters[1].load(Ordering::Relaxed),
            self.counters[2].load(Ordering::Relaxed),
            self.counters[3].load(Ordering::Relaxed),
        ]
    }
}

impl Default for FairScheduler {
    fn default() -> Self {
        Self::new().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_priority_ordering() {
        assert!(RequestPriority::Critical > RequestPriority::High);
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
    }

    #[test]
    fn test_request_payload_creation() {
        let payload = RequestPayload::new(vec![1, 2, 3])
            .with_metadata("key1".to_string(), "value1".to_string())
            .with_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(payload.data.len(), 3);
        assert_eq!(payload.metadata.len(), 2);
    }

    #[tokio::test]
    async fn test_channel_pool_creation() {
        let config = PoolConfig::default();
        let pool = ChannelPool::new("test-node".to_string(), config);

        assert_eq!(pool.node_id(), "test-node");
    }

    #[tokio::test]
    async fn test_fair_scheduler() {
        let (scheduler, _receivers) = FairScheduler::new();

        let request = ChannelRequest {
            request_id: 1,
            payload: RequestPayload::new(vec![1, 2, 3]),
            priority: RequestPriority::High,
            response_tx: oneshot::channel().0,
            submitted_at: Instant::now(),
        };

        assert!(scheduler.enqueue(request).is_ok());

        let stats = scheduler.stats();
        assert_eq!(stats[RequestPriority::High as usize], 1);
    }
}
