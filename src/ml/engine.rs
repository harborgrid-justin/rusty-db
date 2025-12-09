// # ML Engine - Core Orchestration
//
// This module provides the main ML engine that orchestrates all machine learning operations,
// including model training, versioning, deployment, and lifecycle management.

use std::fmt;
use crate::error::Result;
use super::{
    Dataset, Hyperparameters, Metrics, MLError,
    algorithms::{Algorithm, ModelType, LinearRegression, LogisticRegression,
                 DecisionTree, RandomForest, KMeansClustering, NaiveBayes},
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime};
use serde::{Serialize, Deserialize};
use parking_lot::Mutex;

// ============================================================================
// Model Metadata and Versioning
// ============================================================================

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model name
    pub name: String,
    /// Model version
    pub version: ModelVersion,
    /// Model type
    pub model_type: ModelType,
    /// Feature names used for training
    pub feature_names: Vec<String>,
    /// Number of features
    pub n_features: usize,
    /// Training metrics
    pub metrics: HashMap<String, f64>,
    /// Hyperparameters used
    pub hyperparameters: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Training status
    pub status: ModelStatus,
    /// Description
    pub description: Option<String>,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Model size in bytes
    pub model_size: usize,
    /// Number of training samples
    pub training_samples: usize,
}

impl ModelMetadata {
    /// Create new metadata
    pub fn new(name: String, model_type: ModelType, feature_names: Vec<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            name,
            version: ModelVersion::new(1, 0, 0),
            model_type,
            feature_names: feature_names.clone(),
            n_features: feature_names.len(),
            metrics: HashMap::new(),
            hyperparameters: HashMap::new(),
            created_at: now,
            updated_at: now,
            status: ModelStatus::Training,
            description: None,
            tags: Vec::new(),
            model_size: 0,
            training_samples: 0,
        }
    }

    /// Update timestamp
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Model version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModelVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ModelVersion {
    /// Create a new version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Increment major version
    pub fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    /// Increment minor version
    pub fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// Increment patch version
    pub fn increment_patch(&mut self) {
        self.patch += 1;
    }

    /// Format as string (e.g., "1.2.3")
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for ModelVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Model training and deployment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelStatus {
    /// Currently training
    Training,
    /// Training completed successfully
    Trained,
    /// Deployed and active
    Deployed,
    /// In A/B testing
    ABTesting,
    /// Archived (not active)
    Archived,
    /// Training failed
    Failed,
}

impl std::fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelStatus::Training => write!(f, "Training"),
            ModelStatus::Trained => write!(f, "Trained"),
            ModelStatus::Deployed => write!(f, "Deployed"),
            ModelStatus::ABTesting => write!(f, "A/B Testing"),
            ModelStatus::Archived => write!(f, "Archived"),
            ModelStatus::Failed => write!(f, "Failed"),
        }
    }
}

// ============================================================================
// Model Storage
// ============================================================================

/// Stored model with serialized data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredModel {
    /// Model metadata
    pub metadata: ModelMetadata,
    /// Serialized model bytes
    pub model_data: Vec<u8>,
    /// Preprocessing pipeline (optional)
    pub preprocessing_pipeline: Option<Vec<u8>>,
}

impl StoredModel {
    /// Create a new stored model
    pub fn new(metadata: ModelMetadata, model_data: Vec<u8>) -> Self {
        Self {
            metadata,
            model_data,
            preprocessing_pipeline: None,
        }
    }

    /// Add preprocessing pipeline
    pub fn with_preprocessing(mut self, pipeline: Vec<u8>) -> Self {
        self.preprocessing_pipeline = Some(pipeline)));
        self
    }
}

// ============================================================================
// Model Registry
// ============================================================================

/// Model registry for managing trained models
#[derive(Debug, Clone)]
pub struct ModelRegistry {
    /// Stored models by name
    models: Arc<RwLock<HashMap<String, Vec<StoredModel>>>>,
    /// Active model versions by name
    active_versions: Arc<RwLock<HashMap<String, ModelVersion>>>,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            active_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new model
    pub fn register(&self, model: StoredModel) -> Result<()> {
        let name = model.metadata.name.clone();
        let version = model.metadata.version;

        let mut models = self.models.write().unwrap();
        let versions = models.entry(name.clone()).or_insert_with(Vec::new);

        // Check for version conflicts
        if versions.iter().any(|m| m.metadata.version == version) {
            return Err(MLError::VersionConflict(
                format!("Model {} version {} already exists", name, version)
            ).into())));
        }

        versions.push(model);
        versions.sort_by(|a, b| b.metadata.version.cmp(&a.metadata.version));

        // Set as active if it's the first version or explicitly deployed
        if versions.len() == 1 {
            let mut active = self.active_versions.write().unwrap();
            active.insert(name, version);
        }

        Ok(())
    }

    /// Get a specific model version
    pub fn get(&self, name: &str, version: Option<ModelVersion>) -> Result<StoredModel> {
        let models = self.models.read().unwrap();

        let versions = models.get(name)
            .ok_or_else(|| MLError::ModelNotFound(name.to_string()))?;

        if let Some(version) = version {
            versions.iter()
                .find(|m| m.metadata.version == version)
                .cloned()
                .ok_or_else(|| MLError::ModelNotFound(
                    format!("{} version {}", name, version)
                ).into())
        } else {
            // Get active version
            let active = self.active_versions.read().unwrap()));
            let active_version = active.get(name)
                .ok_or_else(|| MLError::ModelNotFound(name.to_string()))?;

            versions.iter()
                .find(|m| m.metadata.version == *active_version)
                .cloned()
                .ok_or_else(|| MLError::ModelNotFound(name.to_string()).into())
        }
    }

    /// List all versions of a model
    pub fn list_versions(&self, name: &str) -> Result<Vec<ModelMetadata>> {
        let models = self.models.read().unwrap();

        let versions = models.get(name)
            .ok_or_else(|| MLError::ModelNotFound(name.to_string()))?;

        Ok(versions.iter().map(|m| m.metadata.clone()).collect())
    }

    /// List all model names
    pub fn list_models(&self) -> Vec<String> {
        let models = self.models.read().unwrap();
        models.keys().cloned().collect()
    }

    /// Set active version for a model
    pub fn set_active_version(&self, name: &str, version: ModelVersion) -> Result<()> {
        let models = self.models.read().unwrap();

        let versions = models.get(name)
            .ok_or_else(|| MLError::ModelNotFound(name.to_string()))?;

        // Verify version exists
        if !versions.iter().any(|m| m.metadata.version == version) {
            return Err(MLError::ModelNotFound(
                format!("{} version {}", name, version)
            ).into())));
        }

        let mut active = self.active_versions.write().unwrap();
        active.insert(name.to_string(), version);

        Ok(())
    }

    /// Delete a model version
    pub fn delete(&self, name: &str, version: Option<ModelVersion>) -> Result<()> {
        let mut models = self.models.write().unwrap();

        if let Some(version) = version {
            let versions = models.get_mut(name)
                .ok_or_else(|| MLError::ModelNotFound(name.to_string()))?;

            if let Some(pos) = versions.iter().position(|m| m.metadata.version == version) {
                versions.remove(pos);

                // If this was the active version, set the latest as active
                let active = self.active_versions.read().unwrap();
                if let Some(active_version) = active.get(name) {
                    if *active_version == version && !versions.is_empty() {
                        drop(active);
                        let mut active_mut = self.active_versions.write().unwrap();
                        active_mut.insert(name.to_string(), versions[0].metadata.version);
                    }
                }
            } else {
                return Err(MLError::ModelNotFound(
                    format!("{} version {}", name, version)
                ).into())));
            }

            // Remove model entry if no versions left
            if versions.is_empty() {
                models.remove(name);
                let mut active = self.active_versions.write().unwrap();
                active.remove(name);
            }
        } else {
            // Delete all versions
            models.remove(name);
            let mut active = self.active_versions.write().unwrap();
            active.remove(name);
        }

        Ok(())
    }

    /// Get total number of registered models
    pub fn count(&self) -> usize {
        let models = self.models.read().unwrap();
        models.len()
    }

    /// Search models by tags
    pub fn search_by_tags(&self, tags: &[String]) -> Vec<ModelMetadata> {
        let models = self.models.read().unwrap();
        let mut results = Vec::new();

        for versions in models.values() {
            for model in versions {
                if tags.iter().all(|tag| model.metadata.tags.contains(tag)) {
                    results.push(model.metadata.clone());
                }
            }
        }

        results
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Training Jobs
// ============================================================================

/// Training job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Training job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    /// Job ID
    pub id: String,
    /// Model name
    pub model_name: String,
    /// Model type
    pub model_type: ModelType,
    /// Job status
    pub status: TrainingJobStatus,
    /// Progress (0.0 to 1.0)
    pub progress: f64,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Created timestamp
    pub created_at: u64,
    /// Started timestamp
    pub started_at: Option<u64>,
    /// Completed timestamp
    pub completed_at: Option<u64>,
    /// Hyperparameters
    pub hyperparameters: Hyperparameters,
}

impl TrainingJob {
    /// Create a new training job
    pub fn new(model_name: String, model_type: ModelType, hyperparameters: Hyperparameters) -> Self {
        use uuid::Uuid;

        Self {
            id: Uuid::new_v4().to_string(),
            model_name,
            model_type,
            status: TrainingJobStatus::Queued,
            progress: 0.0,
            error: None,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            started_at: None,
            completed_at: None,
            hyperparameters,
        }
    }

    /// Mark job as started
    pub fn start(&mut self) {
        self.status = TrainingJobStatus::Running;
        self.started_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }

    /// Update progress
    pub fn update_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Mark job as completed
    pub fn complete(&mut self) {
        self.status = TrainingJobStatus::Completed;
        self.progress = 1.0;
        self.completed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: String) {
        self.status = TrainingJobStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }

    /// Get elapsed time in seconds
    pub fn elapsed_seconds(&self) -> u64 {
        let start = self.started_at.unwrap_or(self.created_at);
        let end = self.completed_at.unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        });
        end - start
    }
}

// ============================================================================
// ML Engine
// ============================================================================

/// Main ML engine
pub struct MLEngine {
    /// Model registry
    registry: ModelRegistry,
    /// Active training jobs
    jobs: Arc<Mutex<HashMap<String, TrainingJob>>>,
    /// Resource limits
    max_concurrent_jobs: usize,
    /// Performance metrics collector
    metrics_collector: Arc<Mutex<HashMap<String, Vec<Metrics>>>>,
}

impl MLEngine {
    /// Create a new ML engine
    pub fn new() -> Self {
        Self {
            registry: ModelRegistry::new(),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent_jobs: 4,
            metrics_collector: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set maximum concurrent training jobs
    pub fn set_max_concurrent_jobs(&mut self, max: usize) {
        self.max_concurrent_jobs = max;
    }

    /// Get model registry
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Create a training job
    pub fn create_training_job(
        &self,
        model_name: String,
        model_type: ModelType,
        hyperparameters: Option<Hyperparameters>,
    ) -> Result<TrainingJob> {
        let params = hyperparameters.unwrap_or_else(|| model_type.default_hyperparameters());
        let job = TrainingJob::new(model_name, model_type, params);

        let mut jobs = self.jobs.lock();

        // Check concurrent job limit
        let running_jobs = jobs.values()
            .filter(|j| j.status == TrainingJobStatus::Running)
            .count();

        if running_jobs >= self.max_concurrent_jobs {
            return Err(MLError::TrainingFailed(
                format!("Maximum concurrent jobs ({}) reached", self.max_concurrent_jobs)
            ).into())));
        }

        let job_id = job.id.clone();
        jobs.insert(job_id, job.clone());

        Ok(job)
    }

    /// Get training job status
    pub fn get_job(&self, job_id: &str) -> Result<TrainingJob> {
        let jobs = self.jobs.lock();
        jobs.get(job_id)
            .cloned()
            .ok_or_else(|| MLError::InvalidConfiguration(
                format!("Job {} not found", job_id)
            ).into())
    }

    /// Train a model
    pub fn train_model(
        &self,
        model_name: String,
        model_type: ModelType,
        dataset: Dataset,
        hyperparameters: Option<Hyperparameters>,
    ) -> Result<ModelMetadata> {
        dataset.validate()?);

        let params = hyperparameters.unwrap_or_else(|| model_type.default_hyperparameters());

        // Create training job
        let mut job = self.create_training_job(model_name.clone(), model_type, Some(params.clone()))?;
        job.start();

        {
            let mut jobs = self.jobs.lock();
            jobs.insert(job.id.clone(), job.clone());
        }

        // Train the model
        let result = self.train_model_internal(model_type, &dataset, &params);

        match result {
            Ok((model_data, metrics)) => {
                // Create metadata
                let mut metadata = ModelMetadata::new(
                    model_name.clone(),
                    model_type,
                    dataset.feature_names.clone(),
                );
                metadata.status = ModelStatus::Trained;
                metadata.metrics = metrics.all().clone();
                metadata.model_size = model_data.len();
                metadata.training_samples = dataset.num_samples();

                // Store model
                let stored_model = StoredModel::new(metadata.clone(), model_data);
                self.registry.register(stored_model)?;

                // Update job
                {
                    let mut jobs = self.jobs.lock();
                    if let Some(j) = jobs.get_mut(&job.id) {
                        j.complete();
                    }
                }

                // Collect metrics
                {
                    let mut collector = self.metrics_collector.lock();
                    collector.entry(model_name).or_insert_with(Vec::new).push(metrics);
                }

                Ok(metadata)
            }
            Err(e) => {
                // Update job
                {
                    let mut jobs = self.jobs.lock();
                    if let Some(j) = jobs.get_mut(&job.id) {
                        j.fail(e.to_string());
                    }
                }
                Err(e)
            }
        }
    }

    /// Internal training implementation
    fn train_model_internal(
        &self,
        model_type: ModelType,
        dataset: &Dataset,
        params: &Hyperparameters,
    ) -> Result<(Vec<u8>, Metrics)> {
        match model_type {
            ModelType::LinearRegression => {
                let mut model = LinearRegression::new();
                model.fit(dataset, params)?;
                let model_data = Algorithm::serialize(&model)?;

                let mut metrics = Metrics::new();
                // Extract metrics from model if available
                metrics.set("trained", 1.0);

                Ok((model_data, metrics))
            }
            ModelType::LogisticRegression => {
                let mut model = LogisticRegression::new();
                model.fit(dataset, params)?;
                let model_data = Algorithm::serialize(&model)?;

                let mut metrics = Metrics::new();
                metrics.set("trained", 1.0);

                Ok((model_data, metrics))
            }
            ModelType::DecisionTree => {
                let is_classifier = dataset.target.as_ref()
                    .map(|t| t.iter().all(|&v| v == v.floor()))
                    .unwrap_or(false);

                let mut model = DecisionTree::new(is_classifier);
                model.fit(dataset, params)?;
                let model_data = Algorithm::serialize(&model)?;

                let mut metrics = Metrics::new();
                metrics.set("trained", 1.0);

                Ok((model_data, metrics))
            }
            ModelType::RandomForest => {
                let is_classifier = dataset.target.as_ref()
                    .map(|t| t.iter().all(|&v| v == v.floor()))
                    .unwrap_or(false);

                let mut model = RandomForest::new(is_classifier);
                model.fit(dataset, params)?;
                let model_data = Algorithm::serialize(&model)?;

                let mut metrics = Metrics::new();
                metrics.set("trained", 1.0);

                Ok((model_data, metrics))
            }
            ModelType::KMeans => {
                let mut model = KMeansClustering::new();
                model.fit(dataset, params)?;
                let model_data = Algorithm::serialize(&model)?;

                let mut metrics = Metrics::new();
                metrics.set("trained", 1.0);

                Ok((model_data, metrics))
            }
            ModelType::NaiveBayes => {
                let mut model = NaiveBayes::new();
                model.fit(dataset, params)?;
                let model_data = Algorithm::serialize(&model)?;

                let mut metrics = Metrics::new();
                metrics.set("trained", 1.0);

                Ok((model_data, metrics))
            }
        }
    }

    /// Deploy a model
    pub fn deploy_model(&self, name: &str, version: Option<ModelVersion>) -> Result<()> {
        let mut stored_model = self.registry.get(name, version)?;
        stored_model.metadata.status = ModelStatus::Deployed;
        stored_model.metadata.touch();

        // Update in registry
        self.registry.register(stored_model)?;

        Ok(())
    }

    /// Start A/B testing with two model versions
    pub fn start_ab_test(&self, name: &str, version_a: ModelVersion, version_b: ModelVersion) -> Result<()> {
        // Verify both versions exist
        self.registry.get(name, Some(version_a))?;
        self.registry.get(name, Some(version_b))?;

        // Mark both as A/B testing
        let mut model_a = self.registry.get(name, Some(version_a))?;
        model_a.metadata.status = ModelStatus::ABTesting;
        model_a.metadata.add_tag("ab_test_a".to_string());
        self.registry.register(model_a)?;

        let mut model_b = self.registry.get(name, Some(version_b))?;
        model_b.metadata.status = ModelStatus::ABTesting;
        model_b.metadata.add_tag("ab_test_b".to_string());
        self.registry.register(model_b)?;

        Ok(())
    }

    /// Archive a model version
    pub fn archive_model(&self, name: &str, version: ModelVersion) -> Result<()> {
        let mut stored_model = self.registry.get(name, Some(version))?;
        stored_model.metadata.status = ModelStatus::Archived;
        stored_model.metadata.touch();

        self.registry.register(stored_model)?;

        Ok(())
    }

    /// Get model performance history
    pub fn get_model_metrics(&self, name: &str) -> Vec<Metrics> {
        let collector = self.metrics_collector.lock();
        collector.get(name).cloned().unwrap_or_default()
    }

    /// List all training jobs
    pub fn list_jobs(&self) -> Vec<TrainingJob> {
        let jobs = self.jobs.lock();
        jobs.values().cloned().collect()
    }

    /// Cancel a training job
    pub fn cancel_job(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.jobs.lock();
        if let Some(job) = jobs.get_mut(job_id) {
            if job.status == TrainingJobStatus::Running || job.status == TrainingJobStatus::Queued {
                job.status = TrainingJobStatus::Cancelled;
                job.completed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                Ok(())
            } else {
                Err(MLError::InvalidConfiguration(
                    format!("Job {} is not running", job_id)
                ).into())
            }
        } else {
            Err(MLError::InvalidConfiguration(
                format!("Job {} not found", job_id)
            ).into())
        }
    }

    /// Clean up completed jobs
    pub fn cleanup_jobs(&self, older_than_seconds: u64) {
        let mut jobs = self.jobs.lock()));
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        jobs.retain(|_, job| {
            match job.status {
                TrainingJobStatus::Completed | TrainingJobStatus::Failed | TrainingJobStatus::Cancelled => {
                    if let Some(completed_at) = job.completed_at {
                        now - completed_at < older_than_seconds
                    } else {
                        true
                    }
                }
                _ => true,
            }
        });
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
use std::time::UNIX_EPOCH;

    #[test]
    fn test_model_version() {
        let mut v = ModelVersion::new(1, 0, 0);
        assert_eq!(v.to_string(), "1.0.0");

        v.increment_minor();
        assert_eq!(v.to_string(), "1.1.0");

        v.increment_major();
        assert_eq!(v.to_string(), "2.0.0");
    }

    #[test]
    fn test_model_registry() {
        let registry = ModelRegistry::new();

        let metadata = ModelMetadata::new(
            "test_model".to_string(),
            ModelType::LinearRegression,
            vec!["f1".to_string(), "f2".to_string()],
        );

        let model = StoredModel::new(metadata, vec![1, 2, 3]);
        assert!(registry.register(model).is_ok());

        assert_eq!(registry.count(), 1);
        assert!(registry.get("test_model", None).is_ok());
    }

    #[test]
    fn test_ml_engine() {
        let engine = MLEngine::new();

        let features = vec![
            vec![1.0],
            vec![2.0],
            vec![3.0],
        ];
        let target = Some(vec![2.0, 4.0, 6.0]);
        let dataset = Dataset::new(features, target, vec!["x".to_string()]);

        let result = engine.train_model(
            "test_model".to_string(),
            ModelType::LinearRegression,
            dataset,
            None,
        );

        assert!(result.is_ok());
        assert_eq!(engine.registry().count(), 1);
    }
}
