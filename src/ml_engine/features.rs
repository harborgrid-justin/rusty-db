// # Feature Engineering
//
// Advanced feature preprocessing, transformation, and selection for ML models.
// Supports automatic feature extraction from SQL tables and zero-copy transformations.

use crate::error::Result;
use super::Dataset;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Feature Transformation Types
// ============================================================================

/// Feature transformation pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturePipeline {
    /// Ordered list of transformations
    transformations: Vec<Transformation>,
    /// Feature metadata
    metadata: FeatureMetadata,
}

impl FeaturePipeline {
    pub fn new() -> Self {
        Self {
            transformations: Vec::new(),
            metadata: FeatureMetadata::new(),
        }
    }

    pub fn add_transformation(&mut self, transform: Transformation) {
        self.transformations.push(transform);
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        let mut transformed = dataset.clone();

        for transform in &mut self.transformations {
            transformed = transform.fit_transform(&transformed)?;
        }

        self.metadata = FeatureMetadata::from_dataset(&transformed);

        Ok(transformed)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        let mut transformed = dataset.clone();

        for transform in &self.transformations {
            transformed = transform.transform(&transformed)?;
        }

        Ok(transformed)
    }
}

impl Default for FeaturePipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual feature transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transformation {
    Normalization(Normalizer),
    Standardization(Standardizer),
    OneHotEncoding(OneHotEncoder),
    Binning(Binner),
    Imputation(Imputer),
    PolynomialFeatures(PolynomialTransform),
    VarianceThreshold(VarianceThresholdSelector),
    CorrelationFilter(CorrelationSelector),
}

impl Transformation {
    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        match self {
            Transformation::Normalization(t) => t.fit_transform(dataset),
            Transformation::Standardization(t) => t.fit_transform(dataset),
            Transformation::OneHotEncoding(t) => t.fit_transform(dataset),
            Transformation::Binning(t) => t.fit_transform(dataset),
            Transformation::Imputation(t) => t.fit_transform(dataset),
            Transformation::PolynomialFeatures(t) => t.fit_transform(dataset),
            Transformation::VarianceThreshold(t) => t.fit_transform(dataset),
            Transformation::CorrelationFilter(t) => t.fit_transform(dataset),
        }
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        match self {
            Transformation::Normalization(t) => t.transform(dataset),
            Transformation::Standardization(t) => t.transform(dataset),
            Transformation::OneHotEncoding(t) => t.transform(dataset),
            Transformation::Binning(t) => t.transform(dataset),
            Transformation::Imputation(t) => t.transform(dataset),
            Transformation::PolynomialFeatures(t) => t.transform(dataset),
            Transformation::VarianceThreshold(t) => t.transform(dataset),
            Transformation::CorrelationFilter(t) => t.transform(dataset),
        }
    }
}

// ============================================================================
// Normalization (Min-Max Scaling)
// ============================================================================

/// Min-Max normalization to [0, 1] range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Normalizer {
    /// Minimum values per feature
    min_values: Vec<f64>,
    /// Maximum values per feature
    max_values: Vec<f64>,
    /// Whether the normalizer is fitted
    fitted: bool,
}

impl Normalizer {
    pub fn new() -> Self {
        Self {
            min_values: Vec::new(),
            max_values: Vec::new(),
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let n_features = dataset.num_features();
        self.min_values = vec![f64::INFINITY; n_features];
        self.max_values = vec![f64::NEG_INFINITY; n_features];

        for sample in &dataset.features {
            for (i, &value) in sample.iter().enumerate() {
                if value < self.min_values[i] {
                    self.min_values[i] = value;
                }
                if value > self.max_values[i] {
                    self.max_values[i] = value;
                }
            }
        }

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("Normalizer not fitted".into()));
        }

        let transformed_features: Vec<Vec<f64>> = dataset.features.iter()
            .map(|sample| {
                sample.iter()
                    .enumerate()
                    .map(|(i, &value)| {
                        let range = self.max_values[i] - self.min_values[i];
                        if range.abs() < 1e-10 {
                            0.5 // All values are the same
                        } else {
                            (value - self.min_values[i]) / range
                        }
                    })
                    .collect()
            })
            .collect();

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names: dataset.feature_names.clone(),
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Standardization (Z-score)
// ============================================================================

/// Z-score standardization (mean=0, std=1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Standardizer {
    /// Mean values per feature
    means: Vec<f64>,
    /// Standard deviations per feature
    std_devs: Vec<f64>,
    /// Whether the standardizer is fitted
    fitted: bool,
}

impl Standardizer {
    pub fn new() -> Self {
        Self {
            means: Vec::new(),
            std_devs: Vec::new(),
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let n_features = dataset.num_features();
        let n_samples = dataset.num_samples() as f64;

        self.means = vec![0.0; n_features];
        self.std_devs = vec![0.0; n_features];

        // Compute means
        for sample in &dataset.features {
            for (i, &value) in sample.iter().enumerate() {
                self.means[i] += value;
            }
        }
        for mean in &mut self.means {
            *mean /= n_samples;
        }

        // Compute standard deviations
        for sample in &dataset.features {
            for (i, &value) in sample.iter().enumerate() {
                self.std_devs[i] += (value - self.means[i]).powi(2);
            }
        }
        for std_dev in &mut self.std_devs {
            *std_dev = (*std_dev / n_samples).sqrt();
            if std_dev.abs() < 1e-10 {
                *std_dev = 1.0; // Avoid division by zero
            }
        }

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("Standardizer not fitted".into()));
        }

        let transformed_features: Vec<Vec<f64>> = dataset.features.iter()
            .map(|sample| {
                sample.iter()
                    .enumerate()
                    .map(|(i, &value)| (value - self.means[i]) / self.std_devs[i])
                    .collect()
            })
            .collect();

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names: dataset.feature_names.clone(),
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

impl Default for Standardizer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// One-Hot Encoding
// ============================================================================

/// One-hot encoding for categorical features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneHotEncoder {
    /// Categories per feature
    categories: HashMap<usize, Vec<i64>>,
    /// Feature indices to encode
    feature_indices: Vec<usize>,
    /// Whether the encoder is fitted
    fitted: bool,
}

impl OneHotEncoder {
    pub fn new(feature_indices: Vec<usize>) -> Self {
        Self {
            categories: HashMap::new(),
            feature_indices,
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        for &feature_idx in &self.feature_indices {
            let mut categories = Vec::new();
            for sample in &dataset.features {
                let value = sample[feature_idx] as i64;
                if !categories.contains(&value) {
                    categories.push(value);
                }
            }
            categories.sort();
            self.categories.insert(feature_idx, categories);
        }

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("OneHotEncoder not fitted".into()));
        }

        let mut new_feature_names = Vec::new();
        let mut transformed_features = Vec::with_capacity(dataset.num_samples());

        for sample in &dataset.features {
            let mut new_sample = Vec::new();

            for (i, &value) in sample.iter().enumerate() {
                if self.feature_indices.contains(&i) {
                    // One-hot encode this feature
                    if let Some(categories) = self.categories.get(&i) {
                        for &category in categories {
                            new_sample.push(if (value as i64) == category { 1.0 } else { 0.0 });
                        }
                    }
                } else {
                    // Keep original feature
                    new_sample.push(value);
                }
            }

            transformed_features.push(new_sample);
        }

        // Build new feature names
        for (i, name) in dataset.feature_names.iter().enumerate() {
            if self.feature_indices.contains(&i) {
                if let Some(categories) = self.categories.get(&i) {
                    for &category in categories {
                        new_feature_names.push(format!("{}_{}", name, category))));
                    }
                }
            } else {
                new_feature_names.push(name.clone());
            }
        }

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names: new_feature_names,
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

// ============================================================================
// Feature Binning
// ============================================================================

/// Equal-width or equal-frequency binning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binner {
    /// Bin edges per feature
    bin_edges: HashMap<usize, Vec<f64>>,
    /// Feature indices to bin
    feature_indices: Vec<usize>,
    /// Number of bins
    n_bins: usize,
    /// Binning strategy
    strategy: BinningStrategy,
    /// Whether the binner is fitted
    fitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinningStrategy {
    EqualWidth,
    EqualFrequency,
}

impl Binner {
    pub fn new(feature_indices: Vec<usize>, n_bins: usize, strategy: BinningStrategy) -> Self {
        Self {
            bin_edges: HashMap::new(),
            feature_indices,
            n_bins,
            strategy,
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        for &feature_idx in &self.feature_indices {
            let mut values: Vec<f64> = dataset.features.iter()
                .map(|sample| sample[feature_idx])
                .collect();

            values.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let edges = match self.strategy {
                BinningStrategy::EqualWidth => {
                    let min_val = values[0];
                    let max_val = values[values.len() - 1];
                    let width = (max_val - min_val) / self.n_bins as f64;

                    (0..=self.n_bins)
                        .map(|i| min_val + i as f64 * width)
                        .collect()
                }
                BinningStrategy::EqualFrequency => {
                    let n_samples = values.len();
                    let samples_per_bin = n_samples / self.n_bins;

                    (0..=self.n_bins)
                        .map(|i| {
                            let idx = (i * samples_per_bin).min(n_samples - 1);
                            values[idx]
                        })
                        .collect()
                }
            };

            self.bin_edges.insert(feature_idx, edges);
        }

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("Binner not fitted".into()));
        }

        let transformed_features: Vec<Vec<f64>> = dataset.features.iter()
            .map(|sample| {
                sample.iter()
                    .enumerate()
                    .map(|(i, &value)| {
                        if self.feature_indices.contains(&i) {
                            if let Some(edges) = self.bin_edges.get(&i) {
                                // Find which bin the value belongs to
                                for (bin_idx, window) in edges.windows(2).enumerate() {
                                    if value >= window[0] && value <= window[1] {
                                        return bin_idx as f64;
                                    }
                                }
                                return (edges.len() - 2) as f64; // Last bin
                            }
                        }
                        value
                    })
                    .collect()
            })
            .collect();

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names: dataset.feature_names.iter()
                .enumerate()
                .map(|(i, name)| {
                    if self.feature_indices.contains(&i) {
                        format!("{}_binned", name)
                    } else {
                        name.clone()
                    }
                })
                .collect(),
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

// ============================================================================
// Missing Value Imputation
// ============================================================================

/// Imputation strategies for missing values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imputer {
    /// Imputation values per feature
    fill_values: Vec<f64>,
    /// Imputation strategy
    strategy: ImputationStrategy,
    /// Missing value indicator
    missing_value: f64,
    /// Whether the imputer is fitted
    fitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImputationStrategy {
    Mean,
    Median,
    Mode,
    Constant,
}

impl Imputer {
    pub fn new(strategy: ImputationStrategy, missing_value: f64) -> Self {
        Self {
            fill_values: Vec::new(),
            strategy,
            missing_value,
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let n_features = dataset.num_features()));
        self.fill_values = vec![0.0; n_features];

        for i in 0..n_features {
            let mut values: Vec<f64> = dataset.features.iter()
                .map(|sample| sample[i])
                .filter(|&v| (v - self.missing_value).abs() > 1e-10)
                .collect();

            if values.is_empty() {
                self.fill_values[i] = 0.0;
                continue;
            }

            self.fill_values[i] = match self.strategy {
                ImputationStrategy::Mean => {
                    values.iter().sum::<f64>() / values.len() as f64
                }
                ImputationStrategy::Median => {
                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let mid = values.len() / 2;
                    if values.len() % 2 == 0 {
                        (values[mid - 1] + values[mid]) / 2.0
                    } else {
                        values[mid]
                    }
                }
                ImputationStrategy::Mode => {
                    let mut counts: HashMap<i64, usize> = HashMap::new();
                    for &v in &values {
                        *counts.entry(v as i64).or_insert(0) += 1;
                    }
                    counts.into_iter()
                        .max_by_key(|(_, count)| *count)
                        .map(|(val, _)| val as f64)
                        .unwrap_or(0.0)
                }
                ImputationStrategy::Constant => 0.0,
            };
        }

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("Imputer not fitted".into()));
        }

        let transformed_features: Vec<Vec<f64>> = dataset.features.iter()
            .map(|sample| {
                sample.iter()
                    .enumerate()
                    .map(|(i, &value)| {
                        if (value - self.missing_value).abs() < 1e-10 {
                            self.fill_values[i]
                        } else {
                            value
                        }
                    })
                    .collect()
            })
            .collect();

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names: dataset.feature_names.clone(),
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

// ============================================================================
// Polynomial Features
// ============================================================================

/// Generate polynomial and interaction features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolynomialTransform {
    /// Polynomial degree
    degree: usize,
    /// Include interaction terms
    interaction_only: bool,
    /// Include bias term
    include_bias: bool,
}

impl PolynomialTransform {
    pub fn new(degree: usize, interaction_only: bool, include_bias: bool) -> Self {
        Self {
            degree,
            interaction_only,
            include_bias,
        }
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        let n_features = dataset.num_features();
        let mut transformed_features = Vec::with_capacity(dataset.num_samples());
        let mut feature_names = Vec::new();

        if self.include_bias {
            feature_names.push("bias".to_string());
        }

        // Original features
        for name in &dataset.feature_names {
            feature_names.push(name.clone());
        }

        // Generate polynomial features
        for sample in &dataset.features {
            let mut new_sample = Vec::new();

            if self.include_bias {
                new_sample.push(1.0);
            }

            // Degree 1 (original features)
            new_sample.extend_from_slice(sample);

            // Higher degrees
            if self.degree >= 2 {
                // Degree 2
                for i in 0..n_features {
                    for j in i..n_features {
                        if !self.interaction_only || i != j {
                            new_sample.push(sample[i] * sample[j]);
                        }
                    }
                }
            }

            transformed_features.push(new_sample);
        }

        // Generate feature names for degree 2
        if self.degree >= 2 {
            for i in 0..n_features {
                for j in i..n_features {
                    if !self.interaction_only || i != j {
                        feature_names.push(format!(
                            "{}*{}",
                            dataset.feature_names[i],
                            dataset.feature_names[j]
                        ))));
                    }
                }
            }
        }

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names,
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

// ============================================================================
// Variance Threshold Selector
// ============================================================================

/// Remove low-variance features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceThresholdSelector {
    /// Variance threshold
    threshold: f64,
    /// Selected feature indices
    selected_features: Vec<usize>,
    /// Feature variances
    variances: Vec<f64>,
    /// Whether the selector is fitted
    fitted: bool,
}

impl VarianceThresholdSelector {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            selected_features: Vec::new(),
            variances: Vec::new(),
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let n_features = dataset.num_features();
        let n_samples = dataset.num_samples() as f64;

        self.variances = Vec::with_capacity(n_features);

        for i in 0..n_features {
            let values: Vec<f64> = dataset.features.iter().map(|s| s[i]).collect();
            let mean = values.iter().sum::<f64>() / n_samples;
            let variance = values.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / n_samples;

            self.variances.push(variance);

            if variance >= self.threshold {
                self.selected_features.push(i);
            }
        }

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("Selector not fitted".into()));
        }

        let transformed_features: Vec<Vec<f64>> = dataset.features.iter()
            .map(|sample| {
                self.selected_features.iter()
                    .map(|&i| sample[i])
                    .collect()
            })
            .collect();

        let feature_names = self.selected_features.iter()
            .map(|&i| dataset.feature_names[i].clone())
            .collect();

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names,
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }
}

// ============================================================================
// Correlation-based Feature Selection
// ============================================================================

/// Remove highly correlated features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationSelector {
    /// Correlation threshold
    threshold: f64,
    /// Selected feature indices
    selected_features: Vec<usize>,
    /// Whether the selector is fitted
    fitted: bool,
}

impl CorrelationSelector {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            selected_features: Vec::new(),
            fitted: false,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let n_features = dataset.num_features();
        let mut keep = vec![true; n_features];

        for i in 0..n_features {
            if !keep[i] {
                continue;
            }

            for j in (i + 1)..n_features {
                if !keep[j] {
                    continue;
                }

                let correlation = self.compute_correlation(dataset, i, j);
                if correlation.abs() > self.threshold {
                    keep[j] = false;
                }
            }
        }

        self.selected_features = keep.iter()
            .enumerate()
            .filter(|(_, &k)| k)
            .map(|(i, _)| i)
            .collect();

        self.fitted = true;
        Ok(())
    }

    pub fn fit_transform(&mut self, dataset: &Dataset) -> Result<Dataset> {
        self.fit(dataset)?;
        self.transform(dataset)
    }

    pub fn transform(&self, dataset: &Dataset) -> Result<Dataset> {
        if !self.fitted {
            return Err(DbError::InvalidInput("Selector not fitted".into()));
        }

        let transformed_features: Vec<Vec<f64>> = dataset.features.iter()
            .map(|sample| {
                self.selected_features.iter()
                    .map(|&i| sample[i])
                    .collect()
            })
            .collect();

        let feature_names = self.selected_features.iter()
            .map(|&i| dataset.feature_names[i].clone())
            .collect();

        Ok(Dataset {
            features: transformed_features,
            targets: dataset.targets.clone(),
            feature_names,
            target_name: dataset.target_name.clone(),
            weights: dataset.weights.clone(),
        })
    }

    fn compute_correlation(&self, dataset: &Dataset, i: usize, j: usize) -> f64 {
        let n = dataset.num_samples() as f64;

        let values_i: Vec<f64> = dataset.features.iter().map(|s| s[i]).collect();
        let values_j: Vec<f64> = dataset.features.iter().map(|s| s[j]).collect();

        let mean_i = values_i.iter().sum::<f64>() / n;
        let mean_j = values_j.iter().sum::<f64>() / n;

        let cov: f64 = values_i.iter()
            .zip(&values_j)
            .map(|(&vi, &vj)| (vi - mean_i) * (vj - mean_j))
            .sum::<f64>() / n;

        let std_i = (values_i.iter().map(|&v| (v - mean_i).powi(2)).sum::<f64>() / n).sqrt();
        let std_j = (values_j.iter().map(|&v| (v - mean_j).powi(2)).sum::<f64>() / n).sqrt();

        if std_i.abs() < 1e-10 || std_j.abs() < 1e-10 {
            0.0
        } else {
            cov / (std_i * std_j)
        }
    }
}

// ============================================================================
// Feature Metadata
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMetadata {
    pub num_features: usize,
    pub feature_names: Vec<String>,
    pub feature_types: Vec<FeatureType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureType {
    Numeric,
    Categorical,
    Binary,
    Ordinal,
}

impl FeatureMetadata {
    pub fn new() -> Self {
        Self {
            num_features: 0,
            feature_names: Vec::new(),
            feature_types: Vec::new(),
        }
    }

    pub fn from_dataset(dataset: &Dataset) -> Self {
        Self {
            num_features: dataset.num_features(),
            feature_names: dataset.feature_names.clone(),
            feature_types: vec![FeatureType::Numeric; dataset.num_features()],
        }
    }
}

impl Default for FeatureMetadata {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Feature Engine
// ============================================================================

/// Main feature engineering coordinator
pub struct FeatureEngine {
    pipelines: HashMap<String, FeaturePipeline>,
}

impl FeatureEngine {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    pub fn create_pipeline(&mut self, name: String, pipeline: FeaturePipeline) {
        self.pipelines.insert(name, pipeline);
    }

    pub fn get_pipeline(&self, name: &str) -> Option<&FeaturePipeline> {
        self.pipelines.get(name)
    }

    pub fn get_pipeline_mut(&mut self, name: &str) -> Option<&mut FeaturePipeline> {
        self.pipelines.get_mut(name)
    }
}

impl Default for FeatureEngine {
    fn default() -> Self {
        Self::new()
    }
}
