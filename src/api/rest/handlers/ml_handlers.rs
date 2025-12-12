// Machine Learning API Handlers
//
// REST API endpoints for in-database machine learning including:
// - Model creation and training
// - Prediction and inference
// - Model management
// - Feature engineering

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::rest::types::{ApiState, ApiError, ApiResult};
use crate::ml::{
    MLEngine, ModelType, Dataset, Hyperparameters, Metrics,
    Algorithm, LinearRegression, LogisticRegression, KMeansClustering,
    InferenceEngine, PredictionResult,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateModelRequest {
    pub name: String,
    pub model_type: String, // linear_regression, logistic_regression, kmeans, etc.
    pub hyperparameters: Option<HashMap<String, serde_json::Value>>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelResponse {
    pub model_id: String,
    pub name: String,
    pub model_type: String,
    pub status: String,
    pub created_at: i64,
    pub version: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrainModelRequest {
    // SQL query to fetch training data
    pub data_query: Option<String>,
    // Direct data for training
    pub features: Option<Vec<Vec<f64>>>,
    pub target: Option<Vec<f64>>,
    pub feature_names: Option<Vec<String>>,
    // Training configuration
    pub validation_split: Option<f64>,
    pub epochs: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrainModelResponse {
    pub model_id: String,
    pub status: String,
    pub metrics: HashMap<String, f64>,
    pub training_time_ms: u64,
    pub epochs_completed: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PredictRequest {
    pub features: Vec<Vec<f64>>,
    pub feature_names: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PredictResponse {
    pub predictions: Vec<f64>,
    pub confidence_scores: Option<Vec<f64>>,
    pub prediction_count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelListResponse {
    pub models: Vec<ModelSummary>,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelSummary {
    pub model_id: String,
    pub name: String,
    pub model_type: String,
    pub status: String,
    pub accuracy: Option<f64>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelMetricsResponse {
    pub model_id: String,
    pub metrics: HashMap<String, f64>,
    pub feature_importance: Option<Vec<FeatureImportance>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeatureImportance {
    pub feature_name: String,
    pub importance: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelEvaluationRequest {
    pub test_features: Vec<Vec<f64>>,
    pub test_target: Vec<f64>,
    pub metrics: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModelEvaluationResponse {
    pub model_id: String,
    pub metrics: HashMap<String, f64>,
    pub confusion_matrix: Option<Vec<Vec<i32>>>,
}

// ============================================================================
// Handler Functions
// ============================================================================

// Global ML engine instance
lazy_static::lazy_static! {
    static ref ML_ENGINE: parking_lot::RwLock<MLEngine> = parking_lot::RwLock::new(MLEngine::new());
}

/// Create a new machine learning model
#[utoipa::path(
    post,
    path = "/api/v1/ml/models",
    request_body = CreateModelRequest,
    responses(
        (status = 201, description = "Model created", body = ModelResponse),
        (status = 400, description = "Invalid model configuration", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn create_model(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateModelRequest>,
) -> ApiResult<(StatusCode, Json<ModelResponse>)> {
    let mut engine = ML_ENGINE.write();

    // Parse model type
    let model_type = match request.model_type.as_str() {
        "linear_regression" => ModelType::LinearRegression,
        "logistic_regression" => ModelType::LogisticRegression,
        "kmeans" => ModelType::KMeansClustering,
        "decision_tree" => ModelType::DecisionTree,
        "random_forest" => ModelType::RandomForest,
        _ => {
            return Err(ApiError::new(
                "INVALID_MODEL_TYPE",
                format!("Unsupported model type: {}", request.model_type),
            ));
        }
    };

    // Build hyperparameters
    let mut hyperparams = Hyperparameters::new();
    if let Some(params) = request.hyperparameters {
        for (key, value) in params {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        hyperparams.set_float(&key, f);
                    }
                }
                serde_json::Value::Bool(b) => {
                    hyperparams.set_bool(&key, b);
                }
                serde_json::Value::String(s) => {
                    hyperparams.set_string(&key, s);
                }
                _ => {}
            }
        }
    }

    // Create model
    let model_id = engine.create_model(
        request.name.clone(),
        model_type,
        hyperparams,
    ).map_err(|e| ApiError::new("MODEL_CREATION_FAILED", format!("Failed to create model: {}", e)))?;

    Ok((StatusCode::CREATED, Json(ModelResponse {
        model_id: model_id.clone(),
        name: request.name,
        model_type: request.model_type,
        status: "created".to_string(),
        created_at: chrono::Utc::now().timestamp(),
        version: 1,
    })))
}

/// Train a machine learning model
#[utoipa::path(
    post,
    path = "/api/v1/ml/models/{id}/train",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    request_body = TrainModelRequest,
    responses(
        (status = 200, description = "Model trained successfully", body = TrainModelResponse),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn train_model(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(request): Json<TrainModelRequest>,
) -> ApiResult<Json<TrainModelResponse>> {
    let start = std::time::Instant::now();
    let mut engine = ML_ENGINE.write();

    // Prepare training dataset
    let features = request.features.ok_or_else(|| {
        ApiError::new("MISSING_DATA", "Training features are required")
    })?;

    let target = request.target;

    let feature_names = request.feature_names.unwrap_or_else(|| {
        (0..features.get(0).map(|r| r.len()).unwrap_or(0))
            .map(|i| format!("feature_{}", i))
            .collect()
    });

    let dataset = Dataset::new(features, target, feature_names);

    // Validate dataset
    dataset.validate()
        .map_err(|e| ApiError::new("INVALID_DATASET", format!("Dataset validation failed: {}", e)))?;

    // Train model
    let metrics = engine.train_model(&id, dataset)
        .map_err(|e| ApiError::new("TRAINING_FAILED", format!("Model training failed: {}", e)))?;

    let training_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(TrainModelResponse {
        model_id: id,
        status: "trained".to_string(),
        metrics: metrics.all().clone(),
        training_time_ms,
        epochs_completed: request.epochs.unwrap_or(100),
    }))
}

/// Make predictions with a trained model
#[utoipa::path(
    post,
    path = "/api/v1/ml/models/{id}/predict",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    request_body = PredictRequest,
    responses(
        (status = 200, description = "Predictions generated", body = PredictResponse),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn predict(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(request): Json<PredictRequest>,
) -> ApiResult<Json<PredictResponse>> {
    let engine = ML_ENGINE.read();

    // Get inference engine
    let inference_engine = InferenceEngine::new();

    // Make predictions
    let predictions = engine.predict(&id, request.features)
        .map_err(|e| ApiError::new("PREDICTION_FAILED", format!("Prediction failed: {}", e)))?;

    Ok(Json(PredictResponse {
        prediction_count: predictions.len(),
        predictions,
        confidence_scores: None, // Would include confidence for classification models
    }))
}

/// List all models
#[utoipa::path(
    get,
    path = "/api/v1/ml/models",
    responses(
        (status = 200, description = "Models listed", body = ModelListResponse),
    ),
    tag = "ml"
)]
pub async fn list_models(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<ModelListResponse>> {
    let engine = ML_ENGINE.read();

    let models = engine.list_models();

    let summaries: Vec<ModelSummary> = models.iter().map(|m| {
        ModelSummary {
            model_id: m.id.clone(),
            name: m.name.clone(),
            model_type: format!("{:?}", m.model_type),
            status: format!("{:?}", m.status),
            accuracy: m.metrics.get("accuracy"),
            created_at: m.created_at,
        }
    }).collect();

    Ok(Json(ModelListResponse {
        total_count: summaries.len(),
        models: summaries,
    }))
}

/// Get model details
#[utoipa::path(
    get,
    path = "/api/v1/ml/models/{id}",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    responses(
        (status = 200, description = "Model details", body = ModelResponse),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn get_model(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<ModelResponse>> {
    let engine = ML_ENGINE.read();

    let model = engine.get_model(&id)
        .map_err(|_| ApiError::new("NOT_FOUND", format!("Model '{}' not found", id)))?;

    Ok(Json(ModelResponse {
        model_id: model.id.clone(),
        name: model.name.clone(),
        model_type: format!("{:?}", model.model_type),
        status: format!("{:?}", model.status),
        created_at: model.created_at,
        version: model.version,
    }))
}

/// Delete a model
#[utoipa::path(
    delete,
    path = "/api/v1/ml/models/{id}",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    responses(
        (status = 204, description = "Model deleted"),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn delete_model(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut engine = ML_ENGINE.write();

    engine.delete_model(&id)
        .map_err(|_| ApiError::new("NOT_FOUND", format!("Model '{}' not found", id)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get model metrics
#[utoipa::path(
    get,
    path = "/api/v1/ml/models/{id}/metrics",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    responses(
        (status = 200, description = "Model metrics", body = ModelMetricsResponse),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn get_model_metrics(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<ModelMetricsResponse>> {
    let engine = ML_ENGINE.read();

    let model = engine.get_model(&id)
        .map_err(|_| ApiError::new("NOT_FOUND", format!("Model '{}' not found", id)))?;

    Ok(Json(ModelMetricsResponse {
        model_id: id,
        metrics: model.metrics.all().clone(),
        feature_importance: None, // Would compute for tree-based models
    }))
}

/// Evaluate a model on test data
#[utoipa::path(
    post,
    path = "/api/v1/ml/models/{id}/evaluate",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    request_body = ModelEvaluationRequest,
    responses(
        (status = 200, description = "Model evaluated", body = ModelEvaluationResponse),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn evaluate_model(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(request): Json<ModelEvaluationRequest>,
) -> ApiResult<Json<ModelEvaluationResponse>> {
    let engine = ML_ENGINE.read();

    // Make predictions on test data
    let predictions = engine.predict(&id, request.test_features)
        .map_err(|e| ApiError::new("EVALUATION_FAILED", format!("Evaluation failed: {}", e)))?;

    // Calculate metrics (simplified)
    let mut metrics_map = HashMap::new();

    // Calculate MSE for regression
    if predictions.len() == request.test_target.len() {
        let mse: f64 = predictions.iter()
            .zip(request.test_target.iter())
            .map(|(pred, actual)| (pred - actual).powi(2))
            .sum::<f64>() / predictions.len() as f64;

        metrics_map.insert("mse".to_string(), mse);
        metrics_map.insert("rmse".to_string(), mse.sqrt());

        // Calculate R²
        let mean_target = request.test_target.iter().sum::<f64>() / request.test_target.len() as f64;
        let ss_tot: f64 = request.test_target.iter()
            .map(|y| (y - mean_target).powi(2))
            .sum();
        let ss_res: f64 = predictions.iter()
            .zip(request.test_target.iter())
            .map(|(pred, actual)| (actual - pred).powi(2))
            .sum();

        let r2 = 1.0 - (ss_res / ss_tot);
        metrics_map.insert("r2".to_string(), r2);
    }

    Ok(Json(ModelEvaluationResponse {
        model_id: id,
        metrics: metrics_map,
        confusion_matrix: None, // Would compute for classification models
    }))
}

/// Export a trained model
#[utoipa::path(
    get,
    path = "/api/v1/ml/models/{id}/export",
    params(
        ("id" = String, Path, description = "Model ID")
    ),
    responses(
        (status = 200, description = "Model exported"),
        (status = 404, description = "Model not found", body = ApiError),
    ),
    tag = "ml"
)]
pub async fn export_model(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let engine = ML_ENGINE.read();

    let model = engine.get_model(&id)
        .map_err(|_| ApiError::new("NOT_FOUND", format!("Model '{}' not found", id)))?;

    // Serialize model (simplified)
    let export_data = serde_json::json!({
        "model_id": model.id,
        "name": model.name,
        "model_type": format!("{:?}", model.model_type),
        "version": model.version,
        "created_at": model.created_at,
    });

    Ok(Json(export_data))
}
