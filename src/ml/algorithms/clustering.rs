// # Clustering Algorithms
//
// K-Means clustering implementation for unsupervised learning.

use crate::error::Result;
use super::super::{Dataset, Vector, Matrix, Hyperparameters, MLError};
use serde::{Serialize, Deserialize};
use super::{Algorithm, ModelType};

// ============================================================================
// K-Means Clustering
// ============================================================================

// K-Means clustering algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KMeansClustering {
    // Cluster centroids
    pub centroids: Matrix,
    // Number of clusters
    n_clusters: usize,
    // Whether the model has been trained
    trained: bool,
}

impl KMeansClustering {
    // Create a new K-means model
    pub fn new() -> Self {
        Self {
            centroids: Vec::new(),
            n_clusters: 0,
            trained: false,
        }
    }

    // Calculate Euclidean distance
    fn euclidean_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    // Assign samples to nearest centroid
    fn assign_clusters(&self, features: &Matrix) -> Vec<usize> {
        features.iter()
            .map(|sample| {
                self.centroids.iter()
                    .enumerate()
                    .map(|(i, centroid)| (i, self.euclidean_distance(sample, centroid)))
                    .min_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap())
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            })
            .collect()
    }

    // Update centroids based on cluster assignments
    fn update_centroids(&self, features: &Matrix, assignments: &[usize], n_features: usize) -> Matrix {
        let mut new_centroids = vec![vec![0.0; n_features]; self.n_clusters];
        let mut counts = vec![0; self.n_clusters];

        for (sample, &cluster) in features.iter().zip(assignments.iter()) {
            for (i, &value) in sample.iter().enumerate() {
                new_centroids[cluster][i] += value;
            }
            counts[cluster] += 1;
        }

        for (centroid, &count) in new_centroids.iter_mut().zip(counts.iter()) {
            if count > 0 {
                for value in centroid.iter_mut() {
                    *value /= count as f64;
                }
            }
        }

        new_centroids
    }
}

impl Default for KMeansClustering {
    fn default() -> Self {
        Self::new()
    }
}

impl Algorithm for KMeansClustering {
    fn fit(&mut self, dataset: &Dataset, params: &Hyperparameters) -> Result<()> {
        dataset.validate()?;

        self.n_clusters = params.get_int("n_clusters").unwrap_or(3) as usize;
        let max_iterations = params.get_int("max_iterations").unwrap_or(300) as usize;
        let tolerance = params.get_float("tolerance").unwrap_or(1e-4);

        let n_features = dataset.num_features();

        // Initialize centroids randomly (k-means++)
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let mut selected_indices: Vec<usize> = (0..dataset.num_samples()).collect();
        selected_indices.shuffle(&mut rng);

        self.centroids = selected_indices[..self.n_clusters]
            .iter()
            .map(|&i| dataset.features[i].clone())
            .collect();

        // Iterate until convergence
        for iteration in 0..max_iterations {
            let assignments = self.assign_clusters(&dataset.features);
            let new_centroids = self.update_centroids(&dataset.features, &assignments, n_features);

            // Check convergence
            let max_change = self.centroids.iter()
                .zip(new_centroids.iter())
                .map(|(old, new)| self.euclidean_distance(old, new))
                .fold(0.0f64, |a, b| a.max(b));

            self.centroids = new_centroids;

            if max_change < tolerance {
                tracing::debug!("K-means converged at iteration {}", iteration);
                break;
            }
        }

        self.trained = true;
        Ok(())
    }

    fn predict(&self, features: &Matrix) -> Result<Vector> {
        if !self.trained {
            return Err(MLError::PredictionFailed("Model not trained".to_string()).into());
        }

        Ok(self.assign_clusters(features)
            .iter()
            .map(|&cluster| cluster as f64)
            .collect())
    }

    fn model_type(&self) -> ModelType {
        ModelType::KMeans
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmeans() {
        let features = vec![
            vec![1.0, 1.0],
            vec![1.5, 2.0],
            vec![3.0, 4.0],
            vec![5.0, 7.0],
            vec![3.5, 5.0],
            vec![4.5, 5.0],
        ];
        let dataset = Dataset::new(features, None, vec!["x".to_string(), "y".to_string()]);

        let mut model = KMeansClustering::new();
        let mut params = ModelType::KMeans.default_hyperparameters();
        params.set_int("n_clusters", 2);
        assert!(model.fit(&dataset, &params).is_ok());
    }
}
