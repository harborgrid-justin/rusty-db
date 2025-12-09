// # In-Database Machine Learning Engine
//
// This module provides comprehensive machine learning capabilities directly within RustyDB,
// enabling model training, inference, and management without data export.
//
// ## Architecture Overview
//
// The ML engine is organized into several key components:
//
// - **engine**: Core ML engine orchestrating all ML operations, model registry, and lifecycle
// - **algorithms**: Pure Rust implementations of ML algorithms (regression, classification, clustering)
// - **preprocessing**: Data preparation, feature engineering, and transformation utilities
// - **inference**: Real-time and batch prediction with optimized execution paths
// - **sql_integration**: SQL syntax extensions for ML operations (CREATE MODEL, PREDICT, etc.)
//
// ## Key Features
//
// - Pure Rust ML implementations with no external ML library dependencies
// - Efficient in-database training on large datasets using streaming algorithms
// - Model versioning and A/B testing support
// - Real-time and batch inference with low latency
// - SQL-native syntax for ML operations
// - Automatic feature detection and engineering
// - Model performance monitoring and drift detection
//
// ## Usage Examples
//
// ### Training a Model via SQL
//
// ```sql
// CREATE MODEL customer_churn_predictor
// USING logistic_regression
// AS SELECT
//     customer_age,
//     account_balance,
//     num_products,
//     is_active_member,
//     churn as target
// FROM customers;
// ```
//
// ### Making Predictions
//
// ```sql
// SELECT
//     customer_id,
//     PREDICT(customer_churn_predictor,
//             customer_age,
//             account_balance,
//             num_products,
//             is_active_member) as churn_probability
// FROM new_customers;
// ```
//
// ## Module Organization
//
// **Target LOC:** 3,000+ lines across all submodules

use std::fmt;
pub mod engine;
pub mod algorithms;
pub mod preprocessing;
pub mod inference;
pub mod sql_integration;
pub mod optimizers;
pub mod simd_ops;
pub mod quantization;

// Re-export key types for convenience
pub use engine::{MLEngine, ModelRegistry, ModelMetadata, TrainingJob, ModelVersion, ModelStatus, StoredModel};
pub use algorithms::{
    Algorithm, LinearRegression, LogisticRegression, DecisionTree, RandomForest,
    KMeansClustering, NaiveBayes, ModelType,
};
pub use preprocessing::{
    Preprocessor, Scaler, StandardScaler, MinMaxScaler, Encoder, OneHotEncoder,
    FeatureSelector, DataSplitter, ImputationStrategy,
};
pub use inference::{
    InferenceEngine, PredictionResult, BatchPredictor, ModelCache,
    FeatureImportance, ConfidenceScore,
};
pub use sql_integration::{
    MLSqlParser, CreateModelStatement, PredictFunction, ModelTable,
};
pub use optimizers::{
    Optimizer, SGDMomentum, AdamOptimizer, LRScheduler, LRSchedule, OptimizerType,
};
pub use simd_ops::{
    simd_dot_product, simd_matrix_vector_multiply, simd_euclidean_distance,
};
pub use quantization::{
    QuantizedWeights, QuantizationConfig, QuantizationMethod, QuantizedLinearModel,
    quantize_weights, dequantize_weights,
};

use crate::error::{Result, DbError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// ML-specific error types
#[derive(Debug, Clone)]
pub enum MLError {
    /// Model not found in registry
    ModelNotFound(String),
    /// Invalid model configuration
    InvalidConfiguration(String),
    /// Training failed
    TrainingFailed(String),
    /// Prediction failed
    PredictionFailed(String),
    /// Feature mismatch
    FeatureMismatch { expected: usize, got: usize },
    /// Insufficient data for training
    InsufficientData(String),
    /// Algorithm not supported
    UnsupportedAlgorithm(String),
    /// Model version conflict
    VersionConflict(String),
    /// Invalid hyperparameters
    InvalidHyperparameters(String),
}

impl std::fmt::Display for MLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MLError::ModelNotFound(name) => write!(f, "Model not found: {}", name),
            MLError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            MLError::TrainingFailed(msg) => write!(f, "Training failed: {}", msg),
            MLError::PredictionFailed(msg) => write!(f, "Prediction failed: {}", msg),
            MLError::FeatureMismatch { expected, got } => {
                write!(f, "Feature mismatch: expected {} features, got {}", expected, got)
            }
            MLError::InsufficientData(msg) => write!(f, "Insufficient data: {}", msg),
            MLError::UnsupportedAlgorithm(algo) => write!(f, "Unsupported algorithm: {}", algo),
            MLError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
            MLError::InvalidHyperparameters(msg) => write!(f, "Invalid hyperparameters: {}", msg),
        }
    }
}

impl std::error::Error for MLError {}

/// Convert MLError to DbError
impl From<MLError> for DbError {
    fn from(err: MLError) -> Self {
        DbError::Execution(err.to_string())
    }
}

/// Matrix type for ML operations
pub type Matrix = Vec<Vec<f64>>;

/// Vector type for ML operations
pub type Vector = Vec<f64>;

/// Feature names
pub type FeatureNames = Vec<String>;

/// Training dataset
#[derive(Debug, Clone)]
pub struct Dataset {
    /// Feature matrix (rows = samples, columns = features)
    pub features: Matrix,
    /// Target values (for supervised learning)
    pub target: Option<Vector>,
    /// Feature names
    pub feature_names: FeatureNames,
    /// Sample weights (optional)
    pub weights: Option<Vector>,
}

impl Dataset {
    /// Create a new dataset
    pub fn new(features: Matrix, target: Option<Vector>, feature_names: FeatureNames) -> Self {
        Self {
            features,
            target,
            feature_names,
            weights: None,
        }
    }

    /// Get number of samples
    pub fn num_samples(&self) -> usize {
        self.features.len()
    }

    /// Get number of features
    pub fn num_features(&self) -> usize {
        self.features.get(0).map(|row| row.len()).unwrap_or(0)
    }

    /// Validate dataset consistency
    pub fn validate(&self) -> Result<()> {
        if self.features.is_empty() {
            return Err(MLError::InsufficientData("Empty feature matrix".to_string()).into());
        }

        let num_features = self.num_features();
        for (i, row) in self.features.iter().enumerate() {
            if row.len() != num_features {
                return Err(MLError::InvalidConfiguration(
                    format!("Inconsistent feature count at row {}: expected {}, got {}",
                            i, num_features, row.len())
                ).into());
            }
        }

        if let Some(ref target) = self.target {
            if target.len() != self.num_samples() {
                return Err(MLError::FeatureMismatch {
                    expected: self.num_samples(),
                    got: target.len(),
                }.into());
            }
        }

        if let Some(ref weights) = self.weights {
            if weights.len() != self.num_samples() {
                return Err(MLError::InvalidConfiguration(
                    format!("Weight vector length {} doesn't match sample count {}",
                            weights.len(), self.num_samples())
                ).into());
            }
        }

        Ok(())
    }

    /// Add sample weights
    pub fn with_weights(mut self, weights: Vector) -> Self {
        self.weights = Some(weights);
        self
    }
}

/// Hyperparameters for ML algorithms
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Hyperparameters {
    params: HashMap<String, HyperparameterValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperparameterValue {
    Float(f64),
    Int(i64),
    String(String),
    Bool(bool),
}

impl Hyperparameters {
    /// Create new hyperparameters
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Set a float parameter
    pub fn set_float(&mut self, key: &str, value: f64) {
        self.params.insert(key.to_string(), HyperparameterValue::Float(value));
    }

    /// Set an integer parameter
    pub fn set_int(&mut self, key: &str, value: i64) {
        self.params.insert(key.to_string(), HyperparameterValue::Int(value));
    }

    /// Set a string parameter
    pub fn set_string(&mut self, key: &str, value: String) {
        self.params.insert(key.to_string(), HyperparameterValue::String(value));
    }

    /// Set a boolean parameter
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.params.insert(key.to_string(), HyperparameterValue::Bool(value));
    }

    /// Get a float parameter
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.params.get(key) {
            Some(HyperparameterValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get an integer parameter
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.params.get(key) {
            Some(HyperparameterValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a string parameter
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.params.get(key) {
            Some(HyperparameterValue::String(v)) => Some(v),
            _ => None,
        }
    }

    /// Get a boolean parameter
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.params.get(key) {
            Some(HyperparameterValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }
}

/// Performance metrics for model evaluation
#[derive(Debug, Clone)]
pub struct Metrics {
    /// Metric values
    values: HashMap<String, f64>,
}

impl Metrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Set a metric value
    pub fn set(&mut self, name: &str, value: f64) {
        self.values.insert(name.to_string(), value);
    }

    /// Get a metric value
    pub fn get(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }

    /// Get all metrics
    pub fn all(&self) -> &HashMap<String, f64> {
        &self.values
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_creation() {
        let features = vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
        ];
        let target = Some(vec![0.0, 1.0]);
        let feature_names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];

        let dataset = Dataset::new(features, target, feature_names);
        assert_eq!(dataset.num_samples(), 2);
        assert_eq!(dataset.num_features(), 3);
    }

    #[test]
    fn test_dataset_validation() {
        let features = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
        ];
        let target = Some(vec![0.0, 1.0]);
        let feature_names = vec!["f1".to_string(), "f2".to_string()];

        let dataset = Dataset::new(features, target, feature_names);
        assert!(dataset.validate().is_ok());
    }

    #[test]
    fn test_hyperparameters() {
        let mut params = Hyperparameters::new();
        params.set_float("learning_rate", 0.01);
        params.set_int("max_iter", 100);
        params.set_bool("verbose", true);

        assert_eq!(params.get_float("learning_rate"), Some(0.01));
        assert_eq!(params.get_int("max_iter"), Some(100));
        assert_eq!(params.get_bool("verbose"), Some(true));
    }
}
