// # Security Labels API Handlers
//
// REST API endpoints for managing security labels, compartments, and
// mandatory access control (MAC) policies.

use axum::{
    extract::{State, Path},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashSet;
use parking_lot::RwLock;
use crate::api::rest::types::{ApiState, ApiResult, ApiError};
use crate::security::labels::{SecurityLabel, ClassificationLevel, Compartment, UserClearance};

// Request/Response Types

/// Security label response
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityLabelResponse {
    pub classification: String,
    pub compartments: Vec<String>,
    pub groups: Vec<String>,
}

/// Create security label request
#[derive(Debug, Deserialize)]
pub struct CreateSecurityLabel {
    pub classification: String,
    pub compartments: Option<Vec<String>>,
    pub groups: Option<Vec<String>>,
}

/// Compartment response
#[derive(Debug, Serialize, Deserialize)]
pub struct CompartmentResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent: Option<String>,
}

/// Create compartment request
#[derive(Debug, Deserialize)]
pub struct CreateCompartment {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent: Option<String>,
}

/// User clearance response
#[derive(Debug, Serialize, Deserialize)]
pub struct UserClearanceResponse {
    pub user_id: String,
    pub max_read_classification: String,
    pub max_write_classification: String,
    pub current_classification: String,
    pub authorized_compartments: Vec<String>,
}

/// Set user clearance request
#[derive(Debug, Deserialize)]
pub struct SetUserClearance {
    pub user_id: String,
    pub max_read_classification: String,
    pub max_write_classification: String,
    pub authorized_compartments: Vec<String>,
}

/// Label dominance check request
#[derive(Debug, Deserialize)]
pub struct CheckDominance {
    pub label1: CreateSecurityLabel,
    pub label2: CreateSecurityLabel,
}

/// Label dominance check result
#[derive(Debug, Serialize, Deserialize)]
pub struct DominanceResult {
    pub label1_dominates_label2: bool,
    pub label2_dominates_label1: bool,
    pub labels_equal: bool,
    pub comparable: bool,
}

// Global label registry
lazy_static::lazy_static! {
    static ref LABEL_REGISTRY: Arc<RwLock<LabelRegistry>> = Arc::new(RwLock::new(LabelRegistry::new()));
}

// Simple label registry for demo purposes
struct LabelRegistry {
    compartments: Vec<Compartment>,
    user_clearances: std::collections::HashMap<String, UserClearance>,
}

impl LabelRegistry {
    fn new() -> Self {
        Self {
            compartments: Vec::new(),
            user_clearances: std::collections::HashMap::new(),
        }
    }
}

// Helper function to parse classification level
fn parse_classification(s: &str) -> Result<ClassificationLevel, ApiError> {
    match s.to_uppercase().as_str() {
        "UNCLASSIFIED" => Ok(ClassificationLevel::Unclassified),
        "RESTRICTED" => Ok(ClassificationLevel::Restricted),
        "CONFIDENTIAL" => Ok(ClassificationLevel::Confidential),
        "SECRET" => Ok(ClassificationLevel::Secret),
        "TOPSECRET" | "TOP_SECRET" => Ok(ClassificationLevel::TopSecret),
        _ => Err(ApiError::new("INVALID_CLASSIFICATION", format!("Unknown classification: {}", s))),
    }
}

// Helper function to convert SecurityLabel to response
fn label_to_response(label: &SecurityLabel) -> SecurityLabelResponse {
    SecurityLabelResponse {
        classification: format!("{:?}", label.classification),
        compartments: label.compartments.iter().cloned().collect(),
        groups: label.groups.iter().cloned().collect(),
    }
}

// Helper function to create SecurityLabel from request
fn request_to_label(request: &CreateSecurityLabel) -> Result<SecurityLabel, ApiError> {
    let classification = parse_classification(&request.classification)?;
    let mut label = SecurityLabel::new(classification);

    if let Some(ref compartments) = request.compartments {
        for comp in compartments {
            label.add_compartment(comp.clone());
        }
    }

    if let Some(ref groups) = request.groups {
        for group in groups {
            label.add_group(group.clone());
        }
    }

    Ok(label)
}

// API Handlers

/// GET /api/v1/security/labels/compartments
///
/// List all security compartments.
pub async fn list_compartments(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<CompartmentResponse>>> {
    let registry = LABEL_REGISTRY.read();

    let responses: Vec<CompartmentResponse> = registry.compartments
        .iter()
        .map(|c| CompartmentResponse {
            id: c.id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            parent: c.parent.clone(),
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/v1/security/labels/compartments
///
/// Create a new security compartment.
pub async fn create_compartment(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateCompartment>,
) -> ApiResult<Json<CompartmentResponse>> {
    let mut registry = LABEL_REGISTRY.write();

    let compartment = Compartment {
        id: request.id.clone(),
        name: request.name.clone(),
        description: request.description.clone(),
        parent: request.parent.clone(),
    };

    registry.compartments.push(compartment.clone());

    Ok(Json(CompartmentResponse {
        id: compartment.id,
        name: compartment.name,
        description: compartment.description,
        parent: compartment.parent,
    }))
}

/// GET /api/v1/security/labels/compartments/{id}
///
/// Get a specific compartment by ID.
pub async fn get_compartment(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<CompartmentResponse>> {
    let registry = LABEL_REGISTRY.read();

    if let Some(compartment) = registry.compartments.iter().find(|c| c.id == id) {
        Ok(Json(CompartmentResponse {
            id: compartment.id.clone(),
            name: compartment.name.clone(),
            description: compartment.description.clone(),
            parent: compartment.parent.clone(),
        }))
    } else {
        Err(ApiError::new("COMPARTMENT_NOT_FOUND", format!("Compartment {} not found", id)))
    }
}

/// DELETE /api/v1/security/labels/compartments/{id}
///
/// Delete a security compartment.
pub async fn delete_compartment(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut registry = LABEL_REGISTRY.write();

    if let Some(pos) = registry.compartments.iter().position(|c| c.id == id) {
        registry.compartments.remove(pos);
        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Compartment {} deleted", id),
        })))
    } else {
        Err(ApiError::new("COMPARTMENT_NOT_FOUND", format!("Compartment {} not found", id)))
    }
}

/// GET /api/v1/security/labels/clearances/{user_id}
///
/// Get user clearance information.
pub async fn get_user_clearance(
    State(_state): State<Arc<ApiState>>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserClearanceResponse>> {
    let registry = LABEL_REGISTRY.read();

    if let Some(clearance) = registry.user_clearances.get(&user_id) {
        Ok(Json(UserClearanceResponse {
            user_id: clearance.user_id.clone(),
            max_read_classification: format!("{:?}", clearance.max_read.classification),
            max_write_classification: format!("{:?}", clearance.max_write.classification),
            current_classification: format!("{:?}", clearance.current_label.classification),
            authorized_compartments: clearance.authorized_compartments.iter().cloned().collect(),
        }))
    } else {
        Err(ApiError::new("CLEARANCE_NOT_FOUND", format!("No clearance found for user {}", user_id)))
    }
}

/// POST /api/v1/security/labels/clearances
///
/// Set user clearance.
pub async fn set_user_clearance(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<SetUserClearance>,
) -> ApiResult<Json<UserClearanceResponse>> {
    let mut registry = LABEL_REGISTRY.write();

    let max_read_class = parse_classification(&request.max_read_classification)?;
    let max_write_class = parse_classification(&request.max_write_classification)?;

    let mut max_read = SecurityLabel::new(max_read_class.clone());
    let mut max_write = SecurityLabel::new(max_write_class.clone());
    let current_label = SecurityLabel::new(max_write_class);

    let mut authorized_compartments = HashSet::new();
    for comp in &request.authorized_compartments {
        authorized_compartments.insert(comp.clone());
        max_read.add_compartment(comp.clone());
        max_write.add_compartment(comp.clone());
    }

    let clearance = UserClearance {
        user_id: request.user_id.clone(),
        max_read,
        max_write,
        current_label,
        authorized_compartments: authorized_compartments.clone(),
        authorized_groups: HashSet::new(),
    };

    registry.user_clearances.insert(request.user_id.clone(), clearance);

    Ok(Json(UserClearanceResponse {
        user_id: request.user_id,
        max_read_classification: request.max_read_classification.clone(),
        max_write_classification: request.max_write_classification.clone(),
        current_classification: request.max_write_classification,
        authorized_compartments: authorized_compartments.into_iter().collect(),
    }))
}

/// POST /api/v1/security/labels/check-dominance
///
/// Check label dominance relationship.
pub async fn check_label_dominance(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CheckDominance>,
) -> ApiResult<Json<DominanceResult>> {
    let label1 = request_to_label(&request.label1)?;
    let label2 = request_to_label(&request.label2)?;

    let l1_dominates_l2 = label1.dominates(&label2);
    let l2_dominates_l1 = label2.dominates(&label1);
    let labels_equal = label1 == label2;
    let comparable = label1.comparable(&label2);

    Ok(Json(DominanceResult {
        label1_dominates_label2: l1_dominates_l2,
        label2_dominates_label1: l2_dominates_l1,
        labels_equal,
        comparable,
    }))
}

/// POST /api/v1/security/labels/validate-access
///
/// Validate if a user can access data with a specific label.
pub async fn validate_label_access(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_id = request["user_id"].as_str().unwrap_or("unknown");
    let label_req = serde_json::from_value::<CreateSecurityLabel>(request["label"].clone())
        .map_err(|e| ApiError::new("INVALID_REQUEST", e.to_string()))?;

    let data_label = request_to_label(&label_req)?;

    let registry = LABEL_REGISTRY.read();

    if let Some(clearance) = registry.user_clearances.get(user_id) {
        let can_read = clearance.max_read.dominates(&data_label);
        let can_write = clearance.max_write.dominates(&data_label);

        Ok(Json(serde_json::json!({
            "user_id": user_id,
            "can_read": can_read,
            "can_write": can_write,
            "data_label": label_to_response(&data_label),
            "user_clearance": {
                "max_read": label_to_response(&clearance.max_read),
                "max_write": label_to_response(&clearance.max_write),
            },
        })))
    } else {
        Err(ApiError::new("CLEARANCE_NOT_FOUND", format!("No clearance found for user {}", user_id)))
    }
}

/// GET /api/v1/security/labels/classifications
///
/// List all available classification levels.
pub async fn list_classifications(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<String>>> {
    Ok(Json(vec![
        "UNCLASSIFIED".to_string(),
        "RESTRICTED".to_string(),
        "CONFIDENTIAL".to_string(),
        "SECRET".to_string(),
        "TOPSECRET".to_string(),
    ]))
}
