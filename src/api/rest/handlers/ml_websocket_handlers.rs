// ML WebSocket Handlers
//
// WebSocket handlers for real-time ML operations including:
// - Model training progress (epochs, loss, accuracy)
// - Real-time predictions streaming
// - Model lifecycle events
// - AutoML progress updates

use axum::{
    extract::{Path, State, ws::{WebSocket, WebSocketUpgrade}},
    response::Response,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;
use futures::{StreamExt, SinkExt};
use tokio::time::{interval, Duration};

use super::super::types::ApiState;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MLWebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrainingProgressRequest {
    pub model_name: String,
    pub algorithm: String,
    pub hyperparameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrainingProgressUpdate {
    pub epoch: usize,
    pub total_epochs: usize,
    pub loss: f64,
    pub accuracy: Option<f64>,
    pub validation_loss: Option<f64>,
    pub validation_accuracy: Option<f64>,
    pub learning_rate: f64,
    pub elapsed_seconds: f64,
    pub estimated_remaining_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PredictionStreamRequest {
    pub model_id: String,
    pub batch_size: Option<usize>,
    pub features: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PredictionResult {
    pub prediction_id: String,
    pub value: f64,
    pub confidence: f64,
    pub class_probabilities: Option<Vec<f64>>,
    pub feature_importance: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutoMLProgressRequest {
    pub task_type: String, // classification, regression, clustering
    pub time_budget_seconds: u64,
    pub metric: String, // accuracy, auc, rmse, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutoMLProgressUpdate {
    pub iteration: usize,
    pub algorithm_tested: String,
    pub score: f64,
    pub best_score: f64,
    pub elapsed_seconds: f64,
    pub remaining_budget_seconds: f64,
    pub models_tested: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelLifecycleEvent {
    pub event_type: String, // created, updated, deleted, deployed, deprecated
    pub model_id: String,
    pub model_name: String,
    pub version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// WebSocket Handlers
// ============================================================================

/// WebSocket endpoint for model training progress
///
/// Streams real-time training updates including epoch progress, loss, accuracy,
/// and estimated time remaining. Useful for monitoring long-running training jobs.
#[utoipa::path(
    get,
    path = "/api/v1/ws/ml/training",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "ml-websocket"
)]
pub async fn ws_ml_training_progress(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_training_progress_websocket(socket, state))
}

/// WebSocket endpoint for real-time predictions
///
/// Streams predictions as they are computed. Useful for batch prediction jobs
/// or real-time inference with large datasets.
#[utoipa::path(
    get,
    path = "/api/v1/ws/ml/predictions",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "ml-websocket"
)]
pub async fn ws_ml_predictions(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_predictions_websocket(socket, state))
}

/// WebSocket endpoint for AutoML progress
///
/// Streams AutoML iteration updates including algorithms tested, scores,
/// and best model found so far.
#[utoipa::path(
    get,
    path = "/api/v1/ws/ml/automl",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "ml-websocket"
)]
pub async fn ws_ml_automl_progress(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_automl_progress_websocket(socket, state))
}

/// WebSocket endpoint for model lifecycle events
///
/// Streams events for model creation, updates, deletions, and deployments
/// across the entire ML model registry.
#[utoipa::path(
    get,
    path = "/api/v1/ws/ml/lifecycle",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
    ),
    tag = "ml-websocket"
)]
pub async fn ws_ml_lifecycle_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_lifecycle_events_websocket(socket, state))
}

// ============================================================================
// WebSocket Connection Handlers
// ============================================================================

/// Training progress WebSocket handler
async fn handle_training_progress_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    // Send welcome message
    let welcome = MLWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to ML Training Progress Stream",
            "supported_algorithms": ["linear_regression", "logistic_regression", "random_forest", "neural_network"]
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        model_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    // Handle incoming training requests
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<TrainingProgressRequest>(&text) {
                        // Simulate training progress
                        let total_epochs = 100;
                        let model_id = format!("model_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

                        for epoch in 1..=total_epochs {
                            let progress = TrainingProgressUpdate {
                                epoch,
                                total_epochs,
                                loss: 1.0 / (epoch as f64 + 1.0), // Simulated decreasing loss
                                accuracy: Some(1.0 - 1.0 / (epoch as f64 + 1.0)),
                                validation_loss: Some(1.2 / (epoch as f64 + 1.0)),
                                validation_accuracy: Some(1.0 - 1.2 / (epoch as f64 + 1.0)),
                                learning_rate: 0.01,
                                elapsed_seconds: epoch as f64 * 0.5,
                                estimated_remaining_seconds: Some((total_epochs - epoch) as f64 * 0.5),
                            };

                            let message = MLWebSocketMessage {
                                message_type: "training_progress".to_string(),
                                data: serde_json::to_value(&progress).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                model_id: Some(model_id.clone()),
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            // Simulate training time
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        // Send completion message
                        let completion = MLWebSocketMessage {
                            message_type: "training_complete".to_string(),
                            data: json!({
                                "model_id": model_id,
                                "model_name": request.model_name,
                                "final_loss": 0.01,
                                "final_accuracy": 0.99,
                                "total_epochs": total_epochs
                            }),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            model_id: Some(model_id),
                        };

                        if let Ok(completion_json) = serde_json::to_string(&completion) {
                            if socket.send(Message::Text(completion_json.into())).await.is_err() {
                                return;
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Predictions WebSocket handler
async fn handle_predictions_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    // Send welcome message
    let welcome = MLWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to ML Predictions Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        model_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    // Handle prediction requests
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<PredictionStreamRequest>(&text) {
                        // Stream predictions for each feature set
                        for (idx, _features) in request.features.iter().enumerate() {
                            let result = PredictionResult {
                                prediction_id: format!("pred_{}", idx),
                                value: 0.5 + (idx as f64 * 0.1) % 0.5, // Simulated prediction
                                confidence: 0.85 + (idx as f64 * 0.01) % 0.15,
                                class_probabilities: Some(vec![0.3, 0.7]),
                                feature_importance: Some({
                                    let mut importance = std::collections::HashMap::new();
                                    importance.insert("feature_0".to_string(), 0.4);
                                    importance.insert("feature_1".to_string(), 0.6);
                                    importance
                                }),
                            };

                            let message = MLWebSocketMessage {
                                message_type: "prediction".to_string(),
                                data: serde_json::to_value(&result).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                model_id: Some(request.model_id.clone()),
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }

                        // Send completion message
                        let completion = MLWebSocketMessage {
                            message_type: "predictions_complete".to_string(),
                            data: json!({
                                "total_predictions": request.features.len(),
                                "model_id": request.model_id
                            }),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            model_id: Some(request.model_id),
                        };

                        if let Ok(completion_json) = serde_json::to_string(&completion) {
                            let _ = socket.send(Message::Text(completion_json.into())).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// AutoML progress WebSocket handler
async fn handle_automl_progress_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    // Send welcome message
    let welcome = MLWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to AutoML Progress Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        model_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    // Handle AutoML requests
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<AutoMLProgressRequest>(&text) {
                        let algorithms = vec![
                            "linear_regression", "decision_tree", "random_forest",
                            "gradient_boosting", "neural_network"
                        ];
                        let mut best_score = 0.0;

                        for (idx, algorithm) in algorithms.iter().enumerate() {
                            let score = 0.5 + (idx as f64 * 0.1);
                            if score > best_score {
                                best_score = score;
                            }

                            let progress = AutoMLProgressUpdate {
                                iteration: idx + 1,
                                algorithm_tested: algorithm.to_string(),
                                score,
                                best_score,
                                elapsed_seconds: (idx + 1) as f64 * 10.0,
                                remaining_budget_seconds: request.time_budget_seconds as f64 - (idx + 1) as f64 * 10.0,
                                models_tested: idx + 1,
                            };

                            let message = MLWebSocketMessage {
                                message_type: "automl_progress".to_string(),
                                data: serde_json::to_value(&progress).unwrap(),
                                timestamp: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64,
                                model_id: None,
                            };

                            if let Ok(message_json) = serde_json::to_string(&message) {
                                if socket.send(Message::Text(message_json.into())).await.is_err() {
                                    return;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }

                        // Send completion message
                        let completion = MLWebSocketMessage {
                            message_type: "automl_complete".to_string(),
                            data: json!({
                                "best_algorithm": "neural_network",
                                "best_score": best_score,
                                "models_tested": algorithms.len(),
                                "metric": request.metric
                            }),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            model_id: None,
                        };

                        if let Ok(completion_json) = serde_json::to_string(&completion) {
                            let _ = socket.send(Message::Text(completion_json.into())).await;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    if socket.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Model lifecycle events WebSocket handler
async fn handle_lifecycle_events_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    // Send welcome message
    let welcome = MLWebSocketMessage {
        message_type: "welcome".to_string(),
        data: json!({
            "message": "Connected to Model Lifecycle Events Stream"
        }),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        model_id: None,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_json.into())).await.is_err() {
            return;
        }
    }

    // Simulate periodic lifecycle events
    let mut ticker = interval(Duration::from_secs(5));
    let event_types = vec!["created", "updated", "deployed", "deprecated"];
    let mut event_idx = 0;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let event = ModelLifecycleEvent {
                    event_type: event_types[event_idx % event_types.len()].to_string(),
                    model_id: format!("model_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
                    model_name: format!("prediction_model_{}", event_idx),
                    version: Some(format!("v1.{}", event_idx)),
                    metadata: Some(json!({
                        "algorithm": "random_forest",
                        "accuracy": 0.95
                    })),
                };

                let message = MLWebSocketMessage {
                    message_type: "lifecycle_event".to_string(),
                    data: serde_json::to_value(&event).unwrap(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    model_id: Some(event.model_id.clone()),
                };

                if let Ok(message_json) = serde_json::to_string(&message) {
                    if socket.send(Message::Text(message_json.into())).await.is_err() {
                        break;
                    }
                }

                event_idx += 1;
            }
            msg = socket.recv() => {
                if let Some(Ok(msg)) = msg {
                    match msg {
                        Message::Close(_) => break,
                        Message::Ping(data) => {
                            if socket.send(Message::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
        }
    }
}
