// Buffer Management Module
//
// Network buffer pooling, scatter-gather I/O, and large object handling
//
// TODO: CONSOLIDATION NEEDED - BufferPool Implementation #4 of 4
// This is one of 4 separate BufferPool implementations in the codebase:
//   1. src/buffer/manager.rs - Page-based buffer pool (4KB pages) for database
//   2. src/memory/buffer_pool/ - General-purpose memory pooling
//   3. src/io/buffer_pool.rs - Async I/O buffer management
//   4. src/network/advanced_protocol/buffer_management.rs - THIS FILE (8KB network buffers)
//
// RECOMMENDATION: Consolidate to 2 pools:
//   - Keep src/buffer/manager.rs for database page caching (4KB pages)
//   - Merge this with src/memory/buffer_pool/ for general-purpose buffering
//     with configurable size classes (8KB, 64KB, 1MB, etc.)
//
// See: diagrams/06_network_api_flow.md - Issue #4.1
//
// ============================================================================
// Buffer Pool
// ============================================================================

#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    pub buffer_size: usize,
    pub max_buffers: usize,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,      // 8 KB buffers
            max_buffers: 1000,      // Max 1000 buffers (8 MB total)
        }
    }
}

pub struct BufferPool {
    config: BufferPoolConfig,
    available: Vec<Vec<u8>>,
}

impl BufferPool {
    pub fn new(config: BufferPoolConfig) -> Self {
        Self {
            config,
            available: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> Vec<u8> {
        self.available
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.config.buffer_size))
    }

    pub fn deallocate(&mut self, mut buffer: Vec<u8>) {
        if self.available.len() < self.config.max_buffers {
            buffer.clear();
            self.available.push(buffer);
        }
        // If pool is full, just drop the buffer
    }

    pub fn available_buffers(&self) -> usize {
        self.available.len()
    }

    pub fn config(&self) -> &BufferPoolConfig {
        &self.config
    }
}

#[derive(Debug, Clone)]
pub struct BufferPoolMetrics {
    pub buffers_allocated: u64,
    pub buffers_freed: u64,
}

impl Default for BufferPoolMetrics {
    fn default() -> Self {
        Self {
            buffers_allocated: 0,
            buffers_freed: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub total_buffers: usize,
    pub free_buffers: usize,
}

// ============================================================================
// Scatter-Gather I/O
// ============================================================================

pub struct ScatterGatherBuffer {
    segments: Vec<Vec<u8>>,
}

impl ScatterGatherBuffer {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn add_segment(&mut self, data: Vec<u8>) {
        self.segments.push(data);
    }

    pub fn segments(&self) -> &[Vec<u8>] {
        &self.segments
    }

    pub fn total_size(&self) -> usize {
        self.segments.iter().map(|s| s.len()).sum()
    }

    pub fn coalesce(&self) -> Vec<u8> {
        let total_size = self.total_size();
        let mut result = Vec::with_capacity(total_size);
        for segment in &self.segments {
            result.extend_from_slice(segment);
        }
        result
    }
}

impl Default for ScatterGatherBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Coalescing Buffer
// ============================================================================

pub struct CoalescingBuffer {
    buffer: Vec<u8>,
    threshold: usize,
}

impl CoalescingBuffer {
    pub fn new() -> Self {
        Self::with_threshold(4096)
    }

    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            buffer: Vec::new(),
            threshold,
        }
    }

    pub fn append(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn should_flush(&self) -> bool {
        self.buffer.len() >= self.threshold
    }

    pub fn flush(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for CoalescingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Large Object Streaming
// ============================================================================

pub struct LargeObjectStream {
    chunks: Vec<Vec<u8>>,
    chunk_size: usize,
}

impl LargeObjectStream {
    pub fn new() -> Self {
        Self::with_chunk_size(1024 * 1024) // 1 MB chunks
    }

    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self {
            chunks: Vec::new(),
            chunk_size,
        }
    }

    pub fn add_chunk(&mut self, chunk: Vec<u8>) {
        self.chunks.push(chunk);
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn total_size(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }

    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }
}

impl Default for LargeObjectStream {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Memory-Mapped Transfer
// ============================================================================

pub struct MemoryMappedTransfer;

impl MemoryMappedTransfer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MemoryMappedTransfer {
    fn default() -> Self {
        Self::new()
    }
}
