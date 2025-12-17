// # In-Database Machine Learning Engine
//
// ⚠️ **CRITICAL: DUAL ML IMPLEMENTATION DETECTED** ⚠️
//
// **Issue**: This module (`src/ml_engine/`) duplicates functionality from `src/ml/`
//
// **Duplication Analysis**:
// - Dataset: Both modules have separate Dataset implementations
// - Algorithms: Linear regression, logistic regression, decision trees, k-means all duplicated
// - Training: Separate training infrastructure in both modules
// - Inference: Separate scoring/inference implementations
//
// **Unique to ml_engine/** (preserve during merge):
// - automl.rs: Automated model selection and tuning
// - model_store.rs: Model versioning and storage
// - timeseries.rs: Time series forecasting
//
// **Unique to ml/** (preserve during merge):
// - sql_integration.rs: SQL syntax for ML operations
// - quantization.rs: Model quantization for efficiency
// - simd_ops.rs: SIMD-optimized operations
//
// **TODO - HIGH PRIORITY**:
// 1. Merge src/ml/ and src/ml_engine/ into unified src/ml/ module
// 2. Consolidate duplicate Dataset, Hyperparameters, and Algorithm types
// 3. Merge training infrastructure (ml/engine.rs + ml_engine/training.rs)
// 4. Merge inference infrastructure (ml/inference.rs + ml_engine/scoring.rs)
// 5. Preserve unique features from both modules:
//    - AutoML (from ml_engine)
//    - Model store with versioning (from ml_engine)
//    - Time series (from ml_engine)
//    - SQL integration (from ml)
//    - Quantization & SIMD (from ml)
// 6. Remove this module after merge complete
//
// **Temporary Delegation Pattern** (until merge):
// - For now, this module should be considered the "extended" ML features
// - Core ML operations should delegate to src/ml/ where possible
// - New features should be added to src/ml/ only
//
// **Impact**: 2x code maintenance, ~3000 lines of duplication, API confusion
// **Priority**: BLOCKER - merge before v1.0 release
//
// Production-grade machine learning engine integrated directly with RustyDB's query engine.
// Provides zero-copy data access, GPU acceleration, federated learning, and incremental updates.
//
// ## Architecture
//
// The ML Engine follows a layered architecture:
//
// 1. **Algorithm Layer** - Core ML algorithms (regression, classification, clustering, etc.)
// 2. **Feature Engineering** - Preprocessing, normalization, encoding
// 3. **Training Infrastructure** - Distributed training, mini-batch, early stopping
// 4. **Model Store** - Versioning, serialization, A/B testing
// 5. **Scoring Engine** - Real-time and batch predictions
// 6. **AutoML** - Automated model selection and tuning
// 7. **Time Series** - Forecasting and anomaly detection
//
// ## Key Innovations
//
// - **Zero-Copy Integration**: Direct access to RustyDB's buffer pool without serialization
// - **GPU Acceleration**: CUDA/OpenCL integration for tensor operations
// - **Federated Learning**: Privacy-preserving distributed training
// - **Incremental Updates**: Online learning without full retraining
// - **SQL Integration**: ML operations via standard SQL syntax
//
// ## Example Usage
//
// ```sql
// -- Train a model
// CREATE MODEL churn_predictor
// USING random_forest
// FROM customer_data
// TARGET churn_flag
// WITH (max_depth=10, n_estimators=100);
//
// -- Make predictions
// SELECT customer_id, PREDICT(churn_predictor, *) as churn_prob
// FROM new_customers;
//
// -- AutoML
// CREATE MODEL best_model
// USING automl
// FROM training_data
// TARGET outcome
// WITH (time_budget=3600, metric='auc');
// ```

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub mod algorithms;
pub mod automl;
pub mod features;
pub mod model_store;
pub mod scoring;
pub mod timeseries;
pub mod training;

// ============================================================================
// Core Types
// ============================================================================

// Unique identifier for a machine learning model
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
pub struct ModelId(pub u64);

impl ModelId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

// Machine learning task types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MLTask {
    // Binary or multi-class classification
    Classification,
    // Continuous value prediction
    Regression,
    // Grouping similar data points
    Clustering,
    // Time series forecasting
    TimeSeries,
    // Anomaly detection
    AnomalyDetection,
    // Recommendation systems
    Recommendation,
    // Dimensionality reduction
    DimensionalityReduction,
}

// Supported ML algorithms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum Algorithm {
    LinearRegression,
    LogisticRegression,
    DecisionTree,
    RandomForest,
    GradientBoosting,
    KMeans,
    DBSCAN,
    NaiveBayes,
    SVM,
    NeuralNetwork,
    ARIMA,
    ExponentialSmoothing,
}

impl Algorithm {
    // Get the primary task type for this algorithm
    pub fn task_type(&self) -> MLTask {
        match self {
            Algorithm::LinearRegression => MLTask::Regression,
            Algorithm::LogisticRegression => MLTask::Classification,
            Algorithm::DecisionTree
            | Algorithm::RandomForest
            | Algorithm::GradientBoosting
            | Algorithm::NaiveBayes
            | Algorithm::SVM => MLTask::Classification,
            Algorithm::KMeans | Algorithm::DBSCAN => MLTask::Clustering,
            Algorithm::NeuralNetwork => MLTask::Classification,
            Algorithm::ARIMA | Algorithm::ExponentialSmoothing => MLTask::TimeSeries,
        }
    }
}

// Model hyperparameters
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Hyperparameters {
    params: HashMap<String, HyperparamValue>,
}

impl Hyperparameters {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: HyperparamValue) {
        self.params.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&HyperparamValue> {
        self.params.get(key)
    }

    pub fn get_int(&self, key: &str, default: i64) -> i64 {
        self.params
            .get(key)
            .and_then(|v| {
                if let HyperparamValue::Int(i) = v {
                    Some(*i)
                } else {
                    None
                }
            })
            .unwrap_or(default)
    }

    pub fn get_float(&self, key: &str, default: f64) -> f64 {
        self.params
            .get(key)
            .and_then(|v| {
                if let HyperparamValue::Float(f) = v {
                    Some(*f)
                } else {
                    None
                }
            })
            .unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.params
            .get(key)
            .and_then(|v| {
                if let HyperparamValue::Bool(b) = v {
                    Some(*b)
                } else {
                    None
                }
            })
            .unwrap_or(default)
    }

    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.params
            .get(key)
            .and_then(|v| {
                if let HyperparamValue::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| default.to_string())
    }
}

impl Default for Hyperparameters {
    fn default() -> Self {
        Self::new()
    }
}

// Hyperparameter value types
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum HyperparamValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    IntList(Vec<i64>),
    FloatList(Vec<f64>),
}

// Training dataset representation
#[derive(Debug, Clone)]
pub struct Dataset {
    // Feature matrix (rows = samples, columns = features)
    pub features: Vec<Vec<f64>>,
    // Target values (for supervised learning)
    pub targets: Option<Vec<f64>>,
    // Feature names
    pub feature_names: Vec<String>,
    // Target name (for supervised learning)
    pub target_name: Option<String>,
    // Sample weights
    pub weights: Option<Vec<f64>>,
}

impl Dataset {
    pub fn new(features: Vec<Vec<f64>>, feature_names: Vec<String>) -> Self {
        Self {
            features,
            targets: None,
            feature_names,
            target_name: None,
            weights: None,
        }
    }

    pub fn with_targets(mut self, targets: Vec<f64>, target_name: String) -> Self {
        self.targets = Some(targets);
        self.target_name = Some(target_name);
        self
    }

    pub fn with_weights(mut self, weights: Vec<f64>) -> Self {
        self.weights = Some(weights);
        self
    }

    pub fn num_samples(&self) -> usize {
        self.features.len()
    }

    pub fn num_features(&self) -> usize {
        if self.features.is_empty() {
            0
        } else {
            self.features[0].len()
        }
    }

    pub fn get_sample(&self, idx: usize) -> Option<&[f64]> {
        self.features.get(idx).map(|v| v.as_slice())
    }

    pub fn get_target(&self, idx: usize) -> Option<f64> {
        self.targets.as_ref().and_then(|t| t.get(idx).copied())
    }
}

// Model training statistics
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct TrainingStats {
    // Number of training samples
    pub num_samples: usize,
    // Number of features
    pub num_features: usize,
    // Training time in seconds
    pub training_time: f64,
    // Number of iterations/epochs
    pub iterations: usize,
    // Final training loss
    pub final_loss: f64,
    // Validation metrics
    pub validation_metrics: HashMap<String, f64>,
    // Convergence information
    pub converged: bool,
}

// Model evaluation metrics
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct EvaluationMetrics {
    // Mean Squared Error (regression)
    pub mse: Option<f64>,
    // Root Mean Squared Error (regression)
    pub rmse: Option<f64>,
    // R² score (regression)
    pub r2: Option<f64>,
    // Mean Absolute Error (regression)
    pub mae: Option<f64>,
    // Accuracy (classification)
    pub accuracy: Option<f64>,
    // Precision (classification)
    pub precision: Option<f64>,
    // Recall (classification)
    pub recall: Option<f64>,
    // F1 Score (classification)
    pub f1: Option<f64>,
    // AUC-ROC (classification)
    pub auc: Option<f64>,
    // Log Loss (classification)
    pub log_loss: Option<f64>,
    // Silhouette Score (clustering)
    pub silhouette: Option<f64>,
    // Davies-Bouldin Index (clustering)
    pub davies_bouldin: Option<f64>,
}

impl EvaluationMetrics {
    pub fn new() -> Self {
        Self {
            mse: None,
            rmse: None,
            r2: None,
            mae: None,
            accuracy: None,
            precision: None,
            recall: None,
            f1: None,
            auc: None,
            log_loss: None,
            silhouette: None,
            davies_bouldin: None,
        }
    }

    pub fn set_regression_metrics(&mut self, mse: f64, r2: f64, mae: f64) {
        self.mse = Some(mse);
        self.rmse = Some(mse.sqrt());
        self.r2 = Some(r2);
        self.mae = Some(mae);
    }

    pub fn set_classification_metrics(&mut self, accuracy: f64, precision: f64, recall: f64) {
        self.accuracy = Some(accuracy);
        self.precision = Some(precision);
        self.recall = Some(recall);
        self.f1 = Some(2.0 * precision * recall / (precision + recall + 1e-10));
    }
}

impl Default for EvaluationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// GPU acceleration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    // Enable GPU acceleration
    pub enabled: bool,
    // CUDA device ID
    pub device_id: i32,
    // Batch size for GPU operations
    pub batch_size: usize,
    // Use mixed precision (FP16)
    pub mixed_precision: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            device_id: 0,
            batch_size: 256,
            mixed_precision: false,
        }
    }
}

// Federated learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    // Enable federated learning
    pub enabled: bool,
    // Number of federated nodes
    pub num_nodes: usize,
    // Aggregation strategy
    pub aggregation: AggregationStrategy,
    // Differential privacy epsilon
    pub dp_epsilon: Option<f64>,
    // Communication rounds
    pub rounds: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationStrategy {
    // Federated Averaging
    FedAvg,
    // Federated Proximal
    FedProx,
    // Secure Aggregation
    SecureAgg,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            num_nodes: 1,
            aggregation: AggregationStrategy::FedAvg,
            dp_epsilon: None,
            rounds: 10,
        }
    }
}

// Model prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    // Predicted value or class
    pub value: f64,
    // Prediction confidence/probability
    pub confidence: f64,
    // Class probabilities (for classification)
    pub class_probabilities: Option<Vec<f64>>,
    // Prediction interval (for regression)
    pub prediction_interval: Option<(f64, f64)>,
    // Feature contributions (SHAP-like)
    pub feature_contributions: Option<HashMap<String, f64>>,
}

impl Prediction {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            confidence: 1.0,
            class_probabilities: None,
            prediction_interval: None,
            feature_contributions: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_class_probabilities(mut self, probs: Vec<f64>) -> Self {
        self.class_probabilities = Some(probs);
        self
    }

    pub fn with_interval(mut self, lower: f64, upper: f64) -> Self {
        self.prediction_interval = Some((lower, upper));
        self
    }

    pub fn with_feature_contributions(mut self, contributions: HashMap<String, f64>) -> Self {
        self.feature_contributions = Some(contributions);
        self
    }
}

// ============================================================================
// ML Engine Orchestrator
// ============================================================================

// Main ML Engine coordinator
pub struct MLEngine {
    // Model registry
    #[allow(dead_code)]
    model_store: Arc<RwLock<model_store::ModelStore>>,
    // Feature engineering pipeline
    #[allow(dead_code)]
    feature_engine: Arc<RwLock<features::FeatureEngine>>,
    // Training coordinator
    #[allow(dead_code)]
    training_engine: Arc<RwLock<training::TrainingEngine>>,
    // Scoring engine
    #[allow(dead_code)]
    scoring_engine: Arc<RwLock<scoring::ScoringEngine>>,
    // AutoML coordinator
    #[allow(dead_code)]
    automl_engine: Arc<RwLock<automl::AutoMLEngine>>,
    // Time series analyzer
    #[allow(dead_code)]
    timeseries_engine: Arc<RwLock<timeseries::TimeSeriesEngine>>,
    // GPU configuration
    #[allow(dead_code)]
    gpu_config: GpuConfig,
    // Federated learning configuration
    #[allow(dead_code)]
    federated_config: FederatedConfig,
}

impl MLEngine {
    // Create a new ML Engine instance
    pub fn new() -> Self {
        Self {
            model_store: Arc::new(RwLock::new(model_store::ModelStore::new())),
            feature_engine: Arc::new(RwLock::new(features::FeatureEngine::new())),
            training_engine: Arc::new(RwLock::new(training::TrainingEngine::new())),
            scoring_engine: Arc::new(RwLock::new(scoring::ScoringEngine::new())),
            automl_engine: Arc::new(RwLock::new(automl::AutoMLEngine::new())),
            timeseries_engine: Arc::new(RwLock::new(timeseries::TimeSeriesEngine::new())),
            gpu_config: GpuConfig::default(),
            federated_config: FederatedConfig::default(),
        }
    }

    // Configure GPU acceleration
    pub fn with_gpu(mut self, config: GpuConfig) -> Self {
        self.gpu_config = config;
        self
    }

    // Configure federated learning
    pub fn with_federated(mut self, config: FederatedConfig) -> Self {
        self.federated_config = config;
        self
    }

    // Train a new model
    pub fn train_model(
        &self,
        algorithm: Algorithm,
        dataset: Dataset,
        hyperparams: Hyperparameters,
    ) -> Result<ModelId> {
        let training_engine = self.training_engine.read().map_err(|_| {
            crate::DbError::Internal("Failed to acquire training engine lock".into())
        })?;

        let model = training_engine.train(algorithm, dataset, hyperparams, &self.gpu_config)?;

        let mut model_store = self
            .model_store
            .write()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        model_store.register_model(model)
    }

    // Make predictions using a trained model
    pub fn predict(&self, model_id: ModelId, features: Vec<Vec<f64>>) -> Result<Vec<Prediction>> {
        let model_store = self
            .model_store
            .read()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        let model = model_store.get_model(model_id)?;

        let scoring_engine = self.scoring_engine.read().map_err(|_| {
            crate::DbError::Internal("Failed to acquire scoring engine lock".into())
        })?;

        scoring_engine.predict(model, features)
    }

    // Perform AutoML to find the best model
    pub fn automl(&self, dataset: Dataset, task: MLTask, time_budget: u64) -> Result<ModelId> {
        let automl_engine = self
            .automl_engine
            .read()
            .map_err(|_| crate::DbError::Internal("Failed to acquire AutoML engine lock".into()))?;

        let best_model = automl_engine.find_best_model(dataset, task, time_budget)?;

        let mut model_store = self
            .model_store
            .write()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        model_store.register_model(best_model)
    }

    // Perform time series forecasting
    pub fn forecast(
        &self,
        series: Vec<f64>,
        horizon: usize,
        algorithm: Algorithm,
    ) -> Result<Vec<f64>> {
        let timeseries_engine = self.timeseries_engine.read().map_err(|_| {
            crate::DbError::Internal("Failed to acquire time series engine lock".into())
        })?;

        timeseries_engine.forecast(series, horizon, algorithm)
    }

    // Get model information
    pub fn get_model_info(&self, model_id: ModelId) -> Result<model_store::ModelMetadata> {
        let model_store = self
            .model_store
            .read()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        model_store.get_metadata(model_id)
    }

    // List all models
    pub fn list_models(&self) -> Result<Vec<model_store::ModelMetadata>> {
        let model_store = self
            .model_store
            .read()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        Ok(model_store.list_models())
    }

    // Delete a model
    pub fn delete_model(&self, model_id: ModelId) -> Result<()> {
        let mut model_store = self
            .model_store
            .write()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        model_store.delete_model(model_id)
    }

    // Export model to PMML
    pub fn export_pmml(&self, model_id: ModelId) -> Result<String> {
        let scoring_engine = self.scoring_engine.read().map_err(|_| {
            crate::DbError::Internal("Failed to acquire scoring engine lock".into())
        })?;

        let model_store = self
            .model_store
            .read()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        let model = model_store.get_model(model_id)?;

        scoring_engine.export_pmml(model)
    }

    // Import model from PMML
    pub fn import_pmml(&self, pmml: &str) -> Result<ModelId> {
        let scoring_engine = self.scoring_engine.read().map_err(|_| {
            crate::DbError::Internal("Failed to acquire scoring engine lock".into())
        })?;

        let model = scoring_engine.import_pmml(pmml)?;

        let mut model_store = self
            .model_store
            .write()
            .map_err(|_| crate::DbError::Internal("Failed to acquire model store lock".into()))?;

        model_store.register_model(model)
    }
}

impl Default for MLEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id() {
        let id1 = ModelId::new(1);
        let id2 = ModelId::new(1);
        let id3 = ModelId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_hyperparameters() {
        let mut params = Hyperparameters::new();
        params.set("learning_rate", HyperparamValue::Float(0.01));
        params.set("max_depth", HyperparamValue::Int(10));
        params.set("verbose", HyperparamValue::Bool(true));

        assert_eq!(params.get_float("learning_rate", 0.0), 0.01);
        assert_eq!(params.get_int("max_depth", 0), 10);
        assert_eq!(params.get_bool("verbose", false), true);
    }

    #[test]
    fn test_dataset() {
        let features = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let feature_names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];
        let targets = vec![0.0, 1.0];

        let dataset =
            Dataset::new(features, feature_names).with_targets(targets, "target".to_string());

        assert_eq!(dataset.num_samples(), 2);
        assert_eq!(dataset.num_features(), 3);
        assert_eq!(dataset.get_target(0), Some(0.0));
        assert_eq!(dataset.get_target(1), Some(1.0));
    }

    #[test]
    fn test_prediction() {
        let pred = Prediction::new(0.8)
            .with_confidence(0.95)
            .with_interval(0.7, 0.9);

        assert_eq!(pred.value, 0.8);
        assert_eq!(pred.confidence, 0.95);
        assert_eq!(pred.prediction_interval, Some((0.7, 0.9)));
    }

    #[test]
    fn test_ml_engine_creation() {
        let engine = MLEngine::new();
        let models = engine.list_models().unwrap();
        assert_eq!(models.len(), 0);
    }
}
