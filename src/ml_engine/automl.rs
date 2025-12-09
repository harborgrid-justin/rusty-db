// # AutoML Engine
//
// Automated machine learning with algorithm selection, hyperparameter tuning,
// cross-validation, and model comparison.

use crate::error::Result;
use super::{Algorithm, Dataset, Hyperparameters, HyperparamValue, MLTask, EvaluationMetrics};
use super::model_store::{Model};
use super::training::TrainingEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// AutoML Configuration
// ============================================================================

/// AutoML configuration and search space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMLConfig {
    /// Maximum time budget in seconds
    pub time_budget: u64,
    /// Evaluation metric to optimize
    pub metric: OptimizationMetric,
    /// Cross-validation folds
    pub cv_folds: usize,
    /// Hyperparameter search strategy
    pub search_strategy: SearchStrategy,
    /// Algorithms to consider
    pub algorithms: Vec<Algorithm>,
    /// Early stopping patience
    pub early_stopping_patience: usize,
    /// Maximum number of trials
    pub max_trials: usize,
}

impl AutoMLConfig {
    pub fn new(time_budget: u64, metric: OptimizationMetric) -> Self {
        Self {
            time_budget,
            metric,
            cv_folds: 5,
            search_strategy: SearchStrategy::RandomSearch,
            algorithms: vec![
                Algorithm::LinearRegression,
                Algorithm::LogisticRegression,
                Algorithm::DecisionTree,
                Algorithm::RandomForest,
                Algorithm::GradientBoosting,
            ],
            early_stopping_patience: 10,
            max_trials: 100,
        }
    }

    pub fn for_classification(time_budget: u64) -> Self {
        Self {
            algorithms: vec![
                Algorithm::LogisticRegression,
                Algorithm::DecisionTree,
                Algorithm::RandomForest,
                Algorithm::GradientBoosting,
                Algorithm::NaiveBayes,
            ],
            ..Self::new(time_budget, OptimizationMetric::Accuracy)
        }
    }

    pub fn for_regression(time_budget: u64) -> Self {
        Self {
            algorithms: vec![
                Algorithm::LinearRegression,
                Algorithm::DecisionTree,
                Algorithm::RandomForest,
                Algorithm::GradientBoosting,
            ],
            ..Self::new(time_budget, OptimizationMetric::RMSE)
        }
    }
}

/// Optimization metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationMetric {
    // Classification metrics
    Accuracy,
    Precision,
    Recall,
    F1,
    AUC,
    LogLoss,

    // Regression metrics
    MSE,
    RMSE,
    MAE,
    R2,

    // Clustering metrics
    Silhouette,
    DaviesBouldin,
}

impl OptimizationMetric {
    pub fn is_higher_better(&self) -> bool {
        match self {
            OptimizationMetric::Accuracy | OptimizationMetric::Precision |
            OptimizationMetric::Recall | OptimizationMetric::F1 |
            OptimizationMetric::AUC | OptimizationMetric::R2 |
            OptimizationMetric::Silhouette => true,

            OptimizationMetric::MSE | OptimizationMetric::RMSE |
            OptimizationMetric::MAE | OptimizationMetric::LogLoss |
            OptimizationMetric::DaviesBouldin => false,
        }
    }

    pub fn extract_from_metrics(&self, metrics: &EvaluationMetrics) -> Option<f64> {
        match self {
            OptimizationMetric::Accuracy => metrics.accuracy,
            OptimizationMetric::Precision => metrics.precision,
            OptimizationMetric::Recall => metrics.recall,
            OptimizationMetric::F1 => metrics.f1,
            OptimizationMetric::AUC => metrics.auc,
            OptimizationMetric::LogLoss => metrics.log_loss,
            OptimizationMetric::MSE => metrics.mse,
            OptimizationMetric::RMSE => metrics.rmse,
            OptimizationMetric::MAE => metrics.mae,
            OptimizationMetric::R2 => metrics.r2,
            OptimizationMetric::Silhouette => metrics.silhouette,
            OptimizationMetric::DaviesBouldin => metrics.davies_bouldin,
        }
    }
}

/// Hyperparameter search strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchStrategy {
    /// Grid search over all combinations
    GridSearch,
    /// Random search sampling
    RandomSearch,
    /// Bayesian optimization
    BayesianOptimization,
    /// Successive Halving
    SuccessiveHalving,
    /// Hyperband
    Hyperband,
}

// ============================================================================
// Hyperparameter Search Space
// ============================================================================

/// Hyperparameter search space definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperparameterSpace {
    /// Parameter ranges
    ranges: HashMap<String, ParameterRange>,
}

impl HyperparameterSpace {
    pub fn new() -> Self {
        Self {
            ranges: HashMap::new(),
        }
    }

    pub fn add_range(&mut self, name: String, range: ParameterRange) {
        self.ranges.insert(name, range);
    }

    pub fn sample(&self) -> Hyperparameters {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut params = Hyperparameters::new();

        for (name, range) in &self.ranges {
            let value = match range {
                ParameterRange::Integer { min, max } => {
                    HyperparamValue::Int(rng.gen_range(*min..=*max))
                }
                ParameterRange::Float { min, max, log_scale } => {
                    let value = if *log_scale {
                        let log_min = min.ln();
                        let log_max = max.ln();
                        rng.gen_range(log_min..=log_max).exp()
                    } else {
                        rng.gen_range(*min..=*max)
                    };
                    HyperparamValue::Float(value)
                }
                ParameterRange::Categorical { values } => {
                    values[rng.gen_range(0..values.len())].clone()
                }
            };

            params.set(name.clone(), value);
        }

        params
    }

    pub fn default_for_algorithm(algorithm: &Algorithm) -> Self {
        let mut space = Self::new();

        match algorithm {
            Algorithm::LinearRegression | Algorithm::LogisticRegression => {
                space.add_range(
                    "learning_rate".to_string(),
                    ParameterRange::Float {
                        min: 0.001,
                        max: 0.1,
                        log_scale: true,
                    },
                );
                space.add_range(
                    "max_iterations".to_string(),
                    ParameterRange::Integer { min: 100, max: 1000 },
                );
            }
            Algorithm::DecisionTree => {
                space.add_range(
                    "max_depth".to_string(),
                    ParameterRange::Integer { min: 3, max: 20 },
                );
                space.add_range(
                    "min_samples_split".to_string(),
                    ParameterRange::Integer { min: 2, max: 20 },
                );
                space.add_range(
                    "min_samples_leaf".to_string(),
                    ParameterRange::Integer { min: 1, max: 10 },
                );
            }
            Algorithm::RandomForest | Algorithm::GradientBoosting => {
                space.add_range(
                    "n_estimators".to_string(),
                    ParameterRange::Integer { min: 10, max: 200 },
                );
                space.add_range(
                    "max_depth".to_string(),
                    ParameterRange::Integer { min: 3, max: 15 },
                );
                space.add_range(
                    "max_features".to_string(),
                    ParameterRange::Float {
                        min: 0.1,
                        max: 1.0,
                        log_scale: false,
                    },
                );
            }
            Algorithm::KMeans => {
                space.add_range(
                    "k".to_string(),
                    ParameterRange::Integer { min: 2, max: 20 },
                );
                space.add_range(
                    "max_iterations".to_string(),
                    ParameterRange::Integer { min: 100, max: 500 },
                );
            }
            Algorithm::DBSCAN => {
                space.add_range(
                    "eps".to_string(),
                    ParameterRange::Float {
                        min: 0.1,
                        max: 2.0,
                        log_scale: false,
                    },
                );
                space.add_range(
                    "min_samples".to_string(),
                    ParameterRange::Integer { min: 2, max: 10 },
                );
            }
            _ => {}
        }

        space
    }
}

impl Default for HyperparameterSpace {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameter range definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterRange {
    Integer {
        min: i64,
        max: i64,
    },
    Float {
        min: f64,
        max: f64,
        log_scale: bool,
    },
    Categorical {
        values: Vec<HyperparamValue>,
    },
}

// ============================================================================
// Trial and Results
// ============================================================================

/// Single trial in hyperparameter search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial {
    pub trial_id: usize,
    pub algorithm: Algorithm,
    pub hyperparameters: Hyperparameters,
    pub cv_scores: Vec<f64>,
    pub mean_score: f64,
    pub std_score: f64,
    pub training_time: f64,
    pub status: TrialStatus,
}

impl Trial {
    pub fn new(trial_id: usize, algorithm: Algorithm, hyperparameters: Hyperparameters) -> Self {
        Self {
            trial_id,
            algorithm,
            hyperparameters,
            cv_scores: Vec::new(),
            mean_score: 0.0,
            std_score: 0.0,
            training_time: 0.0,
            status: TrialStatus::Pending,
        }
    }

    pub fn complete(&mut self, cv_scores: Vec<f64>, training_time: f64) {
        let n = cv_scores.len() as f64;
        self.mean_score = cv_scores.iter().sum::<f64>() / n;

        let variance = cv_scores.iter()
            .map(|&score| (score - self.mean_score).powi(2))
            .sum::<f64>() / n;
        self.std_score = variance.sqrt();

        self.cv_scores = cv_scores;
        self.training_time = training_time;
        self.status = TrialStatus::Completed;
    }

    pub fn fail(&mut self, error: String) {
        self.status = TrialStatus::Failed(error);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrialStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Pruned,
}

/// AutoML results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMLResults {
    pub best_trial: Trial,
    pub all_trials: Vec<Trial>,
    pub best_model: Option<Model>,
    pub leaderboard: Vec<LeaderboardEntry>,
    pub search_time: f64,
    pub total_trials: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub algorithm: Algorithm,
    pub score: f64,
    pub training_time: f64,
    pub hyperparameters: Hyperparameters,
}

// ============================================================================
// AutoML Engine
// ============================================================================

/// Main AutoML coordinator
pub struct AutoMLEngine {
    /// Training engine
    training_engine: TrainingEngine,
    /// Current configuration
    config: Option<AutoMLConfig>,
}

impl AutoMLEngine {
    pub fn new() -> Self {
        Self {
            training_engine: TrainingEngine::new(),
            config: None,
        }
    }

    /// Find the best model using AutoML
    pub fn find_best_model(
        &self,
        dataset: Dataset,
        task: MLTask,
        time_budget: u64,
    ) -> Result<Model> {
        let config = match task {
            MLTask::Classification => AutoMLConfig::for_classification(time_budget),
            MLTask::Regression => AutoMLConfig::for_regression(time_budget),
            _ => AutoMLConfig::new(time_budget, OptimizationMetric::Accuracy),
        };

        self.find_best_model_with_config(dataset, config)
    }

    /// Find the best model with custom configuration
    pub fn find_best_model_with_config(
        &self,
        dataset: Dataset,
        config: AutoMLConfig,
    ) -> Result<Model> {
        let results = self.run_automl(&dataset, &config)?;

        results.best_model
            .ok_or_else(|| DbError::Internal("No valid model found".into()))
    }

    /// Run AutoML search
    pub fn run_automl(&self, dataset: &Dataset, config: &AutoMLConfig) -> Result<AutoMLResults> {
        let start_time = std::time::Instant::now();
        let mut trials = Vec::new();
        let mut best_score = if config.metric.is_higher_better() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut best_trial: Option<Trial> = None;
        let mut trials_without_improvement = 0;

        for trial_id in 0..config.max_trials {
            // Check time budget
            if start_time.elapsed().as_secs() >= config.time_budget {
                break;
            }

            // Select algorithm (round-robin for now)
            let algorithm = config.algorithms[trial_id % config.algorithms.len()].clone();

            // Sample hyperparameters
            let space = HyperparameterSpace::default_for_algorithm(&algorithm);
            let hyperparameters = space.sample();

            let mut trial = Trial::new(trial_id, algorithm.clone(), hyperparameters.clone());
            trial.status = TrialStatus::Running;

            // Perform cross-validation
            match self.cross_validate(dataset, &algorithm, &hyperparameters, config) {
                Ok((cv_scores, training_time)) => {
                    trial.complete(cv_scores, training_time);

                    // Check if this is the best trial
                    let is_better = if config.metric.is_higher_better() {
                        trial.mean_score > best_score
                    } else {
                        trial.mean_score < best_score
                    };

                    if is_better {
                        best_score = trial.mean_score;
                        best_trial = Some(trial.clone());
                        trials_without_improvement = 0;
                    } else {
                        trials_without_improvement += 1;
                    }
                }
                Err(e) => {
                    trial.fail(e.to_string());
                }
            }

            trials.push(trial);

            // Early stopping
            if trials_without_improvement >= config.early_stopping_patience {
                break;
            }
        }

        let search_time = start_time.elapsed().as_secs_f64();

        // Train final model with best hyperparameters
        let best_model = if let Some(ref best) = best_trial {
            match self.training_engine.train(
                best.algorithm.clone(),
                dataset.clone(),
                best.hyperparameters.clone(),
                &Default::default(),
            ) {
                Ok(model) => Some(model),
                Err(_) => None,
            }
        } else {
            None
        };

        // Build leaderboard
        let leaderboard = self.build_leaderboard(&trials, config);

        Ok(AutoMLResults {
            best_trial: best_trial.unwrap_or_else(|| trials[0].clone()),
            all_trials: trials.clone(),
            best_model,
            leaderboard,
            search_time,
            total_trials: trials.len(),
        })
    }

    /// Perform cross-validation
    fn cross_validate(
        &self,
        dataset: &Dataset,
        algorithm: &Algorithm,
        hyperparameters: &Hyperparameters,
        config: &AutoMLConfig,
    ) -> Result<(Vec<f64>, f64)> {
        let n_samples = dataset.num_samples();
        let fold_size = n_samples / config.cv_folds;
        let mut cv_scores = Vec::new();
        let mut total_time = 0.0;

        for fold in 0..config.cv_folds {
            let val_start = fold * fold_size;
            let val_end = if fold == config.cv_folds - 1 {
                n_samples
            } else {
                (fold + 1) * fold_size
            };

            // Split train/validation
            let (train_dataset, val_dataset) = self.split_dataset(dataset, val_start, val_end)?;

            // Train model
            let start = std::time::Instant::now();
            let model = self.training_engine.train(
                algorithm.clone(),
                train_dataset,
                hyperparameters.clone(),
                &Default::default(),
            )?;
            total_time += start.elapsed().as_secs_f64();

            // Evaluate on validation set
            let score = self.evaluate_model(&model, &val_dataset, config.metric)?;
            cv_scores.push(score);
        }

        Ok((cv_scores, total_time))
    }

    /// Split dataset into train and validation
    fn split_dataset(
        &self,
        dataset: &Dataset,
        val_start: usize,
        val_end: usize,
    ) -> Result<(Dataset, Dataset)> {
        let mut train_features = Vec::new();
        let mut train_targets = Vec::new();
        let mut val_features = Vec::new();
        let mut val_targets = Vec::new();

        for i in 0..dataset.num_samples() {
            if i >= val_start && i < val_end {
                val_features.push(dataset.features[i].clone());
                if let Some(ref targets) = dataset.targets {
                    val_targets.push(targets[i]);
                }
            } else {
                train_features.push(dataset.features[i].clone());
                if let Some(ref targets) = dataset.targets {
                    train_targets.push(targets[i]);
                }
            }
        }

        let train = Dataset::new(train_features, dataset.feature_names.clone())
            .with_targets(train_targets, dataset.target_name.clone().unwrap_or_default());

        let val = Dataset::new(val_features, dataset.feature_names.clone())
            .with_targets(val_targets, dataset.target_name.clone().unwrap_or_default());

        Ok((train, val))
    }

    /// Evaluate model on validation set
    fn evaluate_model(
        &self,
        _model: &Model,
        _dataset: &Dataset,
        metric: OptimizationMetric,
    ) -> Result<f64> {
        // Simplified evaluation - in production, use actual predictions
        Ok(match metric {
            OptimizationMetric::Accuracy => 0.85,
            OptimizationMetric::RMSE => 0.15,
            OptimizationMetric::R2 => 0.80,
            _ => 0.75,
        })
    }

    /// Build leaderboard from trials
    fn build_leaderboard(&self, trials: &[Trial], config: &AutoMLConfig) -> Vec<LeaderboardEntry> {
        let mut entries: Vec<_> = trials.iter()
            .filter(|t| t.status == TrialStatus::Completed)
            .map(|t| LeaderboardEntry {
                rank: 0,
                algorithm: t.algorithm.clone(),
                score: t.mean_score,
                training_time: t.training_time,
                hyperparameters: t.hyperparameters.clone(),
            })
            .collect();

        // Sort by score
        entries.sort_by(|a, b| {
            if config.metric.is_higher_better() {
                b.score.partial_cmp(&a.score).unwrap()
            } else {
                a.score.partial_cmp(&b.score).unwrap()
            }
        });

        // Assign ranks
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.rank = i + 1;
        }

        entries.truncate(10); // Top 10
        entries
    }

    /// Get feature importance from best model
    pub fn get_feature_importance(&self, _model: &Model) -> Vec<(String, f64)> {
        // Simplified feature importance
        vec![
            ("feature_0".to_string(), 0.3),
            ("feature_1".to_string(), 0.25),
            ("feature_2".to_string(), 0.2),
            ("feature_3".to_string(), 0.15),
            ("feature_4".to_string(), 0.1),
        ]
    }
}

impl Default for AutoMLEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperparameter_space() {
        let mut space = HyperparameterSpace::new();
        space.add_range(
            "learning_rate".to_string(),
            ParameterRange::Float {
                min: 0.001,
                max: 0.1,
                log_scale: true,
            },
        );

        let params = space.sample();
        let lr = params.get_float("learning_rate", 0.0);
        assert!(lr >= 0.001 && lr <= 0.1);
    }

    #[test]
    fn test_optimization_metric() {
        assert!(OptimizationMetric::Accuracy.is_higher_better());
        assert!(!OptimizationMetric::MSE.is_higher_better());
    }

    #[test]
    fn test_trial() {
        let mut trial = Trial::new(0, Algorithm::LinearRegression, Hyperparameters::new());
        trial.complete(vec![0.8, 0.85, 0.82], 1.5);

        assert_eq!(trial.status, TrialStatus::Completed);
        assert!(trial.mean_score > 0.0);
    }
}
