// # Stream Multiplexing
//
// Yamux-style stream multiplexing over a single connection. Allows multiple
// logical streams to share a single TCP connection, reducing connection overhead
// and improving efficiency.

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use tokio::sync::{Mutex, mpsc, RwLock};
use tokio::io::{AsyncRead, AsyncWrite};
use std::time::Instant;
use bytes::{Bytes, BytesMut};

/// Stream identifier (unique within a connection)
pub type StreamId = u32;

/// Stream priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamPriority {
    /// Low priority (background tasks, metrics)
    Low = 0,

    /// Normal priority (regular queries)
    Normal = 1,

    /// High priority (interactive queries)
    High = 2,

    /// Critical priority (heartbeats, control messages)
    Critical = 3,
}

impl Default for StreamPriority {
    fn default() -> Self {
        StreamPriority::Normal
    }
}

/// Multiplexed connection supporting multiple concurrent streams
pub struct MultiplexedConnection {
    /// Connection identifier
    connection_id: u64,

    /// Active streams
    streams: Arc<RwLock<HashMap<StreamId, Arc<Stream>>>>,

    /// Next stream ID to allocate
    next_stream_id: Arc<AtomicU32>,

    /// Maximum concurrent streams allowed
    max_streams: usize,

    /// Connection-level flow control window
    flow_control: Arc<FlowControlManager>,

    /// Stream creation counter
    streams_created: Arc<AtomicU64>,

    /// Stream closure counter
    streams_closed: Arc<AtomicU64>,

    /// Connection shutdown flag
    shutdown: Arc<AtomicBool>,

    /// Connection creation time
    created_at: Instant,
}

impl MultiplexedConnection {
    /// Create a new multiplexed connection
    pub fn new(connection_id: u64, max_streams: usize) -> Self {
        Self {
            connection_id,
            streams: Arc::new(RwLock::new(HashMap::new())),
            next_stream_id: Arc::new(AtomicU32::new(1)),
            max_streams,
            flow_control: Arc::new(FlowControlManager::new(max_streams)),
            streams_created: Arc::new(AtomicU64::new(0)),
            streams_closed: Arc::new(AtomicU64::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            created_at: Instant::now(),
        }
    }

    /// Open a new stream with default priority
    pub async fn open_stream(&self) -> Result<Arc<Stream>> {
        self.open_stream_with_priority(StreamPriority::Normal).await
    }

    /// Open a new stream with specified priority
    pub async fn open_stream_with_priority(
        &self,
        priority: StreamPriority,
    ) -> Result<Arc<Stream>> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(DbError::InvalidState(
                "Connection is shutting down".to_string()
            ));
        }

        // Check if we've reached max streams
        let streams = self.streams.read().await;
        if streams.len() >= self.max_streams {
            return Err(DbError::ResourceExhausted(format!(
                "Maximum streams ({}) reached",
                self.max_streams
            )));
        }
        drop(streams);

        // Allocate stream ID (odd IDs for client-initiated streams)
        let stream_id = self.next_stream_id.fetch_add(2, Ordering::Relaxed);

        // Create stream
        let stream = Arc::new(Stream::new(
            stream_id,
            priority,
            Arc::clone(&self.flow_control),
        ));

        // Register stream
        let mut streams = self.streams.write().await;
        streams.insert(stream_id, Arc::clone(&stream));

        self.streams_created.fetch_add(1, Ordering::Relaxed);

        Ok(stream)
    }

    /// Get an existing stream by ID
    pub async fn get_stream(&self, stream_id: StreamId) -> Option<Arc<Stream>> {
        let streams = self.streams.read().await;
        streams.get(&stream_id).map(Arc::clone)
    }

    /// Close a stream
    pub async fn close_stream(&self, stream_id: StreamId) -> Result<()> {
        let mut streams = self.streams.write().await;

        if let Some(stream) = streams.remove(&stream_id) {
            stream.close().await?;
            self.streams_closed.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Get number of active streams
    pub async fn active_stream_count(&self) -> usize {
        let streams = self.streams.read().await;
        streams.len()
    }

    /// Close all streams and shutdown the connection
    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown.store(true, Ordering::Relaxed);

        let mut streams = self.streams.write().await;

        for (_, stream) in streams.drain() {
            let _ = stream.close().await;
        }

        Ok(())
    }

    /// Get connection ID
    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }

    /// Get connection statistics
    pub fn stats(&self) -> MultiplexedConnectionStats {
        MultiplexedConnectionStats {
            connection_id: self.connection_id,
            streams_created: self.streams_created.load(Ordering::Relaxed),
            streams_closed: self.streams_closed.load(Ordering::Relaxed),
            max_streams: self.max_streams,
            uptime_secs: self.created_at.elapsed().as_secs(),
        }
    }
}

/// A logical stream within a multiplexed connection
pub struct Stream {
    /// Stream identifier
    stream_id: StreamId,

    /// Stream priority
    priority: StreamPriority,

    /// Stream state
    state: Arc<RwLock<StreamState>>,

    /// Receive channel for incoming data
    rx: Arc<Mutex<mpsc::UnboundedReceiver<Bytes>>>,

    /// Send channel for outgoing data
    tx: mpsc::UnboundedSender<Bytes>,

    /// Flow control manager
    flow_control: Arc<FlowControlManager>,

    /// Bytes sent on this stream
    bytes_sent: Arc<AtomicU64>,

    /// Bytes received on this stream
    bytes_received: Arc<AtomicU64>,

    /// Stream creation time
    created_at: Instant,
}

impl Stream {
    /// Create a new stream
    fn new(
        stream_id: StreamId,
        priority: StreamPriority,
        flow_control: Arc<FlowControlManager>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            stream_id,
            priority,
            state: Arc::new(RwLock::new(StreamState::Open)),
            rx: Arc::new(Mutex::new(rx)),
            tx,
            flow_control,
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            created_at: Instant::now(),
        }
    }

    /// Get stream ID
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    /// Get stream priority
    pub fn priority(&self) -> StreamPriority {
        self.priority
    }

    /// Send data on the stream
    pub async fn send(&self, data: Bytes) -> Result<()> {
        let state = self.state.read().await;
        if *state != StreamState::Open {
            return Err(DbError::InvalidState(
                format!("Stream {} is not open", self.stream_id)
            ));
        }
        drop(state);

        // Check flow control
        self.flow_control.acquire(data.len(), self.priority).await?;

        // Send data
        let len = data.len() as u64;
        self.tx.send(data).map_err(|_| {
            DbError::Network("Stream send channel closed".to_string())
        })?;

        self.bytes_sent.fetch_add(len, Ordering::Relaxed);

        Ok(())
    }

    /// Receive data from the stream
    pub async fn recv(&self) -> Result<Option<Bytes>> {
        let mut rx = self.rx.lock().await;
        let data = rx.recv().await;

        if let Some(ref bytes) = data {
            self.bytes_received.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        }

        Ok(data)
    }

    /// Close the stream
    pub async fn close(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = StreamState::Closed;

        Ok(())
    }

    /// Check if stream is open
    pub async fn is_open(&self) -> bool {
        let state = self.state.read().await;
        *state == StreamState::Open
    }

    /// Get stream statistics
    pub fn stats(&self) -> StreamStats {
        StreamStats {
            stream_id: self.stream_id,
            priority: self.priority,
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            age_secs: self.created_at.elapsed().as_secs(),
        }
    }
}

/// Stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamState {
    /// Stream is open and can send/receive data
    Open,

    /// Stream is half-closed (local side closed)
    HalfClosed,

    /// Stream is fully closed
    Closed,

    /// Stream reset due to error
    Reset,
}

/// Flow control manager for the connection
pub struct FlowControlManager {
    /// Total window size (bytes)
    total_window: usize,

    /// Available window (bytes)
    available: Arc<Mutex<usize>>,

    /// Per-priority quotas
    priority_quotas: [usize; 4],

    /// Waiters for window space
    waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>>,
}

impl FlowControlManager {
    /// Create a new flow control manager
    fn new(max_streams: usize) -> Self {
        // Default window size: 64KB per stream
        let total_window = max_streams * 64 * 1024;

        Self {
            total_window,
            available: Arc::new(Mutex::new(total_window)),
            priority_quotas: [
                total_window / 10,      // Low: 10%
                total_window * 3 / 10,  // Normal: 30%
                total_window * 4 / 10,  // High: 40%
                total_window / 5,       // Critical: 20%
            ],
            waiters: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Acquire flow control window for sending data
    async fn acquire(&self, size: usize, priority: StreamPriority) -> Result<()> {
        if size > self.total_window {
            return Err(DbError::InvalidInput(format!(
                "Data size {} exceeds total window {}",
                size, self.total_window
            )));
        }

        loop {
            let mut available = self.available.lock().await;

            if *available >= size {
                *available -= size;
                return Ok(());
            }

            // Not enough window space, wait
            let (tx, rx) = tokio::sync::oneshot::channel();
            {
                let mut waiters = self.waiters.lock().await;
                waiters.push(tx);
            }
            drop(available);

            // Wait for window space
            let _ = rx.await;
        }
    }

    /// Release flow control window after data is acknowledged
    async fn release(&self, size: usize) {
        let mut available = self.available.lock().await;
        *available = (*available + size).min(self.total_window);

        // Notify waiters
        let mut waiters = self.waiters.lock().await;
        if let Some(waiter) = waiters.pop() {
            let _ = waiter.send(());
        }
    }

    /// Get current window availability
    async fn available(&self) -> usize {
        let available = self.available.lock().await;
        *available
    }
}

/// Statistics for a multiplexed connection
#[derive(Debug, Clone)]
pub struct MultiplexedConnectionStats {
    pub connection_id: u64,
    pub streams_created: u64,
    pub streams_closed: u64,
    pub max_streams: usize,
    pub uptime_secs: u64,
}

/// Statistics for a single stream
#[derive(Debug, Clone)]
pub struct StreamStats {
    pub stream_id: StreamId,
    pub priority: StreamPriority,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub age_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multiplexed_connection_creation() {
        let conn = MultiplexedConnection::new(1, 100);
        assert_eq!(conn.connection_id(), 1);
        assert_eq!(conn.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_open_stream() {
        let conn = MultiplexedConnection::new(1, 100);

        let stream1 = conn.open_stream().await.unwrap();
        assert_eq!(stream1.stream_id(), 1);

        let stream2 = conn.open_stream().await.unwrap();
        assert_eq!(stream2.stream_id(), 3); // Odd IDs

        assert_eq!(conn.active_stream_count().await, 2);
    }

    #[tokio::test]
    async fn test_stream_max_limit() {
        let conn = MultiplexedConnection::new(1, 2);

        let _stream1 = conn.open_stream().await.unwrap();
        let _stream2 = conn.open_stream().await.unwrap();

        // Should fail - max streams reached
        let result = conn.open_stream().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_priority() {
        let conn = MultiplexedConnection::new(1, 100);

        let stream = conn.open_stream_with_priority(StreamPriority::High).await.unwrap();
        assert_eq!(stream.priority(), StreamPriority::High);
    }

    #[tokio::test]
    async fn test_close_stream() {
        let conn = MultiplexedConnection::new(1, 100);

        let stream = conn.open_stream().await.unwrap();
        let stream_id = stream.stream_id();

        assert_eq!(conn.active_stream_count().await, 1);

        conn.close_stream(stream_id).await.unwrap();
        assert_eq!(conn.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_flow_control() {
        let flow_control = FlowControlManager::new(10); // Small window for testing

        // Should succeed - enough space
        assert!(flow_control.acquire(1024, StreamPriority::Normal).await.is_ok());

        // Release the window
        flow_control.release(1024).await;

        let available = flow_control.available().await;
        assert!(available > 0);
    }
}
