// # Machine Learning Algorithms
//
// Pure Rust implementations of common ML algorithms for in-database training and inference.
// All algorithms are designed to work efficiently with streaming data and large datasets.
//
// ## Module Organization
//
// - `regression`: Linear regression algorithms
// - `classification`: Logistic regression and Naive Bayes
// - `trees`: Decision tree and random forest implementations
// - `clustering`: K-Means and other clustering algorithms
// - `neural_networks`: Neural network implementations (placeholder)

pub mod regression;
pub mod classification;
pub mod trees;
pub mod clustering;
pub mod neural_networks;

use crate::error::Result;
use super::{Dataset, Vector, Matrix, Hyperparameters};

// Re-export algorithm implementations
pub use regression::LinearRegression;
pub use classification::{LogisticRegression, NaiveBayes};
pub use trees::{DecisionTree, RandomForest};
pub use clustering::KMeansClustering;
pub use neural_networks::NeuralNetwork;

// ============================================================================
// Common Types and Traits
// ============================================================================

/// Supported model types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModelType {
    LinearRegression,
    LogisticRegression,
    DecisionTree,
    RandomForest,
    KMeans,
    NaiveBayes,
}

impl ModelType {
    /// Get default hyperparameters for this model type
    pub fn default_hyperparameters(&self) -> Hyperparameters {
        let mut params = Hyperparameters::new();
        match self {
            ModelType::LinearRegression => {
                params.set_float("learning_rate", 0.01);
                params.set_int("max_iterations", 1000);
                params.set_float("tolerance", 1e-6);
                params.set_bool("fit_intercept", true);
            }
            ModelType::LogisticRegression => {
                params.set_float("learning_rate", 0.01);
                params.set_int("max_iterations", 1000);
                params.set_float("tolerance", 1e-6);
                params.set_float("regularization", 0.01);
                params.set_bool("fit_intercept", true);
            }
            ModelType::DecisionTree => {
                params.set_int("max_depth", 10);
                params.set_int("min_samples_split", 2);
                params.set_int("min_samples_leaf", 1);
                params.set_string("criterion", "gini".to_string());
            }
            ModelType::RandomForest => {
                params.set_int("n_estimators", 100);
                params.set_int("max_depth", 10);
                params.set_int("min_samples_split", 2);
                params.set_int("min_samples_leaf", 1);
                params.set_float("max_features_ratio", 0.7);
            }
            ModelType::KMeans => {
                params.set_int("n_clusters", 3);
                params.set_int("max_iterations", 300);
                params.set_float("tolerance", 1e-4);
                params.set_string("init_method", "kmeans++".to_string());
            }
            ModelType::NaiveBayes => {
                params.set_float("alpha", 1.0); // Laplace smoothing
                params.set_bool("fit_prior", true);
            }
        }
        params
    }
}

/// Common trait for all ML algorithms
pub trait Algorithm: Send + Sync {
    /// Train the model on the given dataset
    fn fit(&mut self, dataset: &Dataset, params: &Hyperparameters) -> Result<()>;

    /// Make predictions on new data
    fn predict(&self, features: &Matrix) -> Result<Vector>;

    /// Get model type
    fn model_type(&self) -> ModelType;

    /// Serialize model to bytes
    fn serialize(&self) -> Result<Vec<u8>>;

    /// Deserialize model from bytes
    fn deserialize(bytes: &[u8]) -> Result<Self> where Self: Sized;

    /// Calculate feature importance (if supported)
    fn feature_importance(&self) -> Option<Vector> {
        None
    }
}
