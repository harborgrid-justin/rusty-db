//! # Data Preprocessing and Feature Engineering
//!
//! This module provides comprehensive data preprocessing capabilities for ML pipelines,
//! including feature scaling, encoding, selection, and transformation.

use crate::error::Result;
use super::{Dataset, Vector, Matrix, FeatureNames, MLError};
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

// ============================================================================
// Preprocessor Trait
// ============================================================================

/// Common trait for all preprocessors
pub trait Preprocessor: Send + Sync {
    /// Fit the preprocessor to the data
    fn fit(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<()>;

    /// Transform the data
    fn transform(&self, features: &Matrix) -> Result<Matrix>;

    /// Fit and transform in one step
    fn fit_transform(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<Matrix> {
        self.fit(features, feature_names)?;
        self.transform(features)
    }

    /// Check if preprocessor is fitted
    fn is_fitted(&self) -> bool;

    /// Get output feature names after transformation
    fn get_feature_names(&self) -> FeatureNames;
}

// ============================================================================
// Scaling
// ============================================================================

/// Scaling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingStrategy {
    /// Standardization (z-score normalization)
    Standard,
    /// Min-max normalization to [0, 1]
    MinMax,
    /// Robust scaling using median and IQR
    Robust,
    /// Max absolute scaling
    MaxAbs,
}

/// Generic scaler trait
pub trait Scaler: Preprocessor {
    /// Get scaling strategy
    fn strategy(&self) -> ScalingStrategy;
}

/// Standard scaler (z-score normalization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardScaler {
    /// Feature means
    means: Vector,
    /// Feature standard deviations
    std_devs: Vector,
    /// Feature names
    feature_names: FeatureNames,
    /// Whether the scaler is fitted
    fitted: bool,
}

impl StandardScaler {
    /// Create a new standard scaler
    pub fn new() -> Self {
        Self {
            means: Vec::new(),
            std_devs: Vec::new(),
            feature_names: Vec::new(),
            fitted: false,
        }
    }

    /// Calculate mean of a column
    fn calculate_mean(features: &Matrix, col: usize) -> f64 {
        features.iter().map(|row| row[col]).sum::<f64>() / features.len() as f64
    }

    /// Calculate standard deviation of a column
    fn calculate_std(features: &Matrix, col: usize, mean: f64) -> f64 {
        let variance = features.iter()
            .map(|row| (row[col] - mean).powi(2))
            .sum::<f64>() / features.len() as f64;
        variance.sqrt().max(1e-10) // Avoid division by zero
    }
}

impl Default for StandardScaler {
    fn default() -> Self {
        Self::new()
    }
}

impl Preprocessor for StandardScaler {
    fn fit(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<()> {
        if features.is_empty() {
            return Err(MLError::InsufficientData("Empty feature matrix".to_string()).into());
        }

        let n_features = features[0].len();
        self.means = Vec::with_capacity(n_features);
        self.std_devs = Vec::with_capacity(n_features);

        for col in 0..n_features {
            let mean = Self::calculate_mean(features, col);
            let std = Self::calculate_std(features, col, mean);
            self.means.push(mean);
            self.std_devs.push(std);
        }

        self.feature_names = feature_names.clone();
        self.fitted = true;
        Ok(())
    }

    fn transform(&self, features: &Matrix) -> Result<Matrix> {
        if !self.fitted {
            return Err(MLError::InvalidConfiguration("Scaler not fitted".to_string()).into());
        }

        Ok(features.iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, &value)| {
                        (value - self.means[i]) / self.std_devs[i]
                    })
                    .collect()
            })
            .collect())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }

    fn get_feature_names(&self) -> FeatureNames {
        self.feature_names.clone()
    }
}

impl Scaler for StandardScaler {
    fn strategy(&self) -> ScalingStrategy {
        ScalingStrategy::Standard
    }
}

/// Min-max scaler (normalize to [0, 1])
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinMaxScaler {
    /// Feature minimums
    mins: Vector,
    /// Feature maximums
    maxs: Vector,
    /// Feature names
    feature_names: FeatureNames,
    /// Target range
    feature_range: (f64, f64),
    /// Whether the scaler is fitted
    fitted: bool,
}

impl MinMaxScaler {
    /// Create a new min-max scaler with default range [0, 1]
    pub fn new() -> Self {
        Self::with_range(0.0, 1.0)
    }

    /// Create a new min-max scaler with custom range
    pub fn with_range(min: f64, max: f64) -> Self {
        Self {
            mins: Vec::new(),
            maxs: Vec::new(),
            feature_names: Vec::new(),
            feature_range: (min, max),
            fitted: false,
        }
    }

    /// Calculate min and max of a column
    fn calculate_min_max(features: &Matrix, col: usize) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for row in features {
            let value = row[col];
            if value < min {
                min = value;
            }
            if value > max {
                max = value;
            }
        }

        (min, max)
    }
}

impl Default for MinMaxScaler {
    fn default() -> Self {
        Self::new()
    }
}

impl Preprocessor for MinMaxScaler {
    fn fit(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<()> {
        if features.is_empty() {
            return Err(MLError::InsufficientData("Empty feature matrix".to_string()).into());
        }

        let n_features = features[0].len();
        self.mins = Vec::with_capacity(n_features);
        self.maxs = Vec::with_capacity(n_features);

        for col in 0..n_features {
            let (min, max) = Self::calculate_min_max(features, col);
            self.mins.push(min);
            self.maxs.push(max);
        }

        self.feature_names = feature_names.clone();
        self.fitted = true;
        Ok(())
    }

    fn transform(&self, features: &Matrix) -> Result<Matrix> {
        if !self.fitted {
            return Err(MLError::InvalidConfiguration("Scaler not fitted".to_string()).into());
        }

        let (target_min, target_max) = self.feature_range;
        let target_range = target_max - target_min;

        Ok(features.iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, &value)| {
                        let range = self.maxs[i] - self.mins[i];
                        if range.abs() < 1e-10 {
                            target_min
                        } else {
                            target_min + ((value - self.mins[i]) / range) * target_range
                        }
                    })
                    .collect()
            })
            .collect())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }

    fn get_feature_names(&self) -> FeatureNames {
        self.feature_names.clone()
    }
}

impl Scaler for MinMaxScaler {
    fn strategy(&self) -> ScalingStrategy {
        ScalingStrategy::MinMax
    }
}

// ============================================================================
// Encoding
// ============================================================================

/// Categorical encoder trait
pub trait Encoder: Preprocessor {
    /// Handle unknown categories during transform
    fn handle_unknown(&self) -> UnknownHandling;
}

/// Strategy for handling unknown categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnknownHandling {
    /// Raise an error
    Error,
    /// Ignore (set all encoded values to 0)
    Ignore,
    /// Use a default value
    UseDefault,
}

/// One-hot encoder for categorical features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneHotEncoder {
    /// Categories per feature
    categories: Vec<Vec<String>>,
    /// Original feature names
    original_feature_names: FeatureNames,
    /// Output feature names
    output_feature_names: FeatureNames,
    /// Unknown handling strategy
    handle_unknown: UnknownHandling,
    /// Whether the encoder is fitted
    fitted: bool,
}

impl OneHotEncoder {
    /// Create a new one-hot encoder
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
            original_feature_names: Vec::new(),
            output_feature_names: Vec::new(),
            handle_unknown: UnknownHandling::Error,
            fitted: false,
        }
    }

    /// Create encoder with custom unknown handling
    pub fn with_unknown_handling(mut self, strategy: UnknownHandling) -> Self {
        self.handle_unknown = strategy;
        self
    }

    /// Fit on categorical data (strings converted to numbers)
    pub fn fit_categorical(&mut self, data: &[Vec<String>], feature_names: &FeatureNames) -> Result<()> {
        if data.is_empty() {
            return Err(MLError::InsufficientData("Empty data".to_string()).into());
        }

        let n_features = data[0].len();
        self.categories.clear();
        self.output_feature_names.clear();

        // Extract unique categories per feature
        for col in 0..n_features {
            let mut unique_categories: HashSet<String> = HashSet::new();
            for row in data {
                unique_categories.insert(row[col].clone());
            }

            let mut categories: Vec<String> = unique_categories.into_iter().collect();
            categories.sort();

            // Generate feature names for each category
            for category in &categories {
                let feature_name = format!("{}_{}", feature_names[col], category);
                self.output_feature_names.push(feature_name);
            }

            self.categories.push(categories);
        }

        self.original_feature_names = feature_names.clone();
        self.fitted = true;
        Ok(())
    }

    /// Transform categorical data to one-hot encoded matrix
    pub fn transform_categorical(&self, data: &[Vec<String>]) -> Result<Matrix> {
        if !self.fitted {
            return Err(MLError::InvalidConfiguration("Encoder not fitted".to_string()).into());
        }

        let mut result = Vec::new();

        for row in data {
            let mut encoded_row = Vec::new();

            for (col, value) in row.iter().enumerate() {
                let categories = &self.categories[col];

                if let Some(pos) = categories.iter().position(|c| c == value) {
                    // Create one-hot encoding for this feature
                    for i in 0..categories.len() {
                        encoded_row.push(if i == pos { 1.0 } else { 0.0 });
                    }
                } else {
                    // Handle unknown category
                    match self.handle_unknown {
                        UnknownHandling::Error => {
                            return Err(MLError::InvalidConfiguration(
                                format!("Unknown category '{}' in feature {}", value, col)
                            ).into());
                        }
                        UnknownHandling::Ignore | UnknownHandling::UseDefault => {
                            // All zeros
                            for _ in 0..categories.len() {
                                encoded_row.push(0.0);
                            }
                        }
                    }
                }
            }

            result.push(encoded_row);
        }

        Ok(result)
    }
}

impl Default for OneHotEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Preprocessor for OneHotEncoder {
    fn fit(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<()> {
        // Convert numeric features to strings for categorical encoding
        let categorical_data: Vec<Vec<String>> = features.iter()
            .map(|row| row.iter().map(|&v| v.to_string()).collect())
            .collect();

        self.fit_categorical(&categorical_data, feature_names)
    }

    fn transform(&self, features: &Matrix) -> Result<Matrix> {
        let categorical_data: Vec<Vec<String>> = features.iter()
            .map(|row| row.iter().map(|&v| v.to_string()).collect())
            .collect();

        self.transform_categorical(&categorical_data)
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }

    fn get_feature_names(&self) -> FeatureNames {
        self.output_feature_names.clone()
    }
}

impl Encoder for OneHotEncoder {
    fn handle_unknown(&self) -> UnknownHandling {
        self.handle_unknown
    }
}

// ============================================================================
// Missing Value Imputation
// ============================================================================

/// Strategy for imputing missing values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImputationStrategy {
    /// Replace with mean
    Mean,
    /// Replace with median
    Median,
    /// Replace with most frequent value
    MostFrequent,
    /// Replace with constant value
    Constant,
    /// Forward fill
    ForwardFill,
    /// Backward fill
    BackwardFill,
}

/// Imputer for handling missing values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imputer {
    /// Imputation strategy
    strategy: ImputationStrategy,
    /// Fill values per feature
    fill_values: Vector,
    /// Constant fill value (for Constant strategy)
    constant_value: f64,
    /// Feature names
    feature_names: FeatureNames,
    /// Whether the imputer is fitted
    fitted: bool,
}

impl Imputer {
    /// Create a new imputer with given strategy
    pub fn new(strategy: ImputationStrategy) -> Self {
        Self {
            strategy,
            fill_values: Vec::new(),
            constant_value: 0.0,
            feature_names: Vec::new(),
            fitted: false,
        }
    }

    /// Create imputer with constant fill value
    pub fn with_constant(value: f64) -> Self {
        Self {
            strategy: ImputationStrategy::Constant,
            fill_values: Vec::new(),
            constant_value: value,
            feature_names: Vec::new(),
            fitted: false,
        }
    }

    /// Calculate fill value for a column based on strategy
    fn calculate_fill_value(&self, features: &Matrix, col: usize) -> f64 {
        match self.strategy {
            ImputationStrategy::Mean => {
                let values: Vec<f64> = features.iter()
                    .map(|row| row[col])
                    .filter(|&v| !v.is_nan())
                    .collect();
                if values.is_empty() {
                    0.0
                } else {
                    values.iter().sum::<f64>() / values.len() as f64
                }
            }
            ImputationStrategy::Median => {
                let mut values: Vec<f64> = features.iter()
                    .map(|row| row[col])
                    .filter(|&v| !v.is_nan())
                    .collect();
                if values.is_empty() {
                    0.0
                } else {
                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let mid = values.len() / 2;
                    if values.len() % 2 == 0 {
                        (values[mid - 1] + values[mid]) / 2.0
                    } else {
                        values[mid]
                    }
                }
            }
            ImputationStrategy::MostFrequent => {
                let mut counts: HashMap<i64, usize> = HashMap::new();
                for row in features {
                    let value = row[col];
                    if !value.is_nan() {
                        *counts.entry(value as i64).or_insert(0) += 1;
                    }
                }
                counts.into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(value, _)| value as f64)
                    .unwrap_or(0.0)
            }
            ImputationStrategy::Constant => self.constant_value,
            ImputationStrategy::ForwardFill | ImputationStrategy::BackwardFill => {
                // These are handled differently in transform
                0.0
            }
        }
    }
}

impl Preprocessor for Imputer {
    fn fit(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<()> {
        if features.is_empty() {
            return Err(MLError::InsufficientData("Empty feature matrix".to_string()).into());
        }

        let n_features = features[0].len();
        self.fill_values.clear();

        for col in 0..n_features {
            let fill_value = self.calculate_fill_value(features, col);
            self.fill_values.push(fill_value);
        }

        self.feature_names = feature_names.clone();
        self.fitted = true;
        Ok(())
    }

    fn transform(&self, features: &Matrix) -> Result<Matrix> {
        if !self.fitted {
            return Err(MLError::InvalidConfiguration("Imputer not fitted".to_string()).into());
        }

        match self.strategy {
            ImputationStrategy::ForwardFill => {
                let mut result = features.clone();
                let n_features = result[0].len();

                for col in 0..n_features {
                    let mut last_valid = self.fill_values[col];
                    for row in &mut result {
                        if row[col].is_nan() {
                            row[col] = last_valid;
                        } else {
                            last_valid = row[col];
                        }
                    }
                }
                Ok(result)
            }
            ImputationStrategy::BackwardFill => {
                let mut result = features.clone();
                let n_features = result[0].len();

                for col in 0..n_features {
                    let mut last_valid = self.fill_values[col];
                    for row in result.iter_mut().rev() {
                        if row[col].is_nan() {
                            row[col] = last_valid;
                        } else {
                            last_valid = row[col];
                        }
                    }
                }
                Ok(result)
            }
            _ => {
                Ok(features.iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(|(i, &value)| {
                                if value.is_nan() {
                                    self.fill_values[i]
                                } else {
                                    value
                                }
                            })
                            .collect()
                    })
                    .collect())
            }
        }
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }

    fn get_feature_names(&self) -> FeatureNames {
        self.feature_names.clone()
    }
}

// ============================================================================
// Feature Selection
// ============================================================================

/// Feature selection methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMethod {
    /// Select top k features by variance
    VarianceThreshold,
    /// Select top k features by correlation with target
    SelectKBest,
    /// Select features by percentile
    SelectPercentile,
}

/// Feature selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSelector {
    /// Selection method
    method: SelectionMethod,
    /// Selected feature indices
    selected_indices: Vec<usize>,
    /// Original feature names
    original_feature_names: FeatureNames,
    /// Selected feature names
    selected_feature_names: FeatureNames,
    /// Threshold or k value
    threshold: f64,
    /// Whether the selector is fitted
    fitted: bool,
}

impl FeatureSelector {
    /// Create a new feature selector
    pub fn new(method: SelectionMethod, threshold: f64) -> Self {
        Self {
            method,
            selected_indices: Vec::new(),
            original_feature_names: Vec::new(),
            selected_feature_names: Vec::new(),
            threshold,
            fitted: false,
        }
    }

    /// Fit selector with target values
    pub fn fit_with_target(&mut self, features: &Matrix, target: &Vector, feature_names: &FeatureNames) -> Result<()> {
        if features.is_empty() {
            return Err(MLError::InsufficientData("Empty feature matrix".to_string()).into());
        }

        let n_features = features[0].len();
        let mut scores = Vec::new();

        match self.method {
            SelectionMethod::VarianceThreshold => {
                // Calculate variance for each feature
                for col in 0..n_features {
                    let mean = features.iter().map(|row| row[col]).sum::<f64>() / features.len() as f64;
                    let variance = features.iter()
                        .map(|row| (row[col] - mean).powi(2))
                        .sum::<f64>() / features.len() as f64;
                    scores.push((col, variance));
                }
            }
            SelectionMethod::SelectKBest | SelectionMethod::SelectPercentile => {
                // Calculate correlation with target for each feature
                let target_mean = target.iter().sum::<f64>() / target.len() as f64;
                let target_std = (target.iter()
                    .map(|&y| (y - target_mean).powi(2))
                    .sum::<f64>() / target.len() as f64).sqrt();

                for col in 0..n_features {
                    let feature_mean = features.iter().map(|row| row[col]).sum::<f64>() / features.len() as f64;
                    let feature_std = (features.iter()
                        .map(|row| (row[col] - feature_mean).powi(2))
                        .sum::<f64>() / features.len() as f64).sqrt();

                    let correlation = if feature_std.abs() < 1e-10 || target_std.abs() < 1e-10 {
                        0.0
                    } else {
                        features.iter()
                            .zip(target.iter())
                            .map(|(row, &y)| (row[col] - feature_mean) * (y - target_mean))
                            .sum::<f64>() / (features.len() as f64 * feature_std * target_std)
                    };

                    scores.push((col, correlation.abs()));
                }
            }
        }

        // Sort by score descending
        scores.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        // Select features based on method
        self.selected_indices = match self.method {
            SelectionMethod::VarianceThreshold => {
                scores.iter()
                    .filter(|(_, score)| *score >= self.threshold)
                    .map(|(idx, _)| *idx)
                    .collect()
            }
            SelectionMethod::SelectKBest => {
                let k = self.threshold as usize;
                scores.iter().take(k.min(n_features)).map(|(idx, _)| *idx).collect()
            }
            SelectionMethod::SelectPercentile => {
                let k = ((self.threshold / 100.0) * n_features as f64).ceil() as usize;
                scores.iter().take(k.min(n_features)).map(|(idx, _)| *idx).collect()
            }
        };

        self.selected_indices.sort();
        self.original_feature_names = feature_names.clone();
        self.selected_feature_names = self.selected_indices.iter()
            .map(|&idx| feature_names[idx].clone())
            .collect();

        self.fitted = true;
        Ok(())
    }
}

impl Preprocessor for FeatureSelector {
    fn fit(&mut self, features: &Matrix, feature_names: &FeatureNames) -> Result<()> {
        // For unsupervised selection, use variance threshold
        if features.is_empty() {
            return Err(MLError::InsufficientData("Empty feature matrix".to_string()).into());
        }

        let n_features = features[0].len();
        let mut variances = Vec::new();

        for col in 0..n_features {
            let mean = features.iter().map(|row| row[col]).sum::<f64>() / features.len() as f64;
            let variance = features.iter()
                .map(|row| (row[col] - mean).powi(2))
                .sum::<f64>() / features.len() as f64;
            variances.push((col, variance));
        }

        self.selected_indices = variances.iter()
            .filter(|(_, var)| *var >= self.threshold)
            .map(|(idx, _)| *idx)
            .collect();

        self.original_feature_names = feature_names.clone();
        self.selected_feature_names = self.selected_indices.iter()
            .map(|&idx| feature_names[idx].clone())
            .collect();

        self.fitted = true;
        Ok(())
    }

    fn transform(&self, features: &Matrix) -> Result<Matrix> {
        if !self.fitted {
            return Err(MLError::InvalidConfiguration("Selector not fitted".to_string()).into());
        }

        Ok(features.iter()
            .map(|row| {
                self.selected_indices.iter()
                    .map(|&idx| row[idx])
                    .collect()
            })
            .collect())
    }

    fn is_fitted(&self) -> bool {
        self.fitted
    }

    fn get_feature_names(&self) -> FeatureNames {
        self.selected_feature_names.clone()
    }
}

// ============================================================================
// Data Splitting
// ============================================================================

/// Data splitter for train/test split
#[derive(Debug, Clone)]
pub struct DataSplitter;

impl DataSplitter {
    /// Split dataset into train and test sets
    pub fn train_test_split(
        dataset: &Dataset,
        test_size: f64,
        shuffle: bool,
    ) -> Result<(Dataset, Dataset)> {
        if test_size <= 0.0 || test_size >= 1.0 {
            return Err(MLError::InvalidConfiguration(
                format!("test_size must be between 0 and 1, got {}", test_size)
            ).into());
        }

        let n_samples = dataset.num_samples();
        let n_test = (n_samples as f64 * test_size).ceil() as usize;
        let n_train = n_samples - n_test;

        let mut indices: Vec<usize> = (0..n_samples).collect();

        if shuffle {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            indices.shuffle(&mut rng);
        }

        let train_indices = &indices[..n_train];
        let test_indices = &indices[n_train..];

        let train_features: Matrix = train_indices.iter()
            .map(|&i| dataset.features[i].clone())
            .collect();
        let test_features: Matrix = test_indices.iter()
            .map(|&i| dataset.features[i].clone())
            .collect();

        let train_target = dataset.target.as_ref().map(|target| {
            train_indices.iter().map(|&i| target[i]).collect()
        });
        let test_target = dataset.target.as_ref().map(|target| {
            test_indices.iter().map(|&i| target[i]).collect()
        });

        let train_dataset = Dataset::new(
            train_features,
            train_target,
            dataset.feature_names.clone(),
        );
        let test_dataset = Dataset::new(
            test_features,
            test_target,
            dataset.feature_names.clone(),
        );

        Ok((train_dataset, test_dataset))
    }

    /// Create k-fold cross-validation splits
    pub fn k_fold_split(dataset: &Dataset, k: usize, shuffle: bool) -> Result<Vec<(Dataset, Dataset)>> {
        if k < 2 {
            return Err(MLError::InvalidConfiguration(
                format!("k must be at least 2, got {}", k)
            ).into());
        }

        let n_samples = dataset.num_samples();
        if k > n_samples {
            return Err(MLError::InvalidConfiguration(
                format!("k ({}) cannot be greater than number of samples ({})", k, n_samples)
            ).into());
        }

        let mut indices: Vec<usize> = (0..n_samples).collect();

        if shuffle {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            indices.shuffle(&mut rng);
        }

        let fold_size = n_samples / k;
        let mut folds = Vec::new();

        for fold_idx in 0..k {
            let test_start = fold_idx * fold_size;
            let test_end = if fold_idx == k - 1 {
                n_samples
            } else {
                (fold_idx + 1) * fold_size
            };

            let test_indices = &indices[test_start..test_end];
            let train_indices: Vec<usize> = indices.iter()
                .enumerate()
                .filter(|(i, _)| *i < test_start || *i >= test_end)
                .map(|(_, &idx)| idx)
                .collect();

            let train_features: Matrix = train_indices.iter()
                .map(|&i| dataset.features[i].clone())
                .collect();
            let test_features: Matrix = test_indices.iter()
                .map(|&i| dataset.features[i].clone())
                .collect();

            let train_target = dataset.target.as_ref().map(|target| {
                train_indices.iter().map(|&i| target[i]).collect()
            });
            let test_target = dataset.target.as_ref().map(|target| {
                test_indices.iter().map(|&i| target[i]).collect()
            });

            let train_dataset = Dataset::new(
                train_features,
                train_target,
                dataset.feature_names.clone(),
            );
            let test_dataset = Dataset::new(
                test_features,
                test_target,
                dataset.feature_names.clone(),
            );

            folds.push((train_dataset, test_dataset));
        }

        Ok(folds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_scaler() {
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 4.0],
            vec![3.0, 6.0],
        ];
        let feature_names = vec!["f1".to_string(), "f2".to_string()];

        let mut scaler = StandardScaler::new();
        let result = scaler.fit_transform(&features, &feature_names).unwrap();

        // Check that mean is approximately 0
        let mean_col1 = result.iter().map(|row| row[0]).sum::<f64>() / result.len() as f64;
        assert!((mean_col1).abs() < 1e-10);
    }

    #[test]
    fn test_minmax_scaler() {
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 4.0],
            vec![3.0, 6.0],
        ];
        let feature_names = vec!["f1".to_string(), "f2".to_string()];

        let mut scaler = MinMaxScaler::new();
        let result = scaler.fit_transform(&features, &feature_names).unwrap();

        // Check that values are in [0, 1]
        for row in &result {
            for &value in row {
                assert!(value >= 0.0 && value <= 1.0);
            }
        }
    }

    #[test]
    fn test_train_test_split() {
        let features = vec![
            vec![1.0],
            vec![2.0],
            vec![3.0],
            vec![4.0],
            vec![5.0],
        ];
        let target = Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let dataset = Dataset::new(features, target, vec!["x".to_string()]);

        let (train, test) = DataSplitter::train_test_split(&dataset, 0.4, false).unwrap();
        assert_eq!(train.num_samples(), 3);
        assert_eq!(test.num_samples(), 2);
    }
}
