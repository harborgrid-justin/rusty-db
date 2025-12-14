// # Classification Algorithms
//
// Logistic regression and Naive Bayes implementations for classification tasks.

use super::super::{Dataset, Hyperparameters, MLError, Matrix, Vector};
use super::{Algorithm, ModelType};
use crate::error::Result;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Logistic Regression
// ============================================================================

// Logistic regression for binary classification
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LogisticRegression {
    // Model coefficients
    pub weights: Vector,
    // Intercept term
    pub intercept: f64,
    // Whether the model has been trained
    trained: bool,
    // Training metrics
    metrics: HashMap<String, f64>,
}

impl LogisticRegression {
    // Create a new logistic regression model
    pub fn new() -> Self {
        Self {
            weights: Vec::new(),
            intercept: 0.0,
            trained: false,
            metrics: HashMap::new(),
        }
    }

    // Sigmoid function
    fn sigmoid(&self, x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    // Compute predictions (probabilities)
    fn predict_proba_internal(&self, features: &Matrix) -> Vector {
        features
            .iter()
            .map(|sample| {
                let mut logit = self.intercept;
                for (i, &feature) in sample.iter().enumerate() {
                    logit += self.weights.get(i).unwrap_or(&0.0) * feature;
                }
                self.sigmoid(logit)
            })
            .collect()
    }

    // Compute binary cross-entropy loss
    fn binary_cross_entropy(&self, predictions: &Vector, targets: &Vector) -> f64 {
        let epsilon = 1e-15;
        predictions
            .iter()
            .zip(targets.iter())
            .map(|(pred, target)| {
                let p = pred.clamp(epsilon, 1.0 - epsilon);
                -target * p.ln() - (1.0 - target) * (1.0 - p).ln()
            })
            .sum::<f64>()
            / predictions.len() as f64
    }

    // Calculate accuracy
    fn accuracy(&self, predictions: &Vector, targets: &Vector) -> f64 {
        predictions
            .iter()
            .zip(targets.iter())
            .filter(|(pred, target)| {
                let class = if **pred >= 0.5 { 1.0 } else { 0.0 };
                (class - **target).abs() < 1e-9
            })
            .count() as f64
            / predictions.len() as f64
    }
}

impl Default for LogisticRegression {
    fn default() -> Self {
        Self::new()
    }
}

impl Algorithm for LogisticRegression {
    fn fit(&mut self, dataset: &Dataset, params: &Hyperparameters) -> Result<()> {
        dataset.validate()?;

        let target = dataset
            .target
            .as_ref()
            .ok_or_else(|| MLError::InvalidConfiguration("No target provided".to_string()))?;

        let learning_rate = params.get_float("learning_rate").unwrap_or(0.01);
        let max_iterations = params.get_int("max_iterations").unwrap_or(1000) as usize;
        let tolerance = params.get_float("tolerance").unwrap_or(1e-6);
        let regularization = params.get_float("regularization").unwrap_or(0.01);
        let fit_intercept = params.get_bool("fit_intercept").unwrap_or(true);

        let n_samples = dataset.num_samples();
        let n_features = dataset.num_features();

        // Initialize weights
        self.weights = vec![0.0; n_features];
        self.intercept = 0.0;

        let mut prev_loss = f64::INFINITY;

        // Gradient descent with L2 regularization
        for iteration in 0..max_iterations {
            let predictions = self.predict_proba_internal(&dataset.features);

            // Compute gradients
            let mut weight_gradients = vec![0.0; n_features];
            let mut intercept_gradient = 0.0;

            for i in 0..n_samples {
                let error = predictions[i] - target[i];
                for j in 0..n_features {
                    weight_gradients[j] += error * dataset.features[i][j];
                }
                if fit_intercept {
                    intercept_gradient += error;
                }
            }

            // Update weights with L2 regularization
            for j in 0..n_features {
                let gradient = weight_gradients[j] / n_samples as f64;
                let reg_gradient = regularization * self.weights[j];
                self.weights[j] -= learning_rate * (gradient + reg_gradient);
            }
            if fit_intercept {
                self.intercept -= learning_rate * intercept_gradient / n_samples as f64;
            }

            // Check convergence
            let loss = self.binary_cross_entropy(&predictions, target);
            if (prev_loss - loss).abs() < tolerance {
                break;
            }
            prev_loss = loss;

            if iteration % 100 == 0 {
                tracing::debug!("Iteration {}: loss = {}", iteration, loss);
            }
        }

        // Calculate final metrics
        let final_predictions = self.predict_proba_internal(&dataset.features);
        let loss = self.binary_cross_entropy(&final_predictions, target);
        let accuracy = self.accuracy(&final_predictions, target);

        self.metrics.insert("loss".to_string(), loss);
        self.metrics.insert("accuracy".to_string(), accuracy);

        self.trained = true;
        Ok(())
    }

    fn predict(&self, features: &Matrix) -> Result<Vector> {
        if !self.trained {
            return Err(MLError::PredictionFailed("Model not trained".to_string()).into());
        }
        // Return class labels (0 or 1)
        Ok(self
            .predict_proba_internal(features)
            .iter()
            .map(|&p| if p >= 0.5 { 1.0 } else { 0.0 })
            .collect())
    }

    fn model_type(&self) -> ModelType {
        ModelType::LogisticRegression
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        let config = bincode::config::standard();
        bincode::encode_to_vec(self, config).map_err(|e| {
            MLError::InvalidConfiguration(format!("Serialization failed: {}", e)).into()
        })
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let config = bincode::config::standard();
        bincode::decode_from_slice(bytes, config)
            .map(|(model, _)| model)
            .map_err(|e| {
                MLError::InvalidConfiguration(format!("Deserialization failed: {}", e)).into()
            })
    }

    fn feature_importance(&self) -> Option<Vector> {
        Some(self.weights.iter().map(|w| w.abs()).collect())
    }
}

// ============================================================================
// Naive Bayes
// ============================================================================

// Naive Bayes classifier (Gaussian)
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct NaiveBayes {
    // Class priors
    class_priors: HashMap<i64, f64>,
    // Feature means per class
    feature_means: HashMap<i64, Vector>,
    // Feature variances per class
    feature_variances: HashMap<i64, Vector>,
    // Classes
    classes: Vec<i64>,
    // Whether the model has been trained
    trained: bool,
}

impl NaiveBayes {
    // Create a new Naive Bayes model
    pub fn new() -> Self {
        Self {
            class_priors: HashMap::new(),
            feature_means: HashMap::new(),
            feature_variances: HashMap::new(),
            classes: Vec::new(),
            trained: false,
        }
    }

    // Calculate Gaussian probability density
    fn gaussian_pdf(&self, x: f64, mean: f64, variance: f64) -> f64 {
        let epsilon = 1e-9;
        let var = variance.max(epsilon);
        let exponent = -(x - mean).powi(2) / (2.0 * var);
        (1.0 / (2.0 * std::f64::consts::PI * var).sqrt()) * exponent.exp()
    }

    // Helper method to calculate mean and variance for features
    fn calculate_statistics(samples: &[&Vec<f64>], n_features: usize) -> (Vector, Vector) {
        let mut means = vec![0.0; n_features];
        let mut variances = vec![0.0; n_features];

        // Calculate means
        for sample in samples {
            for (i, &value) in sample.iter().enumerate() {
                means[i] += value;
            }
        }
        for mean in &mut means {
            *mean /= samples.len() as f64;
        }

        // Calculate variances
        for sample in samples {
            for (i, &value) in sample.iter().enumerate() {
                variances[i] += (value - means[i]).powi(2);
            }
        }
        for variance in &mut variances {
            *variance /= samples.len() as f64;
        }

        (means, variances)
    }
}

impl Default for NaiveBayes {
    fn default() -> Self {
        Self::new()
    }
}

impl Algorithm for NaiveBayes {
    fn fit(&mut self, dataset: &Dataset, params: &Hyperparameters) -> Result<()> {
        dataset.validate()?;

        let target = dataset
            .target
            .as_ref()
            .ok_or_else(|| MLError::InvalidConfiguration("No target provided".to_string()))?;

        let alpha = params.get_float("alpha").unwrap_or(1.0);
        let n_features = dataset.num_features();
        let n_samples = dataset.num_samples();

        // Find unique classes
        let mut class_counts: HashMap<i64, usize> = HashMap::new();
        for &label in target.iter() {
            let class = label as i64;
            *class_counts.entry(class).or_insert(0) += 1;
        }

        self.classes = class_counts.keys().copied().collect();
        self.classes.sort();

        // Calculate class priors
        for (&class, &count) in &class_counts {
            self.class_priors.insert(
                class,
                (count as f64 + alpha) / (n_samples as f64 + alpha * self.classes.len() as f64),
            );
        }

        // Calculate feature statistics per class
        for &class in &self.classes {
            let class_samples: Vec<&Vec<f64>> = dataset
                .features
                .iter()
                .zip(target.iter())
                .filter(|(_, &label)| label as i64 == class)
                .map(|(features, _)| features)
                .collect();

            let (means, variances) = Self::calculate_statistics(&class_samples, n_features);

            self.feature_means.insert(class, means);
            self.feature_variances.insert(class, variances);
        }

        self.trained = true;
        Ok(())
    }

    fn predict(&self, features: &Matrix) -> Result<Vector> {
        if !self.trained {
            return Err(MLError::PredictionFailed("Model not trained".to_string()).into());
        }

        Ok(features
            .iter()
            .map(|sample| {
                let mut best_class = self.classes[0];
                let mut best_prob = f64::NEG_INFINITY;

                for &class in &self.classes {
                    let mut log_prob = self.class_priors[&class].ln();

                    let means = &self.feature_means[&class];
                    let variances = &self.feature_variances[&class];

                    for (i, &value) in sample.iter().enumerate() {
                        log_prob += self.gaussian_pdf(value, means[i], variances[i]).ln();
                    }

                    if log_prob > best_prob {
                        best_prob = log_prob;
                        best_class = class;
                    }
                }

                best_class as f64
            })
            .collect())
    }

    fn model_type(&self) -> ModelType {
        ModelType::NaiveBayes
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        let config = bincode::config::standard();
        bincode::encode_to_vec(self, config).map_err(|e| {
            MLError::InvalidConfiguration(format!("Serialization failed: {}", e)).into()
        })
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let config = bincode::config::standard();
        bincode::decode_from_slice(bytes, config)
            .map(|(model, _)| model)
            .map_err(|e| {
                MLError::InvalidConfiguration(format!("Deserialization failed: {}", e)).into()
            })
    }
}
