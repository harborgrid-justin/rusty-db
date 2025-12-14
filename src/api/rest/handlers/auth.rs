// Authentication Handlers
//
// Handler functions for authentication operations

use axum::{extract::State, response::Json as AxumJson};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;
use uuid::Uuid;

use super::super::types::*;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub session: SessionInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: Option<String>,
    pub roles: Vec<RoleInfo>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionInfo {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

// ============================================================================
// Authentication Handlers
// ============================================================================

/// Login endpoint
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ApiError),
    )
)]
pub async fn login(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<LoginRequest>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    // For now, accept admin/admin as valid credentials
    // TODO: Integrate with actual AuthenticationManager

    if request.username == "admin" && request.password == "admin" {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = now + 3600; // 1 hour

        let response_data = LoginResponse {
            user: UserInfo {
                id: "1".to_string(),
                username: "admin".to_string(),
                display_name: "Administrator".to_string(),
                email: Some("admin@rustydb.local".to_string()),
                roles: vec![RoleInfo {
                    id: "1".to_string(),
                    name: "admin".to_string(),
                    permissions: vec!["*".to_string()],
                }],
            },
            session: SessionInfo {
                token: format!("token_{}", Uuid::new_v4()),
                refresh_token: format!("refresh_{}", Uuid::new_v4()),
                expires_at: format!("{}", expires_at),
            },
        };

        // Wrap in standard API response format
        Ok(AxumJson(serde_json::json!({
            "success": true,
            "data": response_data
        })))
    } else {
        Err(ApiError::new(
            "UNAUTHORIZED",
            "Invalid username or password",
        ))
    }
}

/// Logout endpoint
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "Logout successful"),
    )
)]
pub async fn logout(State(_state): State<Arc<ApiState>>) -> ApiResult<AxumJson<serde_json::Value>> {
    Ok(AxumJson(serde_json::json!({
        "message": "Logged out successfully"
    })))
}

/// Refresh token endpoint
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "Token refreshed", body = SessionInfo),
    )
)]
pub async fn refresh(State(_state): State<Arc<ApiState>>) -> ApiResult<AxumJson<SessionInfo>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expires_at = now + 3600;

    let session = SessionInfo {
        token: format!("token_{}", Uuid::new_v4()),
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        expires_at: format!("{}", expires_at),
    };

    Ok(AxumJson(session))
}

/// Validate session endpoint
#[utoipa::path(
    get,
    path = "/api/v1/auth/validate",
    tag = "auth",
    responses(
        (status = 200, description = "Session valid"),
    )
)]
pub async fn validate(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    Ok(AxumJson(serde_json::json!({
        "valid": true
    })))
}
