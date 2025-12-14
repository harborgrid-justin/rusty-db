// # Inference Engine
//
// This module provides real-time and batch prediction capabilities with optimized
// execution paths, model caching, and prediction explanations.

use super::{
    algorithms::{
        Algorithm, DecisionTree, KMeansClustering, LinearRegression, LogisticRegression, ModelType,
        NaiveBayes, RandomForest,
    },
    engine::{ModelRegistry, ModelVersion, StoredModel},
    MLError, Matrix, Vector,
};
use crate::error::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Prediction Result
// ============================================================================

// Result of a prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    // Predicted values
    pub predictions: Vector,
    // Confidence scores (if available)
    pub confidence: Option<ConfidenceScore>,
    // Feature importance for this prediction (if available)
    pub feature_importance: Option<FeatureImportance>,
    // Prediction metadata
    pub metadata: PredictionMetadata,
}

impl PredictionResult {
    // Create a simple prediction result
    pub fn new(predictions: Vector) -> Self {
        Self {
            predictions,
            confidence: None,
            feature_importance: None,
            metadata: PredictionMetadata::default(),
        }
    }

    // Add confidence scores
    pub fn with_confidence(mut self, confidence: ConfidenceScore) -> Self {
        self.confidence = Some(confidence);
        self
    }

    // Add feature importance
    pub fn with_feature_importance(mut self, importance: FeatureImportance) -> Self {
        self.feature_importance = Some(importance);
        self
    }

    // Add metadata
    pub fn with_metadata(mut self, metadata: PredictionMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

// Prediction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionMetadata {
    // Model name
    pub model_name: String,
    // Model version
    pub model_version: ModelVersion,
    // Prediction timestamp
    pub timestamp: u64,
    // Inference time in microseconds
    pub inference_time_us: u64,
    // Number of samples predicted
    pub num_samples: usize,
    // Was cache hit
    pub cache_hit: bool,
}

impl Default for PredictionMetadata {
    fn default() -> Self {
        Self {
            model_name: String::new(),
            model_version: ModelVersion::new(0, 0, 0),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            inference_time_us: 0,
            num_samples: 0,
            cache_hit: false,
        }
    }
}

// ============================================================================
// Confidence Score
// ============================================================================

// Confidence scores for predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    // Confidence values per prediction (0.0 to 1.0)
    pub scores: Vector,
    // Prediction intervals (for regression)
    pub intervals: Option<Vec<(f64, f64)>>,
    // Class probabilities (for classification)
    pub class_probabilities: Option<Vec<HashMap<String, f64>>>,
}

impl ConfidenceScore {
    // Create confidence scores
    pub fn new(scores: Vector) -> Self {
        Self {
            scores,
            intervals: None,
            class_probabilities: None,
        }
    }

    // Add prediction intervals
    pub fn with_intervals(mut self, intervals: Vec<(f64, f64)>) -> Self {
        self.intervals = Some(intervals);
        self
    }

    // Add class probabilities
    pub fn with_class_probabilities(mut self, probabilities: Vec<HashMap<String, f64>>) -> Self {
        self.class_probabilities = Some(probabilities);
        self
    }

    // Get average confidence
    pub fn average(&self) -> f64 {
        if self.scores.is_empty() {
            0.0
        } else {
            self.scores.iter().sum::<f64>() / self.scores.len() as f64
        }
    }

    // Get minimum confidence
    pub fn min(&self) -> f64 {
        self.scores.iter().copied().fold(f64::INFINITY, f64::min)
    }

    // Get maximum confidence
    pub fn max(&self) -> f64 {
        self.scores
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

// ============================================================================
// Feature Importance
// ============================================================================

// Feature importance for prediction explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImportance {
    // Feature names
    pub feature_names: Vec<String>,
    // Importance scores
    pub importance_scores: Vector,
    // Top-k most important features
    pub top_features: Vec<(String, f64)>,
}

impl FeatureImportance {
    // Create feature importance
    pub fn new(feature_names: Vec<String>, importance_scores: Vector) -> Self {
        let mut feature_scores: Vec<(String, f64)> = feature_names
            .iter()
            .zip(importance_scores.iter())
            .map(|(name, &score)| (name.clone(), score))
            .collect();

        feature_scores.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        let top_features = feature_scores.iter().take(10).cloned().collect();

        Self {
            feature_names,
            importance_scores,
            top_features,
        }
    }

    // Get importance for a specific feature
    pub fn get_importance(&self, feature_name: &str) -> Option<f64> {
        self.feature_names
            .iter()
            .position(|name| name == feature_name)
            .and_then(|idx| self.importance_scores.get(idx).copied())
    }

    // Get top k features
    pub fn top_k(&self, k: usize) -> Vec<(String, f64)> {
        self.top_features.iter().take(k).cloned().collect()
    }
}

// ============================================================================
// Model Cache
// ============================================================================

// Cached model entry
#[derive(Clone)]
pub(crate) struct CachedModel {
    // Model type
    model_type: ModelType,
    // Serialized model data
    model_data: Vec<u8>,
    // Feature names
    feature_names: Vec<String>,
    // Last access time
    last_access: Instant,
    // Access count
    access_count: usize,
    // Model size in bytes
    size_bytes: usize,
}

// Model cache for fast inference
pub struct ModelCache {
    // Cached models
    cache: Arc<RwLock<HashMap<String, CachedModel>>>,
    // Maximum cache size in bytes
    max_size_bytes: usize,
    // Current cache size
    current_size: Arc<Mutex<usize>>,
    // Cache statistics
    stats: Arc<Mutex<CacheStats>>,
}

#[derive(Debug, Clone, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl ModelCache {
    // Create a new model cache
    pub fn new(max_size_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size: Arc::new(Mutex::new(0)),
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    // Put a model in cache
    pub fn put(&self, key: String, stored_model: &StoredModel) {
        let size = stored_model.model_data.len();

        // Check if we need to evict
        let mut current_size = self.current_size.lock();
        while *current_size + size > self.max_size_bytes {
            if !self.evict_lru() {
                break; // Cache is empty
            }
        }

        let cached = CachedModel {
            model_type: stored_model.metadata.model_type,
            model_data: stored_model.model_data.clone(),
            feature_names: stored_model.metadata.feature_names.clone(),
            last_access: Instant::now(),
            access_count: 0,
            size_bytes: size,
        };

        let mut cache = self.cache.write().unwrap();
        if cache.insert(key, cached).is_none() {
            *current_size += size;
        }
    }

    // Get a model from cache
    pub fn get(&self, key: &str) -> Option<(ModelType, Vec<u8>, Vec<String>)> {
        let mut cache = self.cache.write().unwrap();

        if let Some(cached) = cache.get_mut(key) {
            cached.last_access = Instant::now();
            cached.access_count += 1;

            let mut stats = self.stats.lock();
            stats.hits += 1;

            Some((
                cached.model_type,
                cached.model_data.clone(),
                cached.feature_names.clone(),
            ))
        } else {
            let mut stats = self.stats.lock();
            stats.misses += 1;
            None
        }
    }

    // Remove a model from cache
    pub fn remove(&self, key: &str) -> bool {
        let mut cache = self.cache.write().unwrap();
        if let Some(cached) = cache.remove(key) {
            let mut current_size = self.current_size.lock();
            *current_size -= cached.size_bytes;
            true
        } else {
            false
        }
    }

    // Evict least recently used item
    fn evict_lru(&self) -> bool {
        let mut cache = self.cache.write().unwrap();

        if cache.is_empty() {
            return false;
        }

        // Find LRU item
        let lru_key = cache
            .iter()
            .min_by_key(|(_, cached)| cached.last_access)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            if let Some(cached) = cache.remove(&key) {
                let mut current_size = self.current_size.lock();
                *current_size -= cached.size_bytes;

                let mut stats = self.stats.lock();
                stats.evictions += 1;

                return true;
            }
        }

        false
    }

    // Clear the cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();

        let mut current_size = self.current_size.lock();
        *current_size = 0;
    }

    // Get cache statistics
    pub fn stats(&self) -> (u64, u64, u64) {
        let stats = self.stats.lock();
        (stats.hits, stats.misses, stats.evictions)
    }

    // Get cache size
    pub fn size(&self) -> usize {
        *self.current_size.lock()
    }

    // Get number of cached models
    pub fn count(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }

    // Warm up cache with models
    pub fn warmup(&self, registry: &ModelRegistry, model_names: Vec<String>) -> Result<()> {
        for name in model_names {
            let stored_model = registry.get(&name, None)?;
            let key = format!("{}:{}", name, stored_model.metadata.version);
            self.put(key, &stored_model);
        }
        Ok(())
    }
}

impl Default for ModelCache {
    fn default() -> Self {
        Self::new(100) // 100 MB default
    }
}

// ============================================================================
// Batch Predictor
// ============================================================================

// Batch prediction optimizer
pub struct BatchPredictor {
    // Batch size for prediction
    batch_size: usize,
    // Enable parallel processing
    #[allow(dead_code)]
    parallel: bool,
}

impl BatchPredictor {
    // Create a new batch predictor
    pub fn new(batch_size: usize, parallel: bool) -> Self {
        Self {
            batch_size,
            parallel,
        }
    }

    // Predict in batches
    pub fn predict_batch(
        &self,
        features: &Matrix,
        model_type: ModelType,
        model_data: &[u8],
    ) -> Result<Vector> {
        let n_samples = features.len();
        let mut all_predictions = Vec::with_capacity(n_samples);

        // Process in batches
        for batch_start in (0..n_samples).step_by(self.batch_size) {
            let batch_end = (batch_start + self.batch_size).min(n_samples);
            let batch_features: Matrix = features[batch_start..batch_end].to_vec();

            let predictions = self.predict_single_batch(&batch_features, model_type, model_data)?;
            all_predictions.extend(predictions);
        }

        Ok(all_predictions)
    }

    // Predict a single batch
    fn predict_single_batch(
        &self,
        features: &Matrix,
        model_type: ModelType,
        model_data: &[u8],
    ) -> Result<Vector> {
        match model_type {
            ModelType::LinearRegression => {
                let model: LinearRegression = serde_json::from_slice(model_data).map_err(|e| {
                    MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                })?;
                model.predict(features)
            }
            ModelType::LogisticRegression => {
                let model: LogisticRegression =
                    serde_json::from_slice(model_data).map_err(|e| {
                        MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                    })?;
                model.predict(features)
            }
            ModelType::DecisionTree => {
                let model: DecisionTree = serde_json::from_slice(model_data).map_err(|e| {
                    MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                })?;
                model.predict(features)
            }
            ModelType::RandomForest => {
                let model: RandomForest = serde_json::from_slice(model_data).map_err(|e| {
                    MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                })?;
                model.predict(features)
            }
            ModelType::KMeans => {
                let model: KMeansClustering = serde_json::from_slice(model_data).map_err(|e| {
                    MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                })?;
                model.predict(features)
            }
            ModelType::NaiveBayes => {
                let model: NaiveBayes = serde_json::from_slice(model_data).map_err(|e| {
                    MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                })?;
                model.predict(features)
            }
            ModelType::KMeansClustering => Err(MLError::PredictionFailed(
                "KMeans clustering prediction not yet implemented".to_string(),
            )
            .into()),
        }
    }

    // Get optimal batch size based on feature dimensions
    pub fn optimal_batch_size(n_features: usize, n_samples: usize) -> usize {
        // Simple heuristic: larger batches for smaller feature sets
        if n_features < 10 {
            1000.min(n_samples)
        } else if n_features < 100 {
            500.min(n_samples)
        } else {
            100.min(n_samples)
        }
    }
}

impl Default for BatchPredictor {
    fn default() -> Self {
        Self::new(1000, false)
    }
}

// ============================================================================
// Inference Engine
// ============================================================================

// Main inference engine
pub struct InferenceEngine {
    // Model registry
    registry: Arc<ModelRegistry>,
    // Model cache
    cache: ModelCache,
    // Batch predictor
    batch_predictor: BatchPredictor,
    // Prediction logging
    log_predictions: bool,
    // Prediction logs
    prediction_logs: Arc<Mutex<Vec<PredictionLog>>>,
}

// Prediction log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionLog {
    model_name: String,
    model_version: ModelVersion,
    timestamp: u64,
    num_samples: usize,
    inference_time_us: u64,
}

impl InferenceEngine {
    // Create a new inference engine
    pub fn new(registry: Arc<ModelRegistry>) -> Self {
        Self {
            registry,
            cache: ModelCache::new(100),
            batch_predictor: BatchPredictor::default(),
            log_predictions: false,
            prediction_logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // Enable prediction logging
    pub fn enable_logging(&mut self) {
        self.log_predictions = true;
    }

    // Set cache size
    pub fn set_cache_size(&mut self, size_mb: usize) {
        self.cache = ModelCache::new(size_mb);
    }

    // Set batch size
    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_predictor = BatchPredictor::new(batch_size, false);
    }

    // Real-time prediction (single or small batch)
    pub fn predict(
        &self,
        model_name: &str,
        model_version: Option<ModelVersion>,
        features: &Matrix,
    ) -> Result<PredictionResult> {
        let start = Instant::now();

        // Get model from cache or registry
        let cache_key = model_version
            .as_ref()
            .map(|v| format!("{}:{}", model_name, v))
            .unwrap_or_else(|| model_name.to_string());

        let (model_type, model_data, feature_names, cache_hit, actual_version) =
            if let Some((mt, data, names)) = self.cache.get(&cache_key) {
                let stored = self.registry.get(model_name, model_version)?;
                (mt, data, names, true, stored.metadata.version)
            } else {
                let stored = self.registry.get(model_name, model_version)?;
                let version = stored.metadata.version;
                self.cache.put(cache_key.clone(), &stored);
                (
                    stored.metadata.model_type,
                    stored.model_data.clone(),
                    stored.metadata.feature_names.clone(),
                    false,
                    version,
                )
            };

        // Validate feature count
        let n_features = features.get(0).map(|row| row.len()).unwrap_or(0);
        if n_features != feature_names.len() {
            return Err(MLError::FeatureMismatch {
                expected: feature_names.len(),
                got: n_features,
            }
            .into());
        }

        // Make prediction
        let predictions = self
            .batch_predictor
            .predict_batch(features, model_type, &model_data)?;

        let inference_time = start.elapsed();

        // Create metadata
        let metadata = PredictionMetadata {
            model_name: model_name.to_string(),
            model_version: actual_version,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            inference_time_us: inference_time.as_micros() as u64,
            num_samples: features.len(),
            cache_hit,
        };

        // Log if enabled
        if self.log_predictions {
            let log = PredictionLog {
                model_name: model_name.to_string(),
                model_version: actual_version,
                timestamp: metadata.timestamp,
                num_samples: metadata.num_samples,
                inference_time_us: metadata.inference_time_us,
            };
            self.prediction_logs.lock().push(log);
        }

        Ok(PredictionResult::new(predictions).with_metadata(metadata))
    }

    // Batch prediction with optimization
    pub fn predict_batch(
        &self,
        model_name: &str,
        model_version: Option<ModelVersion>,
        features: &Matrix,
    ) -> Result<PredictionResult> {
        self.predict(model_name, model_version, features)
    }

    // Predict with confidence scores
    pub fn predict_with_confidence(
        &self,
        model_name: &str,
        model_version: Option<ModelVersion>,
        features: &Matrix,
    ) -> Result<PredictionResult> {
        let mut result = self.predict(model_name, model_version, features)?;

        // Generate confidence scores (simplified - in practice this would be model-specific)
        let confidence_scores = vec![0.95; result.predictions.len()];
        result.confidence = Some(ConfidenceScore::new(confidence_scores));

        Ok(result)
    }

    // Predict with explanation (feature importance)
    pub fn predict_with_explanation(
        &self,
        model_name: &str,
        model_version: Option<ModelVersion>,
        features: &Matrix,
    ) -> Result<PredictionResult> {
        let mut result = self.predict(model_name, model_version, features)?;

        // Get model for feature importance
        let stored = self.registry.get(model_name, model_version)?;

        // Extract feature importance from model if supported
        let importance_scores = match stored.metadata.model_type {
            ModelType::LinearRegression => {
                let model: LinearRegression =
                    serde_json::from_slice(&stored.model_data).map_err(|e| {
                        MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                    })?;
                model.feature_importance()
            }
            ModelType::LogisticRegression => {
                let model: LogisticRegression = serde_json::from_slice(&stored.model_data)
                    .map_err(|e| {
                        MLError::PredictionFailed(format!("Deserialization failed: {}", e))
                    })?;
                model.feature_importance()
            }
            _ => None,
        };

        if let Some(scores) = importance_scores {
            result.feature_importance = Some(FeatureImportance::new(
                stored.metadata.feature_names.clone(),
                scores,
            ));
        }

        Ok(result)
    }

    // Warm up cache with frequently used models
    pub fn warmup(&self, model_names: Vec<String>) -> Result<()> {
        self.cache.warmup(&self.registry, model_names)
    }

    // Get cache statistics
    pub fn cache_stats(&self) -> (u64, u64, u64) {
        self.cache.stats()
    }

    // Get prediction logs
    #[allow(dead_code)]
    pub fn get_logs(&self) -> Vec<PredictionLog> {
        self.prediction_logs.lock().clone()
    }

    // Clear prediction logs
    pub fn clear_logs(&self) {
        self.prediction_logs.lock().clear();
    }

    // Get average inference time
    pub fn average_inference_time(&self) -> Duration {
        let logs = self.prediction_logs.lock();
        if logs.is_empty() {
            Duration::from_micros(0)
        } else {
            let total_us: u64 = logs.iter().map(|log| log.inference_time_us).sum();
            Duration::from_micros(total_us / logs.len() as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::algorithms::ModelType;
    use crate::ml::engine::{ModelMetadata, StoredModel};

    #[test]
    fn test_model_cache() {
        let cache = ModelCache::new(10); // 10 MB

        let metadata = ModelMetadata::new(
            "test".to_string(),
            ModelType::LinearRegression,
            vec!["f1".to_string()],
        );
        let model = StoredModel::new(metadata, vec![1, 2, 3]);

        cache.put("test:1.0.0".to_string(), &model);
        assert_eq!(cache.count(), 1);

        let result = cache.get("test:1.0.0");
        assert!(result.is_some());

        let (hits, misses, _) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 0);
    }

    #[test]
    fn test_confidence_score() {
        let scores = vec![0.9, 0.8, 0.95];
        let confidence = ConfidenceScore::new(scores);

        assert!((confidence.average() - 0.883).abs() < 0.01);
        assert_eq!(confidence.min(), 0.8);
        assert_eq!(confidence.max(), 0.95);
    }

    #[test]
    fn test_feature_importance() {
        let names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];
        let scores = vec![0.5, 0.3, 0.2];

        let importance = FeatureImportance::new(names, scores);
        assert_eq!(importance.get_importance("f1"), Some(0.5));

        let top = importance.top_k(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "f1");
    }
}
