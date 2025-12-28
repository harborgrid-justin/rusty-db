# RustyDB v0.6 Machine Learning Engine

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: Data Scientists, ML Engineers, Application Developers

---

## Table of Contents

1. [Overview](#overview)
2. [Supported Algorithms](#supported-algorithms)
3. [Model Training](#model-training)
4. [Model Inference](#model-inference)
5. [AutoML](#automl)
6. [Feature Engineering](#feature-engineering)
7. [Model Management](#model-management)
8. [Time Series Forecasting](#time-series-forecasting)
9. [A/B Testing](#ab-testing)
10. [SQL Integration](#sql-integration)
11. [Performance Optimization](#performance-optimization)
12. [Best Practices](#best-practices)

---

## Overview

RustyDB's ML Engine brings machine learning capabilities directly into the database, enabling **in-database training and inference** without data movement. This approach eliminates ETL overhead, reduces latency, and leverages database optimizations for ML workloads.

### Key Features

**In-Database Training**:
- Train models directly on database tables
- No data export/import required
- Leverage SQL for data preparation

**AutoML**:
- Automatic algorithm selection
- Hyperparameter tuning with grid/random search
- Cross-validation for model selection

**Production-Ready**:
- Model versioning and registry
- A/B testing framework
- PMML export for interoperability

**Performance**:
- SIMD-accelerated operations
- Parallel training
- Incremental learning

---

## Supported Algorithms

### Classification

**Logistic Regression**:
```sql
-- Binary classification
CREATE MODEL credit_risk_model
USING logistic_regression
FROM customer_data
TARGET is_default
FEATURES (income, debt_ratio, credit_score, employment_years);

-- Predict
SELECT customer_id,
       PREDICT(credit_risk_model, customer_id) as risk_score
FROM new_customers;
```

**Decision Trees**:
```sql
CREATE MODEL churn_prediction
USING decision_tree
FROM customer_behavior
TARGET churned
FEATURES (tenure, monthly_charges, total_charges, contract_type)
WITH (
  max_depth = 10,
  min_samples_split = 20,
  criterion = 'gini'
);
```

**Random Forest**:
```sql
CREATE MODEL fraud_detection
USING random_forest
FROM transactions
TARGET is_fraud
FEATURES (amount, merchant_category, time_of_day, location)
WITH (
  n_estimators = 100,
  max_depth = 15,
  min_samples_leaf = 5
);
```

**Naive Bayes**:
```sql
CREATE MODEL spam_filter
USING naive_bayes
FROM emails
TARGET is_spam
FEATURES (subject_length, has_links, keyword_count, sender_domain);
```

### Regression

**Linear Regression**:
```sql
CREATE MODEL price_prediction
USING linear_regression
FROM housing_data
TARGET sale_price
FEATURES (square_feet, bedrooms, bathrooms, year_built, zip_code);

-- Predict with confidence intervals
SELECT property_id,
       PREDICT(price_prediction, property_id) as predicted_price,
       PREDICT_CONFIDENCE(price_prediction, property_id, 0.95) as confidence_interval
FROM new_listings;
```

**Polynomial Regression**:
```sql
CREATE MODEL growth_model
USING linear_regression
FROM company_metrics
TARGET revenue
FEATURES (POLYNOMIAL(year, degree => 3), employees, marketing_spend);
```

### Clustering

**K-Means**:
```sql
CREATE MODEL customer_segments
USING kmeans
FROM customer_features
FEATURES (age, income, purchase_frequency, avg_order_value)
WITH (
  n_clusters = 5,
  max_iterations = 300
);

-- Assign clusters
SELECT customer_id,
       PREDICT_CLUSTER(customer_segments, customer_id) as segment
FROM customers;
```

**DBSCAN**:
```sql
CREATE MODEL anomaly_detector
USING dbscan
FROM network_traffic
FEATURES (bytes_sent, bytes_received, connection_duration, port)
WITH (
  eps = 0.5,
  min_samples = 5
);
```

### Neural Networks (Basic)

```sql
CREATE MODEL image_classifier
USING neural_network
FROM image_features
TARGET category
FEATURES (pixel_data)
WITH (
  hidden_layers = [128, 64, 32],
  activation = 'relu',
  output_activation = 'softmax',
  epochs = 50,
  learning_rate = 0.001
);
```

---

## Model Training

### Training Workflow

```sql
-- 1. Prepare data
CREATE VIEW training_data AS
SELECT user_id,
       age,
       income,
       CASE WHEN purchase_count > 5 THEN 1 ELSE 0 END as high_value
FROM users
WHERE created_at < '2025-01-01';

-- 2. Create and train model
CREATE MODEL customer_value_model
USING random_forest
FROM training_data
TARGET high_value
FEATURES (age, income)
WITH (
  n_estimators = 100,
  test_split = 0.2,  -- 80/20 train/test split
  random_state = 42
);

-- 3. Evaluate model
SELECT * FROM EVALUATE_MODEL(customer_value_model);

-- Output:
-- accuracy: 0.87
-- precision: 0.85
-- recall: 0.82
-- f1_score: 0.83
-- roc_auc: 0.91
```

### Training Options

```sql
CREATE MODEL advanced_model
USING gradient_boosting
FROM large_dataset
TARGET outcome
FEATURES (col1, col2, col3)
WITH (
  -- Data splitting
  test_split = 0.2,
  validation_split = 0.1,

  -- Training parameters
  epochs = 100,
  batch_size = 1024,
  learning_rate = 0.01,

  -- Regularization
  l1_penalty = 0.01,
  l2_penalty = 0.001,

  -- Early stopping
  early_stopping = true,
  patience = 10,

  -- Performance
  parallel_workers = 8,
  use_gpu = false
);
```

### Incremental Learning

```sql
-- Initial training
CREATE MODEL online_model
USING logistic_regression
FROM historical_data
TARGET label
FEATURES (f1, f2, f3);

-- Update with new data
UPDATE MODEL online_model
FROM new_data
WITH (
  incremental = true,
  learning_rate = 0.001
);
```

---

## Model Inference

### Batch Prediction

```sql
-- Predict on entire table
SELECT user_id,
       PREDICT(churn_model, user_id) as churn_probability,
       CASE
         WHEN PREDICT(churn_model, user_id) > 0.7 THEN 'High Risk'
         WHEN PREDICT(churn_model, user_id) > 0.4 THEN 'Medium Risk'
         ELSE 'Low Risk'
       END as risk_category
FROM active_users;
```

### Real-Time Prediction

```sql
-- Single prediction
SELECT PREDICT(fraud_model,
  amount => 1500.00,
  merchant => 'ELECTRONICS_STORE',
  time_of_day => 23,
  location => 'foreign'
) as fraud_score;
```

### Confidence Intervals

```sql
SELECT property_id,
       PREDICT(price_model, property_id) as predicted_price,
       PREDICT_CONFIDENCE(price_model, property_id, 0.95) as ci_95
FROM listings;

-- Output:
-- property_id | predicted_price | ci_95
-- 1001        | 450000          | [420000, 480000]
```

---

## AutoML

### Automatic Algorithm Selection

```sql
CREATE MODEL automl_best_model
USING automl
FROM customer_data
TARGET target_variable
FEATURES (feature1, feature2, feature3)
WITH (
  algorithms = ['logistic_regression', 'random_forest', 'gradient_boosting'],
  metric = 'f1_score',
  cv_folds = 5,
  timeout_minutes = 60
);

-- View AutoML results
SELECT * FROM AUTOML_REPORT(automl_best_model);

-- Output:
-- algorithm           | score  | training_time | parameters
-- random_forest       | 0.89   | 45s          | {n_estimators: 150, max_depth: 12}
-- gradient_boosting   | 0.87   | 120s         | {learning_rate: 0.05, n_estimators: 200}
-- logistic_regression | 0.82   | 5s           | {C: 0.1, penalty: 'l2'}
```

### Hyperparameter Tuning

**Grid Search**:
```sql
CREATE MODEL tuned_model
USING random_forest
FROM training_data
TARGET target
FEATURES (f1, f2, f3)
WITH HYPERPARAMETER_SEARCH (
  method = 'grid',
  parameters = {
    'n_estimators': [50, 100, 150, 200],
    'max_depth': [5, 10, 15, 20],
    'min_samples_split': [2, 5, 10]
  },
  cv_folds = 5,
  metric = 'accuracy'
);
```

**Random Search**:
```sql
CREATE MODEL random_tuned_model
USING gradient_boosting
FROM training_data
TARGET target
FEATURES (f1, f2, f3)
WITH HYPERPARAMETER_SEARCH (
  method = 'random',
  n_iterations = 50,
  parameters = {
    'learning_rate': {'type': 'uniform', 'min': 0.001, 'max': 0.1},
    'n_estimators': {'type': 'int', 'min': 50, 'max': 500},
    'max_depth': {'type': 'int', 'min': 3, 'max': 20}
  }
);
```

---

## Feature Engineering

### Built-in Transformations

**Normalization**:
```sql
CREATE MODEL normalized_model
USING linear_regression
FROM data
TARGET target
FEATURES (
  NORMALIZE(age, method => 'minmax'),  -- Scale to [0, 1]
  NORMALIZE(income, method => 'standard')  -- Z-score normalization
);
```

**Encoding**:
```sql
CREATE MODEL encoded_model
USING logistic_regression
FROM data
TARGET target
FEATURES (
  -- One-hot encoding
  ONEHOT(category, max_categories => 10),

  -- Label encoding
  LABEL_ENCODE(ordinal_feature),

  -- Target encoding
  TARGET_ENCODE(high_cardinality_feature, target)
);
```

**Polynomial Features**:
```sql
CREATE MODEL poly_model
USING linear_regression
FROM data
TARGET target
FEATURES (
  POLYNOMIAL(x, degree => 2),  -- x, x^2
  POLYNOMIAL([x, y], degree => 2)  -- x, y, x^2, xy, y^2
);
```

**Binning**:
```sql
SELECT
  BIN(age, bins => [0, 18, 35, 50, 65, 100]) as age_group,
  BIN(income, n_bins => 5, strategy => 'quantile') as income_bracket
FROM customers;
```

### Custom Feature Functions

```sql
CREATE FEATURE FUNCTION customer_lifetime_value(
  purchase_count INT,
  avg_order_value FLOAT,
  tenure_months INT
) RETURNS FLOAT AS $$
BEGIN
  RETURN purchase_count * avg_order_value * (tenure_months / 12.0);
END;
$$ LANGUAGE plpgsql;

-- Use in model
CREATE MODEL clv_model
USING linear_regression
FROM customers
TARGET future_revenue
FEATURES (
  customer_lifetime_value(purchase_count, avg_order_value, tenure_months),
  tenure_months,
  avg_order_value
);
```

---

## Model Management

### Model Registry

```sql
-- List all models
SELECT model_name,
       algorithm,
       created_at,
       version,
       metrics,
       status
FROM system.ml_models;

-- Get model details
SELECT * FROM MODEL_INFO('customer_churn_model');

-- Output:
-- name: customer_churn_model
-- version: 3
-- algorithm: random_forest
-- features: [tenure, monthly_charges, total_charges]
-- target: churned
-- training_samples: 5000
-- test_accuracy: 0.87
-- created_at: 2025-12-15 10:30:00
```

### Versioning

```sql
-- Create new version
UPDATE MODEL customer_churn_model
FROM new_training_data
WITH (create_new_version = true);

-- List versions
SELECT version,
       created_at,
       test_accuracy,
       is_production
FROM MODEL_VERSIONS('customer_churn_model');

-- Rollback to previous version
ALTER MODEL customer_churn_model
SET VERSION = 2;

-- Delete old versions
DELETE FROM MODEL_VERSIONS('customer_churn_model')
WHERE version < 3 AND is_production = false;
```

### Model Export/Import

**PMML Export**:
```sql
-- Export to PMML (industry standard)
SELECT EXPORT_MODEL('price_prediction', format => 'pmml')
TO '/models/price_prediction.pmml';

-- Import PMML
CREATE MODEL imported_model
FROM PMML '/models/external_model.pmml';
```

**Pickle Export** (Python):
```sql
-- Export to Python pickle
SELECT EXPORT_MODEL('sklearn_compatible', format => 'pickle')
TO '/models/sklearn_model.pkl';
```

---

## Time Series Forecasting

### ARIMA Models

```sql
CREATE MODEL sales_forecast
USING arima
FROM monthly_sales
TIME_COLUMN month
TARGET sales_amount
WITH (
  p = 2,  -- AR order
  d = 1,  -- Differencing
  q = 2,  -- MA order
  seasonal = true,
  seasonal_period = 12  -- Monthly data, yearly seasonality
);

-- Forecast next 6 months
SELECT * FROM FORECAST(sales_forecast, horizon => 6);

-- Output:
-- month       | predicted_sales | lower_bound | upper_bound
-- 2026-01-01  | 125000         | 118000      | 132000
-- 2026-02-01  | 128000         | 120000      | 136000
-- ...
```

### Exponential Smoothing

```sql
CREATE MODEL demand_forecast
USING exponential_smoothing
FROM daily_demand
TIME_COLUMN date
TARGET demand
WITH (
  trend = 'additive',
  seasonal = 'multiplicative',
  seasonal_periods = 7  -- Weekly seasonality
);
```

### Anomaly Detection

```sql
-- Detect anomalies in time series
SELECT date,
       value,
       DETECT_ANOMALY(timeseries_model, date, value) as is_anomaly,
       ANOMALY_SCORE(timeseries_model, date, value) as score
FROM sensor_data;
```

---

## A/B Testing

### Setup A/B Test

```sql
-- Create multiple model variants
CREATE MODEL recommender_v1
USING collaborative_filtering
FROM user_interactions
WITH (method => 'item_based');

CREATE MODEL recommender_v2
USING collaborative_filtering
FROM user_interactions
WITH (method => 'user_based');

-- Create A/B test
CREATE AB_TEST recommendation_test
VARIANTS (
  control => recommender_v1,
  treatment => recommender_v2
)
WITH (
  traffic_split = [0.5, 0.5],
  metric = 'click_through_rate',
  minimum_sample_size = 1000
);
```

### Monitor A/B Test

```sql
-- View A/B test results
SELECT * FROM AB_TEST_RESULTS('recommendation_test');

-- Output:
-- variant   | requests | conversions | conversion_rate | p_value | winner
-- control   | 5000     | 250        | 0.050          | 0.032   | false
-- treatment | 5000     | 312        | 0.062          | 0.032   | true
```

### Deploy Winner

```sql
-- Automatically deploy winning variant
ALTER AB_TEST recommendation_test
DEPLOY_WINNER WITH (confidence_threshold = 0.95);
```

---

## SQL Integration

### Model as Function

```sql
-- Use model in WHERE clause
SELECT * FROM customers
WHERE PREDICT(churn_model, customer_id) > 0.7;

-- Use in SELECT
SELECT customer_id,
       name,
       PREDICT(lifetime_value_model, customer_id) as predicted_ltv
FROM customers
ORDER BY predicted_ltv DESC
LIMIT 100;

-- Use in JOIN
SELECT c.customer_id,
       c.name,
       p.predicted_segment
FROM customers c
JOIN LATERAL (
  SELECT PREDICT_CLUSTER(segment_model, c.customer_id) as predicted_segment
) p ON true;
```

### Batch Scoring

```sql
-- Create materialized predictions
CREATE MATERIALIZED VIEW customer_predictions AS
SELECT customer_id,
       PREDICT(churn_model, customer_id) as churn_probability,
       PREDICT(ltv_model, customer_id) as predicted_ltv,
       PREDICT_CLUSTER(segment_model, customer_id) as segment
FROM customers;

-- Refresh periodically
REFRESH MATERIALIZED VIEW customer_predictions;
```

---

## Performance Optimization

### SIMD Acceleration

RustyDB's ML engine uses SIMD (AVX2/AVX-512) for 4-8x performance improvement:

```rust
// Automatic SIMD vectorization for:
- Matrix operations
- Dot products
- Element-wise operations
- Distance calculations
```

**Enable SIMD**:
```sql
ALTER SYSTEM SET ml_use_simd = true;
```

### Parallel Training

```sql
CREATE MODEL parallel_model
USING random_forest
FROM large_dataset
TARGET target
FEATURES (f1, f2, f3)
WITH (
  parallel_workers = 16,
  parallel_mode = 'data'  -- or 'model' for ensemble parallelism
);
```

### GPU Acceleration

```sql
CREATE MODEL gpu_model
USING neural_network
FROM image_data
TARGET category
FEATURES (pixels)
WITH (
  use_gpu = true,
  gpu_id = 0
);
```

---

## Best Practices

1. **Data Preparation**: Clean and normalize data before training
2. **Train/Test Split**: Always validate on held-out data (20-30% test split)
3. **Cross-Validation**: Use k-fold CV for robust evaluation
4. **Feature Scaling**: Normalize features for distance-based algorithms
5. **Hyperparameter Tuning**: Use AutoML or grid/random search
6. **Model Versioning**: Keep track of model versions in production
7. **A/B Testing**: Test new models before full deployment
8. **Monitoring**: Track model performance metrics over time
9. **Retraining**: Retrain models regularly as data distribution shifts
10. **Documentation**: Document feature engineering and model choices

---

**See Also**:
- [ML Engine Test Report](/docs/ML_ENGINE_TEST_REPORT.md)
- [Performance Tuning](../operations/PERFORMANCE_TUNING.md)
- [Data Preparation Guide](../guides/DATA_PREPARATION.md)

**Document Version**: 1.0
**Last Updated**: December 2025
