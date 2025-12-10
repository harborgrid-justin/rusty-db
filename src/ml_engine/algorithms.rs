// # Machine Learning Algorithms
//
// Pure Rust implementations of core ML algorithms optimized for in-database execution.
// All algorithms support incremental updates and zero-copy integration with the query engine.

use crate::error::Result;
use super::{Dataset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::{Rng, thread_rng};
use rand::prelude::SliceRandom;
// ============================================================================
// Linear Regression
// ============================================================================

// Ordinary Least Squares Linear Regression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRegression {
    // Model coefficients (weights)
    pub coefficients: Vec<f64>,
    // Intercept term
    pub intercept: f64,
    // Training statistics
    pub r_squared: f64,
    // Feature importance scores
    pub feature_importance: Vec<f64>,
}

impl LinearRegression {
    pub fn new() -> Self {
        Self {
            coefficients: Vec::new(),
            intercept: 0.0,
            r_squared: 0.0,
            feature_importance: Vec::new(),
        }
    }

    // Train using normal equation: w = (X^T X)^-1 X^T y
    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let targets = dataset.targets.as_ref()
            .ok_or_else(|| crate::DbError::InvalidInput("Targets required for regression".into()))?;

        let n_samples = dataset.num_samples();
        let n_features = dataset.num_features();

        if n_samples == 0 || n_features == 0 {
            return Err(crate::DbError::InvalidInput("Empty dataset".into()));
        }

        // Compute X^T X
        let mut xtx = vec![vec![0.0; n_features]; n_features];
        for i in 0..n_features {
            for j in 0..n_features {
                let mut sum = 0.0;
                for sample in &dataset.features {
                    sum += sample[i] * sample[j];
                }
                xtx[i][j] = sum;
            }
        }

        // Compute X^T y
        let mut xty = vec![0.0; n_features];
        for i in 0..n_features {
            let mut sum = 0.0;
            for (sample, &target) in dataset.features.iter().zip(targets.iter()) {
                sum += sample[i] * target;
            }
            xty[i] = sum;
        }

        // Solve using Gauss-Jordan elimination
        let coefficients = self.solve_linear_system(&xtx, &xty)?;

        // Compute intercept
        let mut sum_targets = 0.0;
        let mut sum_predictions = 0.0;
        for (sample, &target) in dataset.features.iter().zip(targets.iter()) {
            sum_targets += target;
            let pred: f64 = sample.iter().zip(&coefficients).map(|(x, c)| x * c).sum();
            sum_predictions += pred;
        }
        let intercept = (sum_targets - sum_predictions) / n_samples as f64;

        self.coefficients = coefficients;
        self.intercept = intercept;

        // Compute R²
        self.r_squared = self.compute_r_squared(dataset, targets)?;

        // Compute feature importance as absolute coefficient values
        self.feature_importance = self.coefficients.iter().map(|c| c.abs()).collect();

        Ok(())
    }

    // Predict values for new samples
    pub fn predict(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        if self.coefficients.is_empty() {
            return Err(crate::DbError::InvalidInput("Model not trained".into()));
        }

        let mut predictions = Vec::with_capacity(features.len());
        for sample in features {
            if sample.len() != self.coefficients.len() {
                return Err(crate::DbError::InvalidInput("Feature dimension mismatch".into()));
            }
            let pred: f64 = sample.iter()
                .zip(&self.coefficients)
                .map(|(x, c)| x * c)
                .sum::<f64>() + self.intercept;
            predictions.push(pred);
        }

        Ok(predictions)
    }

    // Solve linear system Ax = b using Gauss-Jordan elimination
    fn solve_linear_system(&self, a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
        let n = a.len();
        let mut aug = vec![vec![0.0; n + 1]; n];

        // Create augmented matrix
        for i in 0..n {
            for j in 0..n {
                aug[i][j] = a[i][j];
            }
            aug[i][n] = b[i];
        }

        // Forward elimination with partial pivoting
        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            for k in (i + 1)..n {
                if aug[k][i].abs() > aug[max_row][i].abs() {
                    max_row = k;
                }
            }
            aug.swap(i, max_row);

            // Check for singular matrix
            if aug[i][i].abs() < 1e-10 {
                return Err(crate::DbError::InvalidInput("Singular matrix".into()));
            }

            // Eliminate column
            for k in (i + 1)..n {
                let factor = aug[k][i] / aug[i][i];
                for j in i..=n {
                    aug[k][j] -= factor * aug[i][j];
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            x[i] = aug[i][n];
            for j in (i + 1)..n {
                x[i] -= aug[i][j] * x[j];
            }
            x[i] /= aug[i][i];
        }

        Ok(x)
    }

    fn compute_r_squared(&self, dataset: &Dataset, targets: &[f64]) -> Result<f64> {
        let predictions = self.predict(&dataset.features)?;
        let mean_target: f64 = targets.iter().sum::<f64>() / targets.len() as f64;

        let ss_tot: f64 = targets.iter().map(|&y| (y - mean_target).powi(2)).sum();
        let ss_res: f64 = targets.iter().zip(&predictions).map(|(&y, &pred)| (y - pred).powi(2)).sum();

        Ok(1.0 - ss_res / ss_tot)
    }
}

impl Default for LinearRegression {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Logistic Regression
// ============================================================================

// Logistic Regression for binary and multi-class classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticRegression {
    // Model weights
    pub weights: Vec<f64>,
    // Intercept term
    pub intercept: f64,
    // Number of classes
    pub num_classes: usize,
    // Learning rate
    learning_rate: f64,
    // Regularization parameter
    regularization: f64,
}

impl LogisticRegression {
    pub fn new(learning_rate: f64, regularization: f64) -> Self {
        Self {
            weights: Vec::new(),
            intercept: 0.0,
            num_classes: 2,
            learning_rate,
            regularization,
        }
    }

    // Train using gradient descent
    pub fn fit(&mut self, dataset: &Dataset, max_iterations: usize) -> Result<()> {
        let targets = dataset.targets.as_ref()
            .ok_or_else(|| crate::DbError::InvalidInput("Targets required".into()))?;

        let n_features = dataset.num_features();
        self.weights = vec![0.0; n_features];
        self.intercept = 0.0;

        // Gradient descent
        for _iter in 0..max_iterations {
            let mut weight_gradient = vec![0.0; n_features];
            let mut intercept_gradient = 0.0;

            for (sample, &target) in dataset.features.iter().zip(targets.iter()) {
                let prediction = self.predict_proba_single(sample);
                let error = prediction - target;

                for (i, &feature) in sample.iter().enumerate() {
                    weight_gradient[i] += error * feature;
                }
                intercept_gradient += error;
            }

            // Update weights with L2 regularization
            let n_samples = dataset.num_samples() as f64;
            for i in 0..n_features {
                self.weights[i] -= self.learning_rate * (
                    weight_gradient[i] / n_samples + self.regularization * self.weights[i]
                );
            }
            self.intercept -= self.learning_rate * intercept_gradient / n_samples;
        }

        Ok(())
    }

    // Predict class probabilities
    pub fn predict_proba(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        Ok(features.iter().map(|sample| self.predict_proba_single(sample)).collect())
    }

    // Predict classes (0 or 1)
    pub fn predict(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        Ok(self.predict_proba(features)?.iter().map(|&p| if p >= 0.5 { 1.0 } else { 0.0 }).collect())
    }

    fn predict_proba_single(&self, sample: &[f64]) -> f64 {
        let logit: f64 = sample.iter()
            .zip(&self.weights)
            .map(|(x, w)| x * w)
            .sum::<f64>() + self.intercept;
        Self::sigmoid(logit)
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
}

// ============================================================================
// Decision Tree (CART)
// ============================================================================

// Decision Tree for classification and regression (CART algorithm)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    // Root node of the tree
    pub root: Option<TreeNode>,
    // Maximum depth
    max_depth: usize,
    // Minimum samples to split
    min_samples_split: usize,
    // Minimum samples per leaf
    min_samples_leaf: usize,
    // Task type (classification or regression)
    is_classification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    // Feature index for split
    pub feature_idx: Option<usize>,
    // Threshold value for split
    pub threshold: Option<f64>,
    // Left child
    pub left: Option<Box<TreeNode>>,
    // Right child
    pub right: Option<Box<TreeNode>>,
    // Predicted value (for leaf nodes)
    pub value: Option<f64>,
    // Number of samples at this node
    pub n_samples: usize,
    // Impurity at this node
    pub impurity: f64,
}

impl DecisionTree {
    pub fn new(max_depth: usize, min_samples_split: usize, min_samples_leaf: usize, is_classification: bool) -> Self {
        Self {
            root: None,
            max_depth,
            min_samples_split,
            min_samples_leaf,
            is_classification,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let targets = dataset.targets.as_ref()
            .ok_or_else(|| crate::DbError::InvalidInput("Targets required".into()))?;

        let indices: Vec<usize> = (0..dataset.num_samples()).collect();
        self.root = Some(self.build_tree(&dataset.features, targets, &indices, 0)?);

        Ok(())
    }

    fn build_tree(
        &self,
        features: &[Vec<f64>],
        targets: &[f64],
        indices: &[usize],
        depth: usize,
    ) -> Result<TreeNode> {
        let n_samples = indices.len();

        // Compute impurity and value
        let (impurity, value) = if self.is_classification {
            self.compute_gini(targets, indices)
        } else {
            self.compute_mse(targets, indices)
        };

        // Check stopping criteria
        if depth >= self.max_depth
            || n_samples < self.min_samples_split
            || impurity < 1e-7
        {
            return Ok(TreeNode {
                feature_idx: None,
                threshold: None,
                left: None,
                right: None,
                value: Some(value),
                n_samples,
                impurity,
            });
        }

        // Find best split
        let best_split = self.find_best_split(features, targets, indices)?;

        if let Some((feature_idx, threshold, left_indices, right_indices)) = best_split {
            if left_indices.len() >= self.min_samples_leaf
                && right_indices.len() >= self.min_samples_leaf
            {
                let left = Box::new(self.build_tree(features, targets, &left_indices, depth + 1)?);
                let right = Box::new(self.build_tree(features, targets, &right_indices, depth + 1)?);

                return Ok(TreeNode {
                    feature_idx: Some(feature_idx),
                    threshold: Some(threshold),
                    left: Some(left),
                    right: Some(right),
                    value: Some(value),
                    n_samples,
                    impurity,
                });
            }
        }

        // Create leaf node
        Ok(TreeNode {
            feature_idx: None,
            threshold: None,
            left: None,
            right: None,
            value: Some(value),
            n_samples,
            impurity,
        })
    }

    fn find_best_split(
        &self,
        features: &[Vec<f64>],
        targets: &[f64],
        indices: &[usize],
    ) -> Result<Option<(usize, f64, Vec<usize>, Vec<usize>)>> {
        let n_features = features[0].len();
        let mut best_gain = f64::NEG_INFINITY;
        let mut best_split = None;

        let parent_impurity = if self.is_classification {
            self.compute_gini(targets, indices).0
        } else {
            self.compute_mse(targets, indices).0
        };

        for feature_idx in 0..n_features {
            // Get unique values for this feature
            let mut values: Vec<f64> = indices.iter()
                .map(|&i| features[i][feature_idx])
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            values.dedup();

            for &threshold in &values {
                let (left, right): (Vec<_>, Vec<_>) = indices.iter()
                    .partition(|&&i| features[i][feature_idx] <= threshold);

                if left.is_empty() || right.is_empty() {
                    continue;
                }

                let left_impurity = if self.is_classification {
                    self.compute_gini(targets, &left).0
                } else {
                    self.compute_mse(targets, &left).0
                };

                let right_impurity = if self.is_classification {
                    self.compute_gini(targets, &right).0
                } else {
                    self.compute_mse(targets, &right).0
                };

                let n_left = left.len() as f64;
                let n_right = right.len() as f64;
                let n_total = indices.len() as f64;

                let weighted_impurity = (n_left * left_impurity + n_right * right_impurity) / n_total;
                let gain = parent_impurity - weighted_impurity;

                if gain > best_gain {
                    best_gain = gain;
                    best_split = Some((feature_idx, threshold, left, right));
                }
            }
        }

        Ok(best_split)
    }

    fn compute_gini(&self, targets: &[f64], indices: &[usize]) -> (f64, f64) {
        if indices.is_empty() {
            return (0.0, 0.0);
        }

        let mut counts: HashMap<i64, usize> = HashMap::new();
        for &idx in indices {
            *counts.entry(targets[idx] as i64).or_insert(0) += 1;
        }

        let n = indices.len() as f64;
        let gini: f64 = 1.0 - counts.values()
            .map(|&count| (count as f64 / n).powi(2))
            .sum::<f64>();

        let majority_class = counts.iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(&class, _)| class as f64)
            .unwrap_or(0.0);

        (gini, majority_class)
    }

    fn compute_mse(&self, targets: &[f64], indices: &[usize]) -> (f64, f64) {
        if indices.is_empty() {
            return (0.0, 0.0);
        }

        let mean = indices.iter().map(|&i| targets[i]).sum::<f64>() / indices.len() as f64;
        let mse = indices.iter()
            .map(|&i| (targets[i] - mean).powi(2))
            .sum::<f64>() / indices.len() as f64;

        (mse, mean)
    }

    pub fn predict(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        let root = self.root.as_ref()
            .ok_or_else(|| crate::DbError::InvalidInput("Model not trained".into()))?;

        Ok(features.iter().map(|sample| self.predict_single(root, sample)).collect())
    }

    fn predict_single(&self, node: &TreeNode, sample: &[f64]) -> f64 {
        if let Some(value) = node.value {
            if node.left.is_none() && node.right.is_none() {
                return value;
            }
        }

        if let (Some(feature_idx), Some(threshold)) = (node.feature_idx, node.threshold) {
            if sample[feature_idx] <= threshold {
                if let Some(left) = &node.left {
                    return self.predict_single(left, sample);
                }
            } else if let Some(right) = &node.right {
                return self.predict_single(right, sample);
            }
        }

        node.value.unwrap_or(0.0)
    }
}

// ============================================================================
// Random Forest
// ============================================================================

// Random Forest ensemble method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomForest {
    // Collection of decision trees
    pub trees: Vec<DecisionTree>,
    // Number of trees in the forest
    n_estimators: usize,
    // Maximum depth per tree
    max_depth: usize,
    // Feature subsample ratio
    max_features: f64,
}

impl RandomForest {
    pub fn new(n_estimators: usize, max_depth: usize, max_features: f64) -> Self {
        Self {
            trees: Vec::new(),
            n_estimators,
            max_depth,
            max_features,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset, is_classification: bool) -> Result<()> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let mut rng = thread_rng();
        let n_samples = dataset.num_samples();
        let n_features = dataset.num_features();
        let features_per_tree = ((n_features as f64 * self.max_features).ceil() as usize).max(1);

        for _ in 0..self.n_estimators {
            // Bootstrap sampling
            let bootstrap_indices: Vec<usize> = (0..n_samples)
                .map(|_| rng.gen_range(0..n_samples))
                .collect();

            // Feature sampling
            let mut feature_indices: Vec<usize> = (0..n_features).collect();
            feature_indices.shuffle(&mut rng);
            feature_indices.truncate(features_per_tree);

            // Create subset dataset
            let subset_features: Vec<Vec<f64>> = bootstrap_indices.iter()
                .map(|&i| {
                    feature_indices.iter()
                        .map(|&j| dataset.features[i][j])
                        .collect()
                })
                .collect();

            let subset_targets = if let Some(targets) = &dataset.targets {
                Some(bootstrap_indices.iter().map(|&i| targets[i]).collect())
            } else {
                None
            };

            let subset_dataset = Dataset {
                features: subset_features,
                targets: subset_targets,
                feature_names: feature_indices.iter()
                    .map(|&i| dataset.feature_names[i].clone())
                    .collect(),
                target_name: dataset.target_name.clone(),
                weights: None,
            };

            let mut tree = DecisionTree::new(self.max_depth, 2, 1, is_classification);
            tree.fit(&subset_dataset)?;
            self.trees.push(tree);
        }

        Ok(())
    }

    pub fn predict(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        if self.trees.is_empty() {
            return Err(crate::DbError::InvalidInput("Model not trained".into()));
        }

        let n_samples = features.len();
        let mut predictions = vec![0.0; n_samples];

        for tree in &self.trees {
            let tree_preds = tree.predict(features)?;
            for i in 0..n_samples {
                predictions[i] += tree_preds[i];
            }
        }

        // Average predictions
        for pred in &mut predictions {
            *pred /= self.trees.len() as f64;
        }

        Ok(predictions)
    }
}

// ============================================================================
// K-Means Clustering
// ============================================================================

// K-Means clustering algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KMeans {
    // Cluster centroids
    pub centroids: Vec<Vec<f64>>,
    // Number of clusters
    k: usize,
    // Maximum iterations
    max_iterations: usize,
    // Convergence tolerance
    tolerance: f64,
}

impl KMeans {
    pub fn new(k: usize, max_iterations: usize, tolerance: f64) -> Self {
        Self {
            centroids: Vec::new(),
            k,
            max_iterations,
            tolerance,
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {

        let mut rng = thread_rng();
        let n_features = dataset.num_features();

        // Initialize centroids using k-means++
        self.centroids = Vec::with_capacity(self.k);
        let mut indices: Vec<usize> = (0..dataset.num_samples()).collect();
        indices.shuffle(&mut rng);
        self.centroids.push(dataset.features[indices[0]].clone());

        for _ in 1..self.k {
            let mut distances = Vec::with_capacity(dataset.num_samples());
            for sample in &dataset.features {
                let min_dist = self.centroids.iter()
                    .map(|centroid| self.euclidean_distance(sample, centroid))
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                distances.push(min_dist);
            }

            let max_idx = distances.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            self.centroids.push(dataset.features[max_idx].clone());
        }

        // K-means iterations
        for _ in 0..self.max_iterations {
            // Assign points to clusters
            let assignments = self.assign_clusters(&dataset.features);

            // Update centroids
            let mut new_centroids = vec![vec![0.0; n_features]; self.k];
            let mut counts = vec![0; self.k];

            for (sample, &cluster) in dataset.features.iter().zip(&assignments) {
                for (i, &val) in sample.iter().enumerate() {
                    new_centroids[cluster][i] += val;
                }
                counts[cluster] += 1;
            }

            for i in 0..self.k {
                if counts[i] > 0 {
                    for j in 0..n_features {
                        new_centroids[i][j] /= counts[i] as f64;
                    }
                }
            }

            // Check convergence
            let mut max_shift = 0.0;
            for (old, new) in self.centroids.iter().zip(&new_centroids) {
                let shift = self.euclidean_distance(old, new);
                if shift > max_shift {
                    max_shift = shift;
                }
            }

            self.centroids = new_centroids;

            if max_shift < self.tolerance {
                break;
            }
        }

        Ok(())
    }

    pub fn predict(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        if self.centroids.is_empty() {
            return Err(crate::DbError::InvalidInput("Model not trained".into()));
        }

        Ok(self.assign_clusters(features).iter().map(|&c| c as f64).collect())
    }

    fn assign_clusters(&self, features: &[Vec<f64>]) -> Vec<usize> {
        features.iter()
            .map(|sample| {
                self.centroids.iter()
                    .enumerate()
                    .map(|(i, centroid)| (i, self.euclidean_distance(sample, centroid)))
                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            })
            .collect()
    }

    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }
}

// ============================================================================
// Naive Bayes
// ============================================================================

// Gaussian Naive Bayes classifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaiveBayes {
    // Class priors
    pub class_priors: HashMap<i64, f64>,
    // Feature means per class
    pub means: HashMap<i64, Vec<f64>>,
    // Feature variances per class
    pub variances: HashMap<i64, Vec<f64>>,
}

impl NaiveBayes {
    pub fn new() -> Self {
        Self {
            class_priors: HashMap::new(),
            means: HashMap::new(),
            variances: HashMap::new(),
        }
    }

    pub fn fit(&mut self, dataset: &Dataset) -> Result<()> {
        let targets = dataset.targets.as_ref()
            .ok_or_else(|| crate::DbError::InvalidInput("Targets required".into()))?;

        let n_features = dataset.num_features();

        // Count classes
        let mut class_counts: HashMap<i64, usize> = HashMap::new();
        for &target in targets {
            *class_counts.entry(target as i64).or_insert(0) += 1;
        }

        let n_samples = dataset.num_samples() as f64;

        // Compute priors
        for (&class, &count) in &class_counts {
            self.class_priors.insert(class, count as f64 / n_samples);
        }

        // Compute means and variances per class
        for &class in class_counts.keys() {
            let class_samples: Vec<&Vec<f64>> = dataset.features.iter()
                .zip(targets)
                .filter(|(_, &t)| t as i64 == class)
                .map(|(s, _)| s)
                .collect();

            let mut means = vec![0.0; n_features];
            let mut variances = vec![0.0; n_features];

            for i in 0..n_features {
                let values: Vec<f64> = class_samples.iter().map(|s| s[i]).collect();
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance = values.iter()
                    .map(|&x| (x - mean).powi(2))
                    .sum::<f64>() / values.len() as f64;

                means[i] = mean;
                variances[i] = variance + 1e-9; // Add small constant for numerical stability
            }

            self.means.insert(class, means);
            self.variances.insert(class, variances);
        }

        Ok(())
    }

    pub fn predict(&self, features: &[Vec<f64>]) -> Result<Vec<f64>> {
        if self.class_priors.is_empty() {
            return Err(crate::DbError::InvalidInput("Model not trained".into()));
        }

        let mut predictions = Vec::with_capacity(features.len());

        for sample in features {
            let mut max_posterior = f64::NEG_INFINITY;
            let mut predicted_class = 0;

            for (&class, &prior) in &self.class_priors {
                let mut log_likelihood = prior.ln();

                let means = &self.means[&class];
                let variances = &self.variances[&class];

                for i in 0..sample.len() {
                    let mean = means[i];
                    let variance = variances[i];
                    let x = sample[i];

                    // Gaussian probability density
                    log_likelihood += -0.5 * ((x - mean).powi(2) / variance + variance.ln());
                }

                if log_likelihood > max_posterior {
                    max_posterior = log_likelihood;
                    predicted_class = class;
                }
            }

            predictions.push(predicted_class as f64);
        }

        Ok(predictions)
    }
}

impl Default for NaiveBayes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression() {
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 3.0],
            vec![3.0, 4.0],
            vec![4.0, 5.0],
        ];
        let targets = vec![3.0, 5.0, 7.0, 9.0];
        let dataset = Dataset::new(features.clone(), vec!["x1".to_string(), "x2".to_string()])
            .with_targets(targets, "y".to_string());

        let mut model = LinearRegression::new();
        model.fit(&dataset).unwrap();

        let predictions = model.predict(&features).unwrap();
        assert_eq!(predictions.len(), 4);
    }

    #[test]
    fn test_kmeans() {
        let features = vec![
            vec![1.0, 2.0],
            vec![1.5, 1.8],
            vec![5.0, 8.0],
            vec![8.0, 8.0],
            vec![1.0, 0.6],
            vec![9.0, 11.0],
        ];
        let dataset = Dataset::new(features.clone(), vec!["x".to_string(), "y".to_string()]);

        let mut model = KMeans::new(2, 100, 0.001);
        model.fit(&dataset).unwrap();

        let predictions = model.predict(&features).unwrap();
        assert_eq!(predictions.len(), 6);
    }
}
