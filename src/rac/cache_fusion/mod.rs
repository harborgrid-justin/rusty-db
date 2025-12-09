// # Cache Fusion Protocol
//
// Oracle RAC-like Cache Fusion technology for direct memory-to-memory block transfers
// between cluster instances without requiring disk I/O for inter-instance data sharing.
//
// ## Module Organization
//
// - `global_cache`: Global Cache Service (GCS) for block coordination
// - `lock_management`: Global Enqueue Service (GES) for distributed locking
// - `cache_coherence`: High-level coordinator integrating GCS and GES
//
// ## Key Components
//
// - **Global Cache Service (GCS)**: Coordinates data block sharing across instances
// - **Global Enqueue Service (GES)**: Manages distributed locks and resources
// - **Block Transfer Engine**: Zero-copy RDMA-like transfers between nodes
// - **Consistency Protocols**: Read-read, read-write, write-write coordination
//
// ## Architecture
//
// Cache Fusion eliminates the traditional disk-ping problem in shared-disk clusters
// by allowing direct transfer of cached blocks from one instance's memory to another's,
// maintaining strict consistency guarantees through sophisticated locking protocols.

pub mod global_cache;
pub mod lock_management;
pub mod cache_coherence;

// Re-export commonly used types from global_cache
pub use global_cache::{
    GlobalCacheService,
    GcsConfig,
    GcsStatistics,
    BlockMode,
    BlockGrant,
    BlockState,
    ResourceId,
    ResourceClass,
    LockValueBlock,
    CacheFusionMessage,
    MAX_BLOCK_SIZE,
    MAX_CONCURRENT_TRANSFERS,
    TRANSFER_TIMEOUT,
    GCS_MESSAGE_TIMEOUT,
};

// Re-export commonly used types from lock_management
pub use lock_management::{
    GlobalEnqueueService,
    GesStatistics,
    LockType,
    LockGrant,
    LOCK_CONVERSION_TIMEOUT,
};

// Re-export commonly used types from cache_coherence
pub use cache_coherence::{
    CacheFusionCoordinator,
    CacheFusionStatistics,
};
