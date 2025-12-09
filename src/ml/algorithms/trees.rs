// # Tree-Based Algorithms
//
// Decision tree and random forest implementations for classification and regression.

use crate::error::Result;
use super::super::{Dataset, Vector, Matrix, Hyperparameters, MLError};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use super::{Algorithm, ModelType};

// ============================================================================
// Decision Tree (CART Algorithm)
// ============================================================================

/// Node in a decision tree
#[derive(Debug, Clone, Serialize, Deserialize)]
enum TreeNode {
    Leaf {
        value: f64,
        samples: usize,
    },
    Split {
        feature: usize,
        threshold: f64,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
        samples: usize,
    },
}

/// Decision tree classifier/regressor using CART algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTree {
    root: Option<TreeNode>,
    n_features: usize,
    trained: bool,
    is_classifier: bool,
}

impl DecisionTree {
    /// Create a new decision tree
    pub fn new(is_classifier: bool) -> Self {
        Self {
            root: None,
            n_features: 0,
            trained: false,
            is_classifier,
        }
    }

    /// Build tree recursively
    fn build_tree(
        &self,
        features: &Matrix,
        targets: &Vector,
        indices: Vec<usize>,
        depth: usize,
        max_depth: usize,
        min_samples_split: usize,
        min_samples_leaf: usize,
    ) -> TreeNode {
        if indices.len() < min_samples_split || depth >= max_depth {
            return self.create_leaf(&targets, &indices);
        }

        // Find best split
        if let Some((feature, threshold, left_indices, right_indices)) =
            self.find_best_split(features, targets, &indices, min_samples_leaf)
        {
            let left = self.build_tree(
                features,
                targets,
                left_indices,
                depth + 1,
                max_depth,
                min_samples_split,
                min_samples_leaf,
            );
            let right = self.build_tree(
                features,
                targets,
                right_indices,
                depth + 1,
                max_depth,
                min_samples_split,
                min_samples_leaf,
            );

            TreeNode::Split {
                feature,
                threshold,
                left: Box::new(left),
                right: Box::new(right),
                samples: indices.len(),
            }
        } else {
            self.create_leaf(&targets, &indices)
        }
    }

    /// Create a leaf node
    fn create_leaf(&self, targets: &Vector, indices: &[usize]) -> TreeNode {
        let value = if self.is_classifier {
            // Mode (most common class)
            self.mode(targets, indices)
        } else {
            // Mean for regression
            indices.iter().map(|&i| targets[i]).sum::<f64>() / indices.len() as f64
        };

        TreeNode::Leaf {
            value,
            samples: indices.len(),
        }
    }

    /// Find mode (most common value)
    fn mode(&self, targets: &Vector, indices: &[usize]) -> f64 {
        let mut counts: HashMap<i64, usize> = HashMap::new();
        for &i in indices {
            let class = targets[i] as i64;
            *counts.entry(class).or_insert(0) += 1;
        }
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(class, _)| class as f64)
            .unwrap_or(0.0)
    }

    /// Find best split point
    fn find_best_split(
        &self,
        features: &Matrix,
        targets: &Vector,
        indices: &[usize],
        min_samples_leaf: usize,
    ) -> Option<(usize, f64, Vec<usize>, Vec<usize>)> {
        let mut best_gain = 0.0;
        let mut best_split = None;

        let parent_impurity = self.calculate_impurity(targets, indices);

        for feature_idx in 0..self.n_features {
            // Get unique values for this feature
            let mut values: Vec<f64> = indices.iter()
                .map(|&i| features[i][feature_idx])
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            values.dedup();

            // Try each potential split point
            for &threshold in &values {
                let (left, right): (Vec<_>, Vec<_>) = indices.iter()
                    .partition(|&&i| features[i][feature_idx] <= threshold);

                if left.len() < min_samples_leaf || right.len() < min_samples_leaf {
                    continue;
                }

                let left_impurity = self.calculate_impurity(targets, &left);
                let right_impurity = self.calculate_impurity(targets, &right);

                let n_left = left.len() as f64;
                let n_right = right.len() as f64;
                let n_total = indices.len() as f64;

                let gain = parent_impurity
                    - (n_left / n_total) * left_impurity
                    - (n_right / n_total) * right_impurity;

                if gain > best_gain {
                    best_gain = gain;
                    best_split = Some((feature_idx, threshold, left, right));
                }
            }
        }

        best_split
    }

    /// Calculate impurity (Gini for classification, variance for regression)
    fn calculate_impurity(&self, targets: &Vector, indices: &[usize]) -> f64 {
        if indices.is_empty() {
            return 0.0;
        }

        if self.is_classifier {
            // Gini impurity
            let mut counts: HashMap<i64, usize> = HashMap::new();
            for &i in indices {
                let class = targets[i] as i64;
                *counts.entry(class).or_insert(0) += 1;
            }

            let n = indices.len() as f64;
            1.0 - counts.values()
                .map(|&count| {
                    let p = count as f64 / n;
                    p * p
                })
                .sum::<f64>()
        } else {
            // Variance
            let mean = indices.iter().map(|&i| targets[i]).sum::<f64>() / indices.len() as f64;
            indices.iter()
                .map(|&i| (targets[i] - mean).powi(2))
                .sum::<f64>() / indices.len() as f64
        }
    }

    /// Predict a single sample
    fn predict_sample(&self, sample: &[f64], node: &TreeNode) -> f64 {
        match node {
            TreeNode::Leaf { value, .. } => *value,
            TreeNode::Split { feature, threshold, left, right, .. } => {
                if sample[*feature] <= *threshold {
                    self.predict_sample(sample, left)
                } else {
                    self.predict_sample(sample, right)
                }
            }
        }
    }
}

impl Default for DecisionTree {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Algorithm for DecisionTree {
    fn fit(&mut self, dataset: &Dataset, params: &Hyperparameters) -> Result<()> {
        dataset.validate()?;

        let target = dataset.target.as_ref()
            .ok_or_else(|| MLError::InvalidConfiguration("No target provided".to_string()))?;

        let max_depth = params.get_int("max_depth").unwrap_or(10) as usize;
        let min_samples_split = params.get_int("min_samples_split").unwrap_or(2) as usize;
        let min_samples_leaf = params.get_int("min_samples_leaf").unwrap_or(1) as usize;

        self.n_features = dataset.num_features();
        let indices: Vec<usize> = (0..dataset.num_samples()).collect();

        self.root = Some(self.build_tree(
            &dataset.features,
            target,
            indices,
            0,
            max_depth,
            min_samples_split,
            min_samples_leaf,
        ));

        self.trained = true;
        Ok(())
    }

    fn predict(&self, features: &Matrix) -> Result<Vector> {
        if !self.trained {
            return Err(MLError::PredictionFailed("Model not trained".to_string()).into());
        }

        let root = self.root.as_ref()
            .ok_or_else(|| MLError::PredictionFailed("No tree built".to_string()))?;

        Ok(features.iter()
            .map(|sample| self.predict_sample(sample, root))
            .collect())
    }

    fn model_type(&self) -> ModelType {
        ModelType::DecisionTree
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| MLError::InvalidConfiguration(format!("Serialization failed: {}", e)).into())
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| MLError::InvalidConfiguration(format!("Deserialization failed: {}", e)).into())
    }
}

// ============================================================================
// Random Forest
// ============================================================================

/// Random forest ensemble of decision trees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomForest {
    trees: Vec<DecisionTree>,
    n_estimators: usize,
    trained: bool,
    is_classifier: bool,
}

impl RandomForest {
    /// Create a new random forest
    pub fn new(is_classifier: bool) -> Self {
        Self {
            trees: Vec::new(),
            n_estimators: 0,
            trained: false,
            is_classifier,
        }
    }

    /// Bootstrap sampling
    fn bootstrap_sample(&self, n_samples: usize) -> Vec<usize> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..n_samples)
            .map(|_| rng.gen_range(0..n_samples))
            .collect()
    }
}

impl Default for RandomForest {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Algorithm for RandomForest {
    fn fit(&mut self, dataset: &Dataset, params: &Hyperparameters) -> Result<()> {
        dataset.validate()?;

        let target = dataset.target.as_ref()
            .ok_or_else(|| MLError::InvalidConfiguration("No target provided".to_string()))?;

        self.n_estimators = params.get_int("n_estimators").unwrap_or(100) as usize;
        let max_depth = params.get_int("max_depth").unwrap_or(10);
        let min_samples_split = params.get_int("min_samples_split").unwrap_or(2);
        let min_samples_leaf = params.get_int("min_samples_leaf").unwrap_or(1);

        self.trees.clear();

        // Train each tree on a bootstrap sample
        for i in 0..self.n_estimators {
            let bootstrap_indices = self.bootstrap_sample(dataset.num_samples());

            // Create bootstrap dataset
            let bootstrap_features: Matrix = bootstrap_indices.iter()
                .map(|&idx| dataset.features[idx].clone())
                .collect();
            let bootstrap_target: Vector = bootstrap_indices.iter()
                .map(|&idx| target[idx])
                .collect();

            let bootstrap_dataset = Dataset::new(
                bootstrap_features,
                Some(bootstrap_target),
                dataset.feature_names.clone(),
            );

            let mut tree = DecisionTree::new(self.is_classifier);
            let tree_params = {
                let mut p = Hyperparameters::new();
                p.set_int("max_depth", max_depth);
                p.set_int("min_samples_split", min_samples_split);
                p.set_int("min_samples_leaf", min_samples_leaf);
                p
            };

            tree.fit(&bootstrap_dataset, &tree_params)?;
            self.trees.push(tree);

            if i % 10 == 0 {
                tracing::debug!("Trained tree {}/{}", i + 1, self.n_estimators);
            }
        }

        self.trained = true;
        Ok(())
    }

    fn predict(&self, features: &Matrix) -> Result<Vector> {
        if !self.trained {
            return Err(MLError::PredictionFailed("Model not trained".to_string()).into());
        }

        let n_samples = features.len();
        let mut predictions = vec![0.0; n_samples];

        // Aggregate predictions from all trees
        for tree in &self.trees {
            let tree_predictions = tree.predict(features)?;
            for (i, &pred) in tree_predictions.iter().enumerate() {
                predictions[i] += pred;
            }
        }

        // Average for regression, majority vote for classification
        if self.is_classifier {
            // Round to nearest class
            Ok(predictions.iter()
                .map(|&sum| (sum / self.n_estimators as f64).round())
                .collect())
        } else {
            Ok(predictions.iter()
                .map(|&sum| sum / self.n_estimators as f64)
                .collect())
        }
    }

    fn model_type(&self) -> ModelType {
        ModelType::RandomForest
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| MLError::InvalidConfiguration(format!("Serialization failed: {}", e)).into())
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| MLError::InvalidConfiguration(format!("Deserialization failed: {}", e)).into())
    }
}
