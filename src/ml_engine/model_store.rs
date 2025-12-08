//! # Model Store and Management
//!
//! Model versioning, serialization, registry, and deployment pipeline.
//! Supports A/B testing and production model management.

use crate::error::Result;
use super::{Algorithm, ModelId, Hyperparameters, TrainingStats, EvaluationMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime};

// ============================================================================
// Model Definition
// ============================================================================

/// A trained machine learning model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Unique model identifier
    pub id: ModelId,
    /// Model name
    pub name: String,
    /// Algorithm type
    pub algorithm: Algorithm,
    /// Model parameters/weights
    pub parameters: ModelParameters,
    /// Hyperparameters used for training
    pub hyperparameters: Hyperparameters,
    /// Training statistics
    pub training_stats: TrainingStats,
    /// Evaluation metrics
    pub metrics: EvaluationMetrics,
    /// Model version
    pub version: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Model status
    pub status: ModelStatus,
    /// Tags for organization
    pub tags: Vec<String>,
}

impl Model {
    pub fn new(
        id: ModelId,
        name: String,
        algorithm: Algorithm,
        parameters: ModelParameters,
        hyperparameters: Hyperparameters,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            name,
            algorithm,
            parameters,
            hyperparameters,
            training_stats: TrainingStats {
                num_samples: 0,
                num_features: 0,
                training_time: 0.0,
                iterations: 0,
                final_loss: 0.0,
                validation_metrics: HashMap::new(),
                converged: false,
            },
            metrics: EvaluationMetrics::new(),
            version: 1,
            created_at: now,
            updated_at: now,
            status: ModelStatus::Training,
            tags: Vec::new(),
        }
    }

    pub fn with_stats(mut self, stats: TrainingStats) -> Self {
        self.training_stats = stats;
        self
    }

    pub fn with_metrics(mut self, metrics: EvaluationMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn mark_ready(&mut self) {
        self.status = ModelStatus::Ready;
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn deploy(&mut self, deployment: DeploymentConfig) -> Result<()> {
        if self.status != ModelStatus::Ready {
            return Err(DbError::InvalidInput("Model not ready for deployment".into()));
        }
        self.status = ModelStatus::Deployed(deployment);
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }
}

/// Model parameters (weights, tree structures, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelParameters {
    LinearModel {
        weights: Vec<f64>,
        intercept: f64,
    },
    TreeModel {
        tree_data: Vec<u8>, // Serialized tree structure
    },
    EnsembleModel {
        models: Vec<Vec<u8>>, // Multiple serialized models
    },
    ClusteringModel {
        centroids: Vec<Vec<f64>>,
    },
    BayesModel {
        priors: HashMap<i64, f64>,
        distributions: Vec<u8>,
    },
    NeuralNetwork {
        layers: Vec<NetworkLayer>,
    },
    TimeSeriesModel {
        coefficients: Vec<f64>,
        seasonal_components: Vec<f64>,
    },
}

/// Neural network layer definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLayer {
    pub weights: Vec<Vec<f64>>,
    pub biases: Vec<f64>,
    pub activation: ActivationType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationType {
    ReLU,
    Sigmoid,
    Tanh,
    Linear,
    Softmax,
}

/// Model status in lifecycle
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelStatus {
    /// Model is being trained
    Training,
    /// Model is trained and ready for deployment
    Ready,
    /// Model is deployed
    Deployed(DeploymentConfig),
    /// Model is archived
    Archived,
    /// Model training failed
    Failed(String),
}

// ============================================================================
// Deployment Configuration
// ============================================================================

/// Model deployment configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment environment
    pub environment: DeploymentEnvironment,
    /// Traffic allocation (for A/B testing)
    pub traffic_allocation: f64,
    /// Deployment timestamp
    pub deployed_at: u64,
    /// Endpoint URL
    pub endpoint: Option<String>,
    /// Auto-scaling configuration
    pub scaling: ScalingConfig,
    /// Performance SLA
    pub sla: PerformanceSLA,
}

impl DeploymentConfig {
    pub fn new(environment: DeploymentEnvironment) -> Self {
        Self {
            environment,
            traffic_allocation: 1.0,
            deployed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            endpoint: None,
            scaling: ScalingConfig::default(),
            sla: PerformanceSLA::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentEnvironment {
    Development,
    Staging,
    Production,
}

/// Auto-scaling configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalingConfig {
    /// Minimum instances
    pub min_instances: u32,
    /// Maximum instances
    pub max_instances: u32,
    /// Target requests per second per instance
    pub target_rps: f64,
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            min_instances: 1,
            max_instances: 10,
            target_rps: 100.0,
        }
    }
}

/// Performance SLA requirements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceSLA {
    /// Maximum latency in milliseconds (p99)
    pub max_latency_ms: f64,
    /// Minimum accuracy/score
    pub min_accuracy: f64,
    /// Maximum error rate
    pub max_error_rate: f64,
}

impl Default for PerformanceSLA {
    fn default() -> Self {
        Self {
            max_latency_ms: 100.0,
            min_accuracy: 0.8,
            max_error_rate: 0.01,
        }
    }
}

// ============================================================================
// Model Metadata
// ============================================================================

/// Model metadata for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub id: ModelId,
    pub name: String,
    pub algorithm: Algorithm,
    pub version: u32,
    pub status: ModelStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub tags: Vec<String>,
    pub metrics: EvaluationMetrics,
    pub training_samples: usize,
    pub feature_count: usize,
}

impl From<&Model> for ModelMetadata {
    fn from(model: &Model) -> Self {
        Self {
            id: model.id,
            name: model.name.clone(),
            algorithm: model.algorithm.clone(),
            version: model.version,
            status: model.status.clone(),
            created_at: model.created_at,
            updated_at: model.updated_at,
            tags: model.tags.clone(),
            metrics: model.metrics.clone(),
            training_samples: model.training_stats.num_samples,
            feature_count: model.training_stats.num_features,
        }
    }
}

// ============================================================================
// A/B Testing
// ============================================================================

/// A/B test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    /// Test identifier
    pub id: String,
    /// Test name
    pub name: String,
    /// Model variants
    pub variants: Vec<ABVariant>,
    /// Traffic split strategy
    pub split_strategy: SplitStrategy,
    /// Test start time
    pub start_time: u64,
    /// Test end time
    pub end_time: Option<u64>,
    /// Success metric
    pub metric: String,
    /// Current results
    pub results: HashMap<String, ABResults>,
}

/// A/B test variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABVariant {
    pub name: String,
    pub model_id: ModelId,
    pub traffic_allocation: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitStrategy {
    /// Random traffic split
    Random,
    /// Hash-based consistent split
    Consistent,
    /// Bandit algorithm (Thompson sampling)
    Bandit,
}

/// A/B test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABResults {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub mean_latency: f64,
    pub mean_score: f64,
    pub confidence_interval: (f64, f64),
}

impl ABResults {
    pub fn new() -> Self {
        Self {
            requests: 0,
            successes: 0,
            failures: 0,
            mean_latency: 0.0,
            mean_score: 0.0,
            confidence_interval: (0.0, 0.0),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            self.successes as f64 / self.requests as f64
        }
    }
}

impl Default for ABResults {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Model Store
// ============================================================================

/// Model registry and storage
pub struct ModelStore {
    /// All registered models
    models: HashMap<ModelId, Model>,
    /// Model name to ID mapping
    name_to_id: HashMap<String, Vec<ModelId>>,
    /// Active A/B tests
    ab_tests: HashMap<String, ABTest>,
    /// Next model ID
    next_id: u64,
}

impl ModelStore {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            name_to_id: HashMap::new(),
            ab_tests: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a new model
    pub fn register_model(&mut self, mut model: Model) -> Result<ModelId> {
        let id = ModelId::new(self.next_id);
        self.next_id += 1;

        model.id = id;

        // Add to name index
        self.name_to_id
            .entry(model.name.clone())
            .or_insert_with(Vec::new)
            .push(id);

        self.models.insert(id, model);

        Ok(id)
    }

    /// Get a model by ID
    pub fn get_model(&self, id: ModelId) -> Result<&Model> {
        self.models
            .get(&id)
            .ok_or_else(|| DbError::InvalidInput(format!("Model not found: {:?}", id)))
    }

    /// Get a mutable model by ID
    pub fn get_model_mut(&mut self, id: ModelId) -> Result<&mut Model> {
        self.models
            .get_mut(&id)
            .ok_or_else(|| DbError::InvalidInput(format!("Model not found: {:?}", id)))
    }

    /// Get model metadata
    pub fn get_metadata(&self, id: ModelId) -> Result<ModelMetadata> {
        let model = self.get_model(id)?;
        Ok(ModelMetadata::from(model))
    }

    /// List all models
    pub fn list_models(&self) -> Vec<ModelMetadata> {
        self.models
            .values()
            .map(ModelMetadata::from)
            .collect()
    }

    /// Find models by name
    pub fn find_by_name(&self, name: &str) -> Vec<&Model> {
        self.name_to_id
            .get(name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.models.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get latest version of a named model
    pub fn get_latest_version(&self, name: &str) -> Option<&Model> {
        self.find_by_name(name)
            .into_iter()
            .max_by_key(|m| m.version)
    }

    /// Create a new version of a model
    pub fn create_version(&mut self, base_id: ModelId, new_model: Model) -> Result<ModelId> {
        let base = self.get_model(base_id)?;
        let mut versioned_model = new_model;
        versioned_model.name = base.name.clone();
        versioned_model.version = base.version + 1;

        self.register_model(versioned_model)
    }

    /// Delete a model
    pub fn delete_model(&mut self, id: ModelId) -> Result<()> {
        let model = self.models.remove(&id)
            .ok_or_else(|| DbError::InvalidInput(format!("Model not found: {:?}", id)))?;

        // Remove from name index
        if let Some(ids) = self.name_to_id.get_mut(&model.name) {
            ids.retain(|&i| i != id);
            if ids.is_empty() {
                self.name_to_id.remove(&model.name);
            }
        }

        Ok(())
    }

    /// Archive a model
    pub fn archive_model(&mut self, id: ModelId) -> Result<()> {
        let model = self.get_model_mut(id)?;
        model.status = ModelStatus::Archived;
        model.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    /// Deploy a model
    pub fn deploy_model(&mut self, id: ModelId, config: DeploymentConfig) -> Result<()> {
        let model = self.get_model_mut(id)?;
        model.deploy(config)
    }

    /// Create an A/B test
    pub fn create_ab_test(&mut self, test: ABTest) -> Result<()> {
        // Validate that all variant models exist
        for variant in &test.variants {
            self.get_model(variant.model_id)?;
        }

        // Validate traffic allocations sum to 1.0
        let total_traffic: f64 = test.variants.iter()
            .map(|v| v.traffic_allocation)
            .sum();

        if (total_traffic - 1.0).abs() > 1e-6 {
            return Err(DbError::InvalidInput(
                "Traffic allocations must sum to 1.0".into()
            ));
        }

        self.ab_tests.insert(test.id.clone(), test);
        Ok(())
    }

    /// Get A/B test
    pub fn get_ab_test(&self, id: &str) -> Option<&ABTest> {
        self.ab_tests.get(id)
    }

    /// Update A/B test results
    pub fn update_ab_results(&mut self, test_id: &str, variant: &str, results: ABResults) -> Result<()> {
        let test = self.ab_tests.get_mut(test_id)
            .ok_or_else(|| DbError::InvalidInput("A/B test not found".into()))?;

        test.results.insert(variant.to_string(), results);
        Ok(())
    }

    /// Get winning variant from A/B test
    pub fn get_winner(&self, test_id: &str) -> Option<(String, ModelId)> {
        let test = self.ab_tests.get(test_id)?;

        let winner = test.results.iter()
            .max_by(|(_, a), (_, b)| {
                a.mean_score.partial_cmp(&b.mean_score).unwrap()
            })?;

        let variant = test.variants.iter()
            .find(|v| v.name == *winner.0)?;

        Some((variant.name.clone(), variant.model_id))
    }

    /// Serialize model to bytes
    pub fn serialize_model(&self, id: ModelId) -> Result<Vec<u8>> {
        let model = self.get_model(id)?;
        bincode::serialize(model)
            .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))
    }

    /// Deserialize model from bytes
    pub fn deserialize_model(&mut self, bytes: &[u8]) -> Result<ModelId> {
        let model: Model = bincode::deserialize(bytes)
            .map_err(|e| DbError::Internal(format!("Deserialization error: {}", e)))?;

        self.register_model(model)
    }

    /// Export model metadata to JSON
    pub fn export_metadata(&self, id: ModelId) -> Result<String> {
        let metadata = self.get_metadata(id)?;
        serde_json::to_string_pretty(&metadata)
            .map_err(|e| DbError::Internal(format!("JSON export error: {}", e)))
    }

    /// Search models by tags
    pub fn search_by_tags(&self, tags: &[String]) -> Vec<&Model> {
        self.models
            .values()
            .filter(|model| {
                tags.iter().any(|tag| model.tags.contains(tag))
            })
            .collect()
    }

    /// Get models by status
    pub fn get_by_status(&self, status: &ModelStatus) -> Vec<&Model> {
        self.models
            .values()
            .filter(|model| std::mem::discriminant(&model.status) == std::mem::discriminant(status))
            .collect()
    }

    /// Get model performance summary
    pub fn get_performance_summary(&self, id: ModelId) -> Result<PerformanceSummary> {
        let model = self.get_model(id)?;

        Ok(PerformanceSummary {
            model_id: id,
            model_name: model.name.clone(),
            algorithm: model.algorithm.clone(),
            metrics: model.metrics.clone(),
            training_time: model.training_stats.training_time,
            version: model.version,
            status: model.status.clone(),
        })
    }
}

impl Default for ModelStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance summary for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub model_id: ModelId,
    pub model_name: String,
    pub algorithm: Algorithm,
    pub metrics: EvaluationMetrics,
    pub training_time: f64,
    pub version: u32,
    pub status: ModelStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_store() {
        let mut store = ModelStore::new();

        let model = Model::new(
            ModelId::new(0),
            "test_model".to_string(),
            Algorithm::LinearRegression,
            ModelParameters::LinearModel {
                weights: vec![1.0, 2.0],
                intercept: 0.5,
            },
            Hyperparameters::new(),
        );

        let id = store.register_model(model).unwrap();
        assert_eq!(id, ModelId::new(1));

        let retrieved = store.get_model(id).unwrap();
        assert_eq!(retrieved.name, "test_model");
    }

    #[test]
    fn test_model_versioning() {
        let mut store = ModelStore::new();

        let model1 = Model::new(
            ModelId::new(0),
            "model".to_string(),
            Algorithm::LinearRegression,
            ModelParameters::LinearModel {
                weights: vec![1.0],
                intercept: 0.0,
            },
            Hyperparameters::new(),
        );

        let id1 = store.register_model(model1).unwrap();

        let model2 = Model::new(
            ModelId::new(0),
            "model_v2".to_string(),
            Algorithm::LinearRegression,
            ModelParameters::LinearModel {
                weights: vec![2.0],
                intercept: 1.0,
            },
            Hyperparameters::new(),
        );

        let id2 = store.create_version(id1, model2).unwrap();

        let latest = store.get_latest_version("model").unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.id, id2);
    }
}


