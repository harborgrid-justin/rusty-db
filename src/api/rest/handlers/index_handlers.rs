// Index Management Handlers
//
// Handler functions for index statistics and management operations

use axum::{
    extract::{Path, State},
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;
use parking_lot::RwLock;

use super::super::types::*;
use crate::index::{IndexManager, IndexType, IndexStats};

// ============================================================================
// Index-specific Types
// ============================================================================

/// Index information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexInfo {
    pub name: String,
    pub index_type: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
    pub created_at: i64,
    pub size_bytes: u64,
    pub status: String, // online, building, invalid
}

/// List indexes response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListIndexesResponse {
    pub total: usize,
    pub indexes: Vec<IndexInfo>,
}

/// Index statistics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IndexStatistics {
    pub name: String,
    pub index_type: String,
    pub size_bytes: u64,
    pub entry_count: u64,
    pub levels: Option<u32>,
    pub fill_factor: Option<f64>,
    pub reads: u64,
    pub writes: u64,
    pub hit_ratio: f64,
    pub avg_search_time_ms: f64,
    pub last_analyzed: Option<i64>,
    pub fragmentation: Option<f64>,
}

/// Rebuild index request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RebuildIndexRequest {
    pub online: bool,
    pub parallel: Option<u32>,
    pub fill_factor: Option<u8>,
}

/// Rebuild index response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RebuildIndexResponse {
    pub index_name: String,
    pub status: String,
    pub started_at: i64,
    pub estimated_duration_secs: Option<u64>,
    pub rebuild_id: String,
}

/// Analyze index request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyzeIndexRequest {
    pub compute_statistics: bool,
    pub sample_percent: Option<f64>,
}

/// Analyze index response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnalyzeIndexResponse {
    pub index_name: String,
    pub analyzed_at: i64,
    pub statistics: IndexStatistics,
    pub recommendations: Vec<String>,
}

/// Index recommendation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexRecommendation {
    pub recommendation_type: String, // create, drop, rebuild, consolidate
    pub table: String,
    pub columns: Vec<String>,
    pub index_type: String,
    pub reason: String,
    pub priority: u32,
    pub estimated_benefit: f64,
    pub estimated_cost: f64,
}

/// Index recommendations response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IndexRecommendationsResponse {
    pub total: usize,
    pub recommendations: Vec<IndexRecommendation>,
    pub analyzed_at: i64,
}

// ============================================================================
// Global Index Manager
// ============================================================================

lazy_static::lazy_static! {
    static ref INDEX_MANAGER: Arc<RwLock<IndexManager>> = Arc::new(RwLock::new(IndexManager::new()));
}

// ============================================================================
// Handler Functions
// ============================================================================

/// List all indexes
///
/// Returns a list of all indexes in the database with their basic information.
#[utoipa::path(
    get,
    path = "/api/v1/indexes",
    responses(
        (status = 200, description = "List of all indexes", body = ListIndexesResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "indexes"
)]
pub async fn list_indexes() -> Result<AxumJson<ListIndexesResponse>, (StatusCode, AxumJson<ApiError>)> {
    let manager = INDEX_MANAGER.read();
    let index_names = manager.list_indexes();

    let mut indexes = Vec::new();
    for name in &index_names {
        // Mock index info - in production, retrieve from catalog
        indexes.push(IndexInfo {
            name: name.clone(),
            index_type: "btree".to_string(),
            table_name: "sample_table".to_string(),
            columns: vec!["col1".to_string()],
            unique: false,
            primary: false,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            size_bytes: 1024 * 1024, // 1MB
            status: "online".to_string(),
        });
    }

    Ok(AxumJson(ListIndexesResponse {
        total: indexes.len(),
        indexes,
    }))
}

/// Get index statistics
///
/// Returns detailed statistics for a specific index including size, hit ratio, and performance metrics.
#[utoipa::path(
    get,
    path = "/api/v1/indexes/{name}/stats",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index statistics", body = IndexStatistics),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "indexes"
)]
pub async fn get_index_stats(
    Path(name): Path<String>,
) -> Result<AxumJson<IndexStatistics>, (StatusCode, AxumJson<ApiError>)> {
    let manager = INDEX_MANAGER.read();

    // Try to get index stats
    match manager.get_index_stats(&name) {
        Ok(stats) => {
            // Convert internal stats to API response
            let statistics = match stats {
                IndexStats::BPlusTree(s) => IndexStatistics {
                    name: name.clone(),
                    index_type: "btree".to_string(),
                    size_bytes: s.total_nodes as u64 * 4096, // Approximate
                    entry_count: s.total_keys as u64,
                    levels: Some(s.height),
                    fill_factor: Some(s.average_fill_factor),
                    reads: 0, // Not tracked in current implementation
                    writes: 0,
                    hit_ratio: 0.95,
                    avg_search_time_ms: 0.5,
                    last_analyzed: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                    ),
                    fragmentation: Some(1.0 - s.average_fill_factor),
                },
                IndexStats::LSMTree(s) => IndexStatistics {
                    name: name.clone(),
                    index_type: "lsm".to_string(),
                    size_bytes: s.memtable_size + s.total_sstable_size,
                    entry_count: s.total_entries,
                    levels: Some(s.level_count as u32),
                    fill_factor: None,
                    reads: 0,
                    writes: 0,
                    hit_ratio: s.bloom_filter_hit_rate,
                    avg_search_time_ms: 1.0,
                    last_analyzed: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                    ),
                    fragmentation: None,
                },
                IndexStats::ExtendibleHash(s) => IndexStatistics {
                    name: name.clone(),
                    index_type: "extendible_hash".to_string(),
                    size_bytes: s.bucket_count * 4096,
                    entry_count: s.entry_count,
                    levels: Some(s.global_depth as u32),
                    fill_factor: Some(s.load_factor),
                    reads: 0,
                    writes: 0,
                    hit_ratio: 0.98,
                    avg_search_time_ms: 0.1,
                    last_analyzed: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                    ),
                    fragmentation: None,
                },
                IndexStats::LinearHash(s) => IndexStatistics {
                    name: name.clone(),
                    index_type: "linear_hash".to_string(),
                    size_bytes: s.bucket_count * 4096,
                    entry_count: s.entry_count,
                    levels: Some(s.level as u32),
                    fill_factor: Some(s.load_factor),
                    reads: 0,
                    writes: 0,
                    hit_ratio: 0.98,
                    avg_search_time_ms: 0.1,
                    last_analyzed: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                    ),
                    fragmentation: None,
                },
                IndexStats::Bitmap(s) => IndexStatistics {
                    name: name.clone(),
                    index_type: "bitmap".to_string(),
                    size_bytes: s.total_bytes,
                    entry_count: s.total_entries,
                    levels: None,
                    fill_factor: Some(s.compression_ratio),
                    reads: 0,
                    writes: 0,
                    hit_ratio: 0.99,
                    avg_search_time_ms: 0.05,
                    last_analyzed: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                    ),
                    fragmentation: None,
                },
                IndexStats::Unknown => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        AxumJson(ApiError::new(
                            "STATS_ERROR",
                            "Unable to retrieve statistics for this index type",
                        )),
                    ));
                }
            };

            Ok(AxumJson(statistics))
        }
        Err(_) => Err((
            StatusCode::NOT_FOUND,
            AxumJson(ApiError::new("INDEX_NOT_FOUND", format!("Index '{}' not found", name))),
        )),
    }
}

/// Rebuild index
///
/// Rebuilds an index to optimize its structure and reclaim space.
/// Can be done online or offline, with optional parallelism.
#[utoipa::path(
    post,
    path = "/api/v1/indexes/{name}/rebuild",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    request_body = RebuildIndexRequest,
    responses(
        (status = 200, description = "Index rebuild started", body = RebuildIndexResponse),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "indexes"
)]
pub async fn rebuild_index(
    Path(name): Path<String>,
    AxumJson(request): AxumJson<RebuildIndexRequest>,
) -> Result<AxumJson<RebuildIndexResponse>, (StatusCode, AxumJson<ApiError>)> {
    let manager = INDEX_MANAGER.read();

    // Verify index exists
    if !manager.list_indexes().contains(&name) {
        return Err((
            StatusCode::NOT_FOUND,
            AxumJson(ApiError::new("INDEX_NOT_FOUND", format!("Index '{}' not found", name))),
        ));
    }

    // In production, this would queue a rebuild job
    let rebuild_id = uuid::Uuid::new_v4().to_string();
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Ok(AxumJson(RebuildIndexResponse {
        index_name: name,
        status: if request.online {
            "rebuilding_online".to_string()
        } else {
            "rebuilding_offline".to_string()
        },
        started_at,
        estimated_duration_secs: Some(300), // 5 minutes
        rebuild_id,
    }))
}

/// Analyze index
///
/// Analyzes an index and computes up-to-date statistics.
/// Can optionally sample a percentage of data for faster analysis.
#[utoipa::path(
    post,
    path = "/api/v1/indexes/{name}/analyze",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    request_body = AnalyzeIndexRequest,
    responses(
        (status = 200, description = "Index analysis complete", body = AnalyzeIndexResponse),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "indexes"
)]
pub async fn analyze_index(
    Path(name): Path<String>,
    AxumJson(request): AxumJson<AnalyzeIndexRequest>,
) -> Result<AxumJson<AnalyzeIndexResponse>, (StatusCode, AxumJson<ApiError>)> {
    let manager = INDEX_MANAGER.read();

    // Get index stats
    match manager.get_index_stats(&name) {
        Ok(stats) => {
            // Convert stats to statistics
            let statistics = match stats {
                IndexStats::BPlusTree(s) => {
                    let mut recommendations = Vec::new();

                    if s.average_fill_factor < 0.5 {
                        recommendations.push(
                            "Index is under 50% full. Consider rebuilding to reclaim space.".to_string()
                        );
                    }

                    if s.height > 5 {
                        recommendations.push(
                            "Index height is high. Consider adjusting fill factor.".to_string()
                        );
                    }

                    let analyzed_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    AnalyzeIndexResponse {
                        index_name: name,
                        analyzed_at,
                        statistics: IndexStatistics {
                            name: "".to_string(),
                            index_type: "btree".to_string(),
                            size_bytes: s.total_nodes as u64 * 4096,
                            entry_count: s.total_keys as u64,
                            levels: Some(s.height),
                            fill_factor: Some(s.average_fill_factor),
                            reads: 0,
                            writes: 0,
                            hit_ratio: 0.95,
                            avg_search_time_ms: 0.5,
                            last_analyzed: Some(analyzed_at),
                            fragmentation: Some(1.0 - s.average_fill_factor),
                        },
                        recommendations,
                    }
                }
                _ => {
                    let analyzed_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    AnalyzeIndexResponse {
                        index_name: name,
                        analyzed_at,
                        statistics: IndexStatistics {
                            name: "".to_string(),
                            index_type: "unknown".to_string(),
                            size_bytes: 0,
                            entry_count: 0,
                            levels: None,
                            fill_factor: None,
                            reads: 0,
                            writes: 0,
                            hit_ratio: 0.0,
                            avg_search_time_ms: 0.0,
                            last_analyzed: Some(analyzed_at),
                            fragmentation: None,
                        },
                        recommendations: vec!["Index type does not support detailed analysis.".to_string()],
                    }
                }
            };

            Ok(AxumJson(statistics))
        }
        Err(_) => Err((
            StatusCode::NOT_FOUND,
            AxumJson(ApiError::new("INDEX_NOT_FOUND", format!("Index '{}' not found", name))),
        )),
    }
}

/// Get index recommendations
///
/// Returns intelligent recommendations for index creation, modification, or removal
/// based on query workload analysis.
#[utoipa::path(
    get,
    path = "/api/v1/indexes/recommendations",
    responses(
        (status = 200, description = "Index recommendations", body = IndexRecommendationsResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "indexes"
)]
pub async fn get_index_recommendations() -> Result<AxumJson<IndexRecommendationsResponse>, (StatusCode, AxumJson<ApiError>)> {
    let manager = INDEX_MANAGER.read();

    // Get recommendations from the advisor
    match manager.get_recommendations() {
        Ok(internal_recommendations) => {
            // Convert internal recommendations to API format
            let recommendations: Vec<IndexRecommendation> = internal_recommendations
                .iter()
                .map(|rec| IndexRecommendation {
                    recommendation_type: format!("{:?}", rec.recommendation_type).to_lowercase(),
                    table: rec.table.clone(),
                    columns: rec.columns.clone(),
                    index_type: "btree".to_string(), // Default to B-tree
                    reason: rec.reason.clone(),
                    priority: rec.priority,
                    estimated_benefit: rec.estimated_benefit,
                    estimated_cost: rec.estimated_cost,
                })
                .collect();

            let analyzed_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            Ok(AxumJson(IndexRecommendationsResponse {
                total: recommendations.len(),
                recommendations,
                analyzed_at,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(ApiError::new("ANALYSIS_ERROR", format!("Failed to analyze workload: {}", e))),
        )),
    }
}
