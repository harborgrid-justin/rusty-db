// Health Probe Handlers
//
// Kubernetes-style health probe endpoints for liveness, readiness, and startup checks

use axum::{
    extract::State,
    response::Json as AxumJson,
    http::StatusCode,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use super::super::types::*;
use crate::api::monitoring::{HealthCheckCoordinator, HealthCheckResult, HealthStatus};

// Liveness probe response
#[derive(Debug, Serialize, Deserialize)]
pub struct LivenessProbeResponse {
    pub status: String,
    pub timestamp: i64,
    pub uptime_seconds: Option<u64>,
}

// Readiness probe response
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadinessProbeResponse {
    pub status: String,
    pub timestamp: i64,
    pub ready: bool,
    pub dependencies: HashMap<String, String>,
}

// Startup probe response
#[derive(Debug, Serialize, Deserialize)]
pub struct StartupProbeResponse {
    pub status: String,
    pub timestamp: i64,
    pub initialized: bool,
    pub checks: HashMap<String, bool>,
}

// Full health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct FullHealthResponse {
    pub status: String,
    pub timestamp: i64,
    pub components: Vec<ComponentHealthDetail>,
    pub liveness: LivenessProbeResponse,
    pub readiness: ReadinessProbeResponse,
    pub startup: StartupProbeResponse,
}

// Component health detail
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealthDetail {
    pub component: String,
    pub status: String,
    pub message: String,
    pub duration_ms: u64,
    pub details: HashMap<String, serde_json::Value>,
}

// Helper function to convert HealthStatus to string
fn health_status_to_string(status: HealthStatus) -> String {
    match status {
        HealthStatus::Healthy => "healthy".to_string(),
        HealthStatus::Degraded => "degraded".to_string(),
        HealthStatus::Unhealthy => "unhealthy".to_string(),
        HealthStatus::Unknown => "unknown".to_string(),
    }
}

// Helper function to convert HealthCheckResult to ComponentHealthDetail
fn health_result_to_detail(result: HealthCheckResult) -> ComponentHealthDetail {
    ComponentHealthDetail {
        component: result.component,
        status: health_status_to_string(result.status),
        message: result.message,
        duration_ms: result.duration.as_millis() as u64,
        details: result.details,
    }
}

/// GET /api/v1/health/liveness
///
/// Liveness probe - indicates if the service is alive
/// Returns 200 if alive, 503 if not
#[utoipa::path(
    get,
    path = "/api/v1/health/liveness",
    tag = "health",
    responses(
        (status = 200, description = "Service is alive", body = LivenessProbeResponse),
        (status = 503, description = "Service is not alive", body = LivenessProbeResponse),
    )
)]
pub async fn liveness_probe(
    State(_state): State<Arc<ApiState>>,
) -> Result<AxumJson<LivenessProbeResponse>, (StatusCode, AxumJson<LivenessProbeResponse>)> {
    // In a real implementation, this would check with the HealthCheckCoordinator
    // For now, we'll return a simple response indicating the service is alive
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let response = LivenessProbeResponse {
        status: "healthy".to_string(),
        timestamp: now,
        uptime_seconds: Some(3600), // Mock uptime
    };

    Ok(AxumJson(response))
}

/// GET /api/v1/health/readiness
///
/// Readiness probe - indicates if the service is ready to accept traffic
/// Returns 200 if ready, 503 if not
#[utoipa::path(
    get,
    path = "/api/v1/health/readiness",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessProbeResponse),
        (status = 503, description = "Service is not ready", body = ReadinessProbeResponse),
    )
)]
pub async fn readiness_probe(
    State(_state): State<Arc<ApiState>>,
) -> Result<AxumJson<ReadinessProbeResponse>, (StatusCode, AxumJson<ReadinessProbeResponse>)> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // In a real implementation, check database connections, external dependencies, etc.
    let mut dependencies = HashMap::new();
    dependencies.insert("database".to_string(), "healthy".to_string());
    dependencies.insert("cache".to_string(), "healthy".to_string());

    let response = ReadinessProbeResponse {
        status: "healthy".to_string(),
        timestamp: now,
        ready: true,
        dependencies,
    };

    Ok(AxumJson(response))
}

/// GET /api/v1/health/startup
///
/// Startup probe - indicates if the service has completed initialization
/// Returns 200 if initialized, 503 if not
#[utoipa::path(
    get,
    path = "/api/v1/health/startup",
    tag = "health",
    responses(
        (status = 200, description = "Service is initialized", body = StartupProbeResponse),
        (status = 503, description = "Service is not initialized", body = StartupProbeResponse),
    )
)]
pub async fn startup_probe(
    State(_state): State<Arc<ApiState>>,
) -> Result<AxumJson<StartupProbeResponse>, (StatusCode, AxumJson<StartupProbeResponse>)> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // In a real implementation, check initialization status
    let mut checks = HashMap::new();
    checks.insert("database_migration".to_string(), true);
    checks.insert("cache_warmed".to_string(), true);
    checks.insert("configs_loaded".to_string(), true);

    let response = StartupProbeResponse {
        status: "healthy".to_string(),
        timestamp: now,
        initialized: true,
        checks,
    };

    Ok(AxumJson(response))
}

/// GET /api/v1/health/full
///
/// Comprehensive health check - returns detailed health information
/// Returns 200 if healthy, 503 if unhealthy, 429 if degraded
#[utoipa::path(
    get,
    path = "/api/v1/health/full",
    tag = "health",
    responses(
        (status = 200, description = "All components healthy", body = FullHealthResponse),
        (status = 429, description = "Some components degraded", body = FullHealthResponse),
        (status = 503, description = "Some components unhealthy", body = FullHealthResponse),
    )
)]
pub async fn full_health_check(
    State(_state): State<Arc<ApiState>>,
) -> Result<AxumJson<FullHealthResponse>, (StatusCode, AxumJson<FullHealthResponse>)> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Mock component health checks
    let components = vec![
        ComponentHealthDetail {
            component: "storage".to_string(),
            status: "healthy".to_string(),
            message: "Storage system operational".to_string(),
            duration_ms: 5,
            details: HashMap::new(),
        },
        ComponentHealthDetail {
            component: "network".to_string(),
            status: "healthy".to_string(),
            message: "Network connections stable".to_string(),
            duration_ms: 3,
            details: HashMap::new(),
        },
        ComponentHealthDetail {
            component: "memory".to_string(),
            status: "healthy".to_string(),
            message: "Memory usage within limits".to_string(),
            duration_ms: 2,
            details: HashMap::new(),
        },
    ];

    let liveness = LivenessProbeResponse {
        status: "healthy".to_string(),
        timestamp: now,
        uptime_seconds: Some(3600),
    };

    let mut readiness_deps = HashMap::new();
    readiness_deps.insert("database".to_string(), "healthy".to_string());
    readiness_deps.insert("cache".to_string(), "healthy".to_string());

    let readiness = ReadinessProbeResponse {
        status: "healthy".to_string(),
        timestamp: now,
        ready: true,
        dependencies: readiness_deps,
    };

    let mut startup_checks = HashMap::new();
    startup_checks.insert("database_migration".to_string(), true);
    startup_checks.insert("cache_warmed".to_string(), true);
    startup_checks.insert("configs_loaded".to_string(), true);

    let startup = StartupProbeResponse {
        status: "healthy".to_string(),
        timestamp: now,
        initialized: true,
        checks: startup_checks,
    };

    let response = FullHealthResponse {
        status: "healthy".to_string(),
        timestamp: now,
        components,
        liveness,
        readiness,
        startup,
    };

    Ok(AxumJson(response))
}
