//\! Arena Allocator Implementation
//\!
//\! Bump allocation for per-query memory contexts with hierarchical support.

use super::common::*;

// Memory arena for bump allocation
struct Arena {
    // Current chunk
    current_chunk: Option<ArenaChunk>,
    // Previous chunks
    chunks: Vec<ArenaChunk>,
    // Total allocated in this arena
    total_allocated: usize,
    // Allocation limit (0 = unlimited)
    limit: usize,
    // Arena name
    name: String,
}

// Arena chunk
struct ArenaChunk {
    // Base pointer
    base: NonNull<u8>,
    // Chunk size
    size: usize,
    // Current offset
    offset: usize,
}

impl ArenaChunk {
    // Create a new arena chunk
    unsafe fn new(size: usize) -> Result<Self> {
        let layout = Layout::from_size_align(size, 16)
            .map_err(|e| DbError::OutOfMemory(format!("Invalid arena layout: {}", e)))?;

        let base = System.alloc(layout);
        if base.is_null() {
            return Err(DbError::OutOfMemory(
                "Failed to allocate arena chunk".to_string(),
            ));
        }

        Ok(Self {
            base: NonNull::new_unchecked(base),
            size,
            offset: 0,
        })
    }

    // Allocate from this chunk
    unsafe fn allocate(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let offset = (self.offset + align - 1) & !(align - 1);
        let end = offset + size;

        if end <= self.size {
            let ptr = NonNull::new_unchecked(self.base.as_ptr().add(offset));
            self.offset = end;
            Some(ptr)
        } else {
            None
        }
    }

    /// Get number of bytes used in arena (for monitoring)
    #[allow(dead_code)]
    fn bytes_used(&self) -> usize {
        self.offset
    }

    /// Get number of bytes free in arena (for monitoring)
    #[allow(dead_code)]
    fn bytes_free(&self) -> usize {
        self.size - self.offset
    }
}

impl Drop for ArenaChunk {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, 16);
            System.dealloc(self.base.as_ptr(), layout);
        }
    }
}

impl Arena {
    fn new(name: String, initial_size: usize, limit: usize) -> Result<Self> {
        Ok(Self {
            current_chunk: Some(unsafe { ArenaChunk::new(initial_size)? }),
            chunks: Vec::new(),
            total_allocated: 0,
            limit,
            name,
        })
    }

    unsafe fn allocate(&mut self, size: usize, align: usize) -> Result<NonNull<u8>> {
        // Check limit
        if self.limit > 0 && self.total_allocated + size > self.limit {
            return Err(DbError::LimitExceeded(format!(
                "Arena '{}' limit {} exceeded",
                self.name, self.limit
            )));
        }

        // Try current chunk
        if let Some(ref mut chunk) = self.current_chunk {
            if let Some(ptr) = chunk.allocate(size, align) {
                self.total_allocated += size;
                return Ok(ptr);
            }
        }

        // Need new chunk
        let chunk_size = (size.max(64 * 1024) + 4095) & !4095; // Round up to 4KB
        let old_chunk = self.current_chunk.take();
        if let Some(chunk) = old_chunk {
            self.chunks.push(chunk);
        }

        let mut new_chunk = ArenaChunk::new(chunk_size)?;
        let ptr = new_chunk
            .allocate(size, align)
            .ok_or_else(|| DbError::OutOfMemory("Failed to allocate from new chunk".to_string()))?;

        self.current_chunk = Some(new_chunk);
        self.total_allocated += size;

        Ok(ptr)
    }

    fn reset(&mut self) {
        self.chunks.clear();
        if let Some(ref mut chunk) = self.current_chunk {
            chunk.offset = 0;
        }
        self.total_allocated = 0;
    }

    fn bytes_allocated(&self) -> usize {
        self.total_allocated
    }
}

// Memory context (hierarchical arena)
pub struct MemoryContext {
    // Context ID
    id: u64,
    // Context type
    context_type: ContextType,
    // Arena for this context
    arena: Arena,
    // Parent context
    _parent: Option<Weak<Mutex<MemoryContext>>>,
    // Child contexts
    children: Vec<Arc<Mutex<MemoryContext>>>,
    // Creation time
    created_at: Instant,
    // Statistics
    stats: ContextStats,
}

// Memory context statistics
struct ContextStats {
    allocations: AtomicU64,
    _deallocations: AtomicU64,
    resets: AtomicU64,
    peak_usage: AtomicUsize,
}

impl ContextStats {
    fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            _deallocations: AtomicU64::new(0),
            resets: AtomicU64::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }
}

impl MemoryContext {
    // Create a new top-level memory context
    pub fn new_top_level(name: String, limit: usize) -> Result<Arc<Mutex<Self>>> {
        static CONTEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = CONTEXT_ID.fetch_add(1, Ordering::SeqCst);

        Ok(Arc::new(Mutex::new(Self {
            id,
            context_type: ContextType::TopLevel,
            arena: Arena::new(name, 64 * 1024, limit)?,
            _parent: None,
            children: Vec::new(),
            created_at: Instant::now(),
            stats: ContextStats::new(),
        })))
    }

    // Create a child context
    pub fn create_child(
        parent: &Arc<Mutex<Self>>,
        name: String,
        context_type: ContextType,
        limit: usize,
    ) -> Result<Arc<Mutex<Self>>> {
        static CONTEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = CONTEXT_ID.fetch_add(1, Ordering::SeqCst);

        let child = Arc::new(Mutex::new(Self {
            id,
            context_type,
            arena: Arena::new(name, 32 * 1024, limit)?,
            _parent: Some(Arc::downgrade(parent)),
            children: Vec::new(),
            created_at: Instant::now(),
            stats: ContextStats::new(),
        }));

        parent.lock().unwrap().children.push(Arc::clone(&child));

        Ok(child)
    }

    // Allocate memory in this context
    pub fn allocate(&mut self, size: usize) -> Result<NonNull<u8>> {
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);

        let ptr = unsafe { self.arena.allocate(size, 16)? };

        let current = self.arena.bytes_allocated();
        let peak = self.stats.peak_usage.load(Ordering::Relaxed);
        if current > peak {
            self.stats.peak_usage.store(current, Ordering::Relaxed);
        }

        Ok(ptr)
    }

    // Allocate aligned memory in this context
    pub fn allocate_aligned(&mut self, size: usize, align: usize) -> Result<NonNull<u8>> {
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);

        let ptr = unsafe { self.arena.allocate(size, align)? };

        let current = self.arena.bytes_allocated();
        let peak = self.stats.peak_usage.load(Ordering::Relaxed);
        if current > peak {
            self.stats.peak_usage.store(current, Ordering::Relaxed);
        }

        Ok(ptr)
    }

    // Reset this context (free all memory)
    pub fn reset(&mut self) {
        self.stats.resets.fetch_add(1, Ordering::Relaxed);
        self.arena.reset();

        // Reset all children
        for child in &self.children {
            if let Ok(mut child) = child.lock() {
                child.reset();
            }
        }
    }

    // Delete this context and all children
    pub fn delete(context: Arc<Mutex<Self>>) {
        let mut ctx = context.lock().unwrap();

        // Delete all children first
        let children = std::mem::take(&mut ctx.children);
        drop(ctx);

        for child in children {
            Self::delete(child);
        }
    }

    // Get context statistics
    pub fn get_stats(&self) -> MemoryContextStats {
        MemoryContextStats {
            id: self.id,
            context_type: self.context_type,
            bytes_allocated: self.arena.bytes_allocated(),
            peak_usage: self.stats.peak_usage.load(Ordering::Relaxed),
            allocations: self.stats.allocations.load(Ordering::Relaxed),
            resets: self.stats.resets.load(Ordering::Relaxed),
            age: self.created_at.elapsed(),
            child_count: self.children.len(),
        }
    }
}

// Public memory context statistics
#[derive(Debug, Clone)]
pub struct MemoryContextStats {
    pub id: u64,
    pub context_type: ContextType,
    pub bytes_allocated: usize,
    pub peak_usage: usize,
    pub allocations: u64,
    pub resets: u64,
    pub age: Duration,
    pub child_count: usize,
}

// Arena allocator manager
pub struct ArenaAllocator {
    // All active contexts
    contexts: RwLock<HashMap<u64, Weak<Mutex<MemoryContext>>>>,
    // Global statistics
    stats: ArenaStats,
}

unsafe impl Send for ArenaAllocator {}
unsafe impl Sync for ArenaAllocator {}

struct ArenaStats {
    contexts_created: AtomicU64,
    contexts_deleted: AtomicU64,
    total_resets: AtomicU64,
}

impl ArenaStats {
    fn new() -> Self {
        Self {
            contexts_created: AtomicU64::new(0),
            contexts_deleted: AtomicU64::new(0),
            total_resets: AtomicU64::new(0),
        }
    }
}

impl ArenaAllocator {
    // Create a new arena allocator
    pub fn new() -> Self {
        Self {
            contexts: RwLock::new(HashMap::new()),
            stats: ArenaStats::new(),
        }
    }

    // Create a new top-level context
    pub fn create_context(&self, name: String, limit: usize) -> Result<Arc<Mutex<MemoryContext>>> {
        let context = MemoryContext::new_top_level(name, limit)?;
        let id = context.lock().unwrap().id;

        self.contexts
            .write()
            .unwrap()
            .insert(id, Arc::downgrade(&context));
        self.stats.contexts_created.fetch_add(1, Ordering::Relaxed);

        Ok(context)
    }

    // Get global statistics
    pub fn get_stats(&self) -> ArenaAllocatorStats {
        let contexts = self.contexts.read().unwrap();
        let active_contexts = contexts.values().filter(|w| w.strong_count() > 0).count();
        let contexts_created = self.stats.contexts_created.load(Ordering::Relaxed);
        let contexts_deleted = self.stats.contexts_deleted.load(Ordering::Relaxed);
        let total_resets = self.stats.total_resets.load(Ordering::Relaxed);

        // Estimate allocated bytes based on active contexts (assume 1MB per context)
        let estimated_allocated = contexts_created.saturating_mul(1_048_576); // 1MB per context
        let estimated_freed = contexts_deleted.saturating_mul(1_048_576);
        let current_usage = estimated_allocated.saturating_sub(estimated_freed);
        let peak_usage = estimated_allocated;

        // Calculate fragmentation based on resets vs contexts
        let fragmentation = if contexts_created > 0 {
            (total_resets as f64 / contexts_created as f64).min(1.0)
        } else {
            0.0
        };

        ArenaAllocatorStats {
            contexts_created,
            contexts_deleted,
            active_contexts: active_contexts as u64,
            total_resets,
            total_allocated: estimated_allocated,
            total_freed: estimated_freed,
            current_usage,
            allocation_count: contexts_created,
            deallocation_count: contexts_deleted,
            peak_usage,
            fragmentation,
        }
    }

    // Cleanup dead contexts
    pub fn cleanup_dead_contexts(&self) -> usize {
        let mut contexts = self.contexts.write().unwrap();
        let before = contexts.len();
        contexts.retain(|_, weak| weak.strong_count() > 0);
        let removed = before - contexts.len();

        self.stats
            .contexts_deleted
            .fetch_add(removed as u64, Ordering::Relaxed);
        removed
    }
}

// Public arena allocator statistics
#[derive(Debug, Clone)]
pub struct ArenaAllocatorStats {
    pub contexts_created: u64,
    pub contexts_deleted: u64,
    pub active_contexts: u64,
    pub total_resets: u64,
    pub total_allocated: u64,
    pub total_freed: u64,
    pub current_usage: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub peak_usage: u64,
    pub fragmentation: f64,
}

// ============================================================================
