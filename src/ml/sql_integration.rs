// # SQL Integration for Machine Learning
//
// This module provides SQL syntax extensions for ML operations, enabling
// seamless integration of machine learning into SQL queries.
//
// ## Supported SQL Syntax
//
// - `CREATE MODEL`: Train a new ML model
// - `DROP MODEL`: Remove a model
// - `PREDICT()`: Make predictions using a trained model
// - `MODEL_INFO()`: Get model metadata
// - `MODEL_METRICS()`: Get model performance metrics
// - `RETRAIN MODEL`: Retrain an existing model
//
// ## Examples
//
// ```sql
// -- Train a model
// CREATE MODEL customer_churn
// USING logistic_regression
// WITH (learning_rate=0.01, max_iterations=1000)
// AS SELECT age, balance, products, churn FROM customers;
//
// -- Make predictions
// SELECT customer_id, PREDICT(customer_churn, age, balance, products) as churn_prob
// FROM new_customers;
//
// -- Get model info
// SELECT MODEL_INFO('customer_churn');
// ```

use super::{
    algorithms::ModelType,
    engine::{MLEngine, ModelMetadata, ModelVersion},
    inference::InferenceEngine,
    Hyperparameters, MLError, Matrix, Vector,
};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// SQL Statement Parsing
// ============================================================================

// CREATE MODEL statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelStatement {
    // Model name
    pub name: String,
    // Algorithm type
    pub algorithm: String,
    // Hyperparameters
    pub hyperparameters: HashMap<String, String>,
    // Training query
    pub training_query: String,
    // Target column name
    pub target_column: Option<String>,
    // Feature columns (if specified)
    pub feature_columns: Option<Vec<String>>,
    // Replace if exists
    pub replace: bool,
}

impl CreateModelStatement {
    // Parse CREATE MODEL statement
    pub fn parse(sql: &str) -> Result<Self> {
        // Simplified parser - in production, integrate with full SQL parser
        let sql = sql.trim();

        // Extract model name
        let name_start = sql
            .find("MODEL")
            .ok_or_else(|| MLError::InvalidConfiguration("Missing MODEL keyword".to_string()))?
            + 5;

        let name_part = &sql[name_start..].trim();
        let name_end = name_part
            .find(|c: char| c.is_whitespace())
            .ok_or_else(|| MLError::InvalidConfiguration("Missing model name".to_string()))?;
        let name = name_part[..name_end].trim().to_string();

        // Extract algorithm
        let using_start = sql
            .find("USING")
            .ok_or_else(|| MLError::InvalidConfiguration("Missing USING keyword".to_string()))?
            + 5;

        let using_part = &sql[using_start..].trim();
        let algorithm_end = using_part
            .find(|c: char| c.is_whitespace() || c == '(' || c == '\n')
            .unwrap_or(using_part.len());
        let algorithm = using_part[..algorithm_end].trim().to_string();

        // Extract hyperparameters (if WITH clause exists)
        let mut hyperparameters = HashMap::new();
        if let Some(with_pos) = sql.find("WITH") {
            let with_part = &sql[with_pos + 4..].trim();
            if let Some(start) = with_part.find('(') {
                if let Some(end) = with_part.find(')') {
                    let params_str = &with_part[start + 1..end];
                    for param in params_str.split(',') {
                        let parts: Vec<&str> = param.split('=').map(|s| s.trim()).collect();
                        if parts.len() == 2 {
                            hyperparameters.insert(parts[0].to_string(), parts[1].to_string());
                        }
                    }
                }
            }
        }

        // Extract training query
        let as_start = sql
            .find("AS")
            .ok_or_else(|| MLError::InvalidConfiguration("Missing AS keyword".to_string()))?
            + 2;
        let training_query = sql[as_start..].trim().to_string();

        Ok(Self {
            name,
            algorithm,
            hyperparameters,
            training_query,
            target_column: None,
            feature_columns: None,
            replace: sql.contains("OR REPLACE"),
        })
    }

    // Convert algorithm string to ModelType
    pub fn get_model_type(&self) -> Result<ModelType> {
        match self.algorithm.to_lowercase().as_str() {
            "linear_regression" | "linearregression" => Ok(ModelType::LinearRegression),
            "logistic_regression" | "logisticregression" => Ok(ModelType::LogisticRegression),
            "decision_tree" | "decisiontree" => Ok(ModelType::DecisionTree),
            "random_forest" | "randomforest" => Ok(ModelType::RandomForest),
            "kmeans" | "k_means" => Ok(ModelType::KMeans),
            "naive_bayes" | "naivebayes" => Ok(ModelType::NaiveBayes),
            _ => Err(MLError::UnsupportedAlgorithm(self.algorithm.clone()).into()),
        }
    }

    // Convert hyperparameters to Hyperparameters struct
    pub fn get_hyperparameters(&self) -> Hyperparameters {
        let mut params = Hyperparameters::new();

        for (key, value) in &self.hyperparameters {
            // Try to parse as different types
            if let Ok(f) = value.parse::<f64>() {
                params.set_float(key, f);
            } else if let Ok(i) = value.parse::<i64>() {
                params.set_int(key, i);
            } else if let Ok(b) = value.parse::<bool>() {
                params.set_bool(key, b);
            } else {
                params.set_string(key, value.clone());
            }
        }

        params
    }
}

// DROP MODEL statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropModelStatement {
    // Model name
    pub name: String,
    // Version (if specified)
    pub version: Option<ModelVersion>,
    // If exists flag
    pub if_exists: bool,
}

impl DropModelStatement {
    // Parse DROP MODEL statement
    pub fn parse(sql: &str) -> Result<Self> {
        let sql = sql.trim();

        let if_exists = sql.contains("IF EXISTS");

        // Extract model name
        let model_start = if if_exists {
            sql.find("EXISTS").unwrap() + 6
        } else {
            sql.find("MODEL").unwrap() + 5
        };

        let name = sql[model_start..].trim().to_string();

        Ok(Self {
            name,
            version: None,
            if_exists,
        })
    }
}

// RETRAIN MODEL statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrainModelStatement {
    // Model name
    pub name: String,
    // New training query (optional)
    pub training_query: Option<String>,
    // New hyperparameters (optional)
    pub hyperparameters: Option<HashMap<String, String>>,
}

impl RetrainModelStatement {
    // Parse RETRAIN MODEL statement
    pub fn parse(sql: &str) -> Result<Self> {
        let sql = sql.trim();

        let name_start = sql
            .find("MODEL")
            .ok_or_else(|| MLError::InvalidConfiguration("Missing MODEL keyword".to_string()))?
            + 5;

        let name_part = &sql[name_start..].trim();
        let name_end = name_part
            .find(|c: char| c.is_whitespace())
            .unwrap_or(name_part.len());
        let name = name_part[..name_end].trim().to_string();

        // Extract new training query if provided
        let training_query = if let Some(as_pos) = sql.find("AS") {
            Some(sql[as_pos + 2..].trim().to_string())
        } else {
            None
        };

        Ok(Self {
            name,
            training_query,
            hyperparameters: None,
        })
    }
}

// ============================================================================
// PREDICT Function
// ============================================================================

// PREDICT function implementation
#[derive(Debug, Clone)]
pub struct PredictFunction {
    // Model name
    pub model_name: String,
    // Model version (optional)
    pub model_version: Option<ModelVersion>,
    // Feature column names
    pub feature_columns: Vec<String>,
}

impl PredictFunction {
    // Create a new PREDICT function
    pub fn new(model_name: String, feature_columns: Vec<String>) -> Self {
        Self {
            model_name,
            model_version: None,
            feature_columns,
        }
    }

    // With specific model version
    pub fn with_version(mut self, version: ModelVersion) -> Self {
        self.model_version = Some(version);
        self
    }

    // Parse PREDICT function call
    pub fn parse(expr: &str) -> Result<Self> {
        let expr = expr.trim();

        if !expr.to_uppercase().starts_with("PREDICT") {
            return Err(MLError::InvalidConfiguration("Not a PREDICT function".to_string()).into());
        }

        // Extract arguments between parentheses
        let start = expr.find('(').ok_or_else(|| {
            MLError::InvalidConfiguration("Missing opening parenthesis".to_string())
        })?;
        let end = expr.rfind(')').ok_or_else(|| {
            MLError::InvalidConfiguration("Missing closing parenthesis".to_string())
        })?;

        let args = &expr[start + 1..end];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

        if parts.is_empty() {
            return Err(MLError::InvalidConfiguration("Missing model name".to_string()).into());
        }

        let model_name = parts[0].to_string();
        let feature_columns = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok(Self::new(model_name, feature_columns))
    }

    // Execute prediction
    pub fn execute(
        &self,
        inference_engine: &InferenceEngine,
        feature_values: &Matrix,
    ) -> Result<Vector> {
        let result =
            inference_engine.predict(&self.model_name, self.model_version, feature_values)?;

        Ok(result.predictions)
    }
}

// ============================================================================
// MODEL_INFO Function
// ============================================================================

// Model info result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub model_type: String,
    pub status: String,
    pub features: Vec<String>,
    pub created_at: u64,
    pub training_samples: usize,
    pub metrics: HashMap<String, f64>,
}

impl ModelInfo {
    // Create from model metadata
    pub fn from_metadata(metadata: &ModelMetadata) -> Self {
        Self {
            name: metadata.name.clone(),
            version: metadata.version.to_string(),
            model_type: format!("{:?}", metadata.model_type),
            status: format!("{}", metadata.status),
            features: metadata.feature_names.clone(),
            created_at: metadata.created_at,
            training_samples: metadata.training_samples,
            metrics: metadata.metrics.clone(),
        }
    }

    // Format as JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            MLError::InvalidConfiguration(format!("JSON serialization failed: {}", e)).into()
        })
    }
}

// ============================================================================
// Model as Virtual Table
// ============================================================================

// Virtual table representing a model's predictions
#[derive(Debug, Clone)]
pub struct ModelTable {
    // Model name
    pub model_name: String,
    // Model version
    pub model_version: Option<ModelVersion>,
    // Source table for features
    pub source_table: String,
    // Feature columns
    pub feature_columns: Vec<String>,
    // Prediction column name
    pub prediction_column: String,
}

impl ModelTable {
    // Create a new model table
    pub fn new(model_name: String, source_table: String, feature_columns: Vec<String>) -> Self {
        Self {
            model_name,
            model_version: None,
            source_table,
            feature_columns,
            prediction_column: "prediction".to_string(),
        }
    }

    // Set prediction column name
    pub fn with_prediction_column(mut self, name: String) -> Self {
        self.prediction_column = name;
        self
    }

    // Generate SQL to create the virtual table view
    pub fn to_sql(&self) -> String {
        let features = self.feature_columns.join(", ");
        format!(
            "CREATE VIEW {}_predictions AS \
             SELECT *, PREDICT('{}', {}) as {} FROM {}",
            self.model_name, self.model_name, features, self.prediction_column, self.source_table
        )
    }
}

// ============================================================================
// ML SQL Parser
// ============================================================================

// ML-specific SQL parser
pub struct MLSqlParser {
    // ML engine reference
    ml_engine: Arc<MLEngine>,
}

impl MLSqlParser {
    // Create a new ML SQL parser
    pub fn new(ml_engine: Arc<MLEngine>) -> Self {
        Self { ml_engine }
    }

    // Check if SQL statement is ML-related
    pub fn is_ml_statement(sql: &str) -> bool {
        let sql_upper = sql.trim().to_uppercase();
        sql_upper.starts_with("CREATE MODEL")
            || sql_upper.starts_with("DROP MODEL")
            || sql_upper.starts_with("RETRAIN MODEL")
            || sql_upper.contains("PREDICT(")
            || sql_upper.contains("MODEL_INFO(")
            || sql_upper.contains("MODEL_METRICS(")
    }

    // Parse and execute ML statement
    pub fn parse_and_execute(&self, sql: &str) -> Result<MLExecutionResult> {
        let sql_upper = sql.trim().to_uppercase();

        if sql_upper.starts_with("CREATE MODEL") {
            self.execute_create_model(sql)
        } else if sql_upper.starts_with("DROP MODEL") {
            self.execute_drop_model(sql)
        } else if sql_upper.starts_with("RETRAIN MODEL") {
            self.execute_retrain_model(sql)
        } else {
            Err(MLError::InvalidConfiguration(format!("Unsupported ML statement: {}", sql)).into())
        }
    }

    // Execute CREATE MODEL
    fn execute_create_model(&self, sql: &str) -> Result<MLExecutionResult> {
        let stmt = CreateModelStatement::parse(sql)?;
        let model_type = stmt.get_model_type()?;
        let _hyperparameters = stmt.get_hyperparameters();

        // In production, this would execute the training query to get data
        // For now, return a placeholder result
        Ok(MLExecutionResult::ModelCreated {
            name: stmt.name,
            model_type: format!("{:?}", model_type),
            message: "Model training initiated".to_string(),
        })
    }

    // Execute DROP MODEL
    fn execute_drop_model(&self, sql: &str) -> Result<MLExecutionResult> {
        let stmt = DropModelStatement::parse(sql)?;

        match self.ml_engine.registry().delete(&stmt.name, stmt.version) {
            Ok(_) => Ok(MLExecutionResult::ModelDropped {
                name: stmt.name,
                message: "Model deleted successfully".to_string(),
            }),
            Err(e) => {
                if stmt.if_exists {
                    Ok(MLExecutionResult::ModelDropped {
                        name: stmt.name,
                        message: "Model does not exist (IF EXISTS)".to_string(),
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    // Execute RETRAIN MODEL
    fn execute_retrain_model(&self, sql: &str) -> Result<MLExecutionResult> {
        let stmt = RetrainModelStatement::parse(sql)?;

        // Get existing model to determine type
        let stored = self.ml_engine.registry().get(&stmt.name, None)?;
        let model_type = stored.metadata.model_type;

        Ok(MLExecutionResult::ModelRetrained {
            name: stmt.name,
            model_type: format!("{:?}", model_type),
            message: "Model retraining initiated".to_string(),
        })
    }
}

// Result of ML SQL execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MLExecutionResult {
    ModelCreated {
        name: String,
        model_type: String,
        message: String,
    },
    ModelDropped {
        name: String,
        message: String,
    },
    ModelRetrained {
        name: String,
        model_type: String,
        message: String,
    },
    PredictionResult {
        predictions: Vec<f64>,
        metadata: HashMap<String, String>,
    },
    ModelInfo {
        info: ModelInfo,
    },
}

impl MLExecutionResult {
    // Convert to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            MLError::InvalidConfiguration(format!("JSON serialization failed: {}", e)).into()
        })
    }
}

// ============================================================================
// Feature Detection
// ============================================================================

// Automatic feature detection from schema
pub struct FeatureDetector;

impl FeatureDetector {
    // Detect numeric features from column types
    pub fn detect_numeric_features(columns: &[(String, String)]) -> Vec<String> {
        columns
            .iter()
            .filter(|(_, col_type)| {
                let ct = col_type.to_uppercase();
                ct.contains("INT")
                    || ct.contains("FLOAT")
                    || ct.contains("DOUBLE")
                    || ct.contains("DECIMAL")
                    || ct.contains("NUMERIC")
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    // Detect categorical features
    pub fn detect_categorical_features(columns: &[(String, String)]) -> Vec<String> {
        columns
            .iter()
            .filter(|(_, col_type)| {
                let ct = col_type.to_uppercase();
                ct.contains("VARCHAR")
                    || ct.contains("TEXT")
                    || ct.contains("CHAR")
                    || ct.contains("ENUM")
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    // Suggest preprocessing steps based on column types
    pub fn suggest_preprocessing(columns: &[(String, String)]) -> Vec<String> {
        let mut suggestions = Vec::new();

        let numeric = Self::detect_numeric_features(columns);
        let categorical = Self::detect_categorical_features(columns);

        if !numeric.is_empty() {
            suggestions.push(format!(
                "Consider standardizing numeric features: {}",
                numeric.join(", ")
            ));
        }

        if !categorical.is_empty() {
            suggestions.push(format!(
                "Consider one-hot encoding categorical features: {}",
                categorical.join(", ")
            ));
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_model_parse() {
        let sql = "CREATE MODEL churn_model USING logistic_regression \
                   WITH (learning_rate=0.01, max_iterations=1000) \
                   AS SELECT age, balance FROM customers";

        let stmt = CreateModelStatement::parse(sql).unwrap();
        assert_eq!(stmt.name, "churn_model");
        assert_eq!(stmt.algorithm, "logistic_regression");
        assert_eq!(
            stmt.hyperparameters.get("learning_rate"),
            Some(&"0.01".to_string())
        );
    }

    #[test]
    fn test_drop_model_parse() {
        let sql = "DROP MODEL IF EXISTS churn_model";
        let stmt = DropModelStatement::parse(sql).unwrap();
        assert_eq!(stmt.name, "churn_model");
        assert!(stmt.if_exists);
    }

    #[test]
    fn test_predict_function_parse() {
        let expr = "PREDICT(churn_model, age, balance, products)";
        let func = PredictFunction::parse(expr).unwrap();
        assert_eq!(func.model_name, "churn_model");
        assert_eq!(func.feature_columns.len(), 3);
    }

    #[test]
    fn test_feature_detection() {
        let columns = vec![
            ("age".to_string(), "INTEGER".to_string()),
            ("name".to_string(), "VARCHAR".to_string()),
            ("balance".to_string(), "FLOAT".to_string()),
            ("category".to_string(), "TEXT".to_string()),
        ];

        let numeric = FeatureDetector::detect_numeric_features(&columns);
        assert_eq!(numeric.len(), 2);
        assert!(numeric.contains(&"age".to_string()));
        assert!(numeric.contains(&"balance".to_string()));

        let categorical = FeatureDetector::detect_categorical_features(&columns);
        assert_eq!(categorical.len(), 2);
        assert!(categorical.contains(&"name".to_string()));
        assert!(categorical.contains(&"category".to_string()));
    }

    #[test]
    fn test_model_table() {
        let table = ModelTable::new(
            "churn_model".to_string(),
            "customers".to_string(),
            vec!["age".to_string(), "balance".to_string()],
        );

        let sql = table.to_sql();
        assert!(sql.contains("CREATE VIEW"));
        assert!(sql.contains("churn_model_predictions"));
        assert!(sql.contains("PREDICT"));
    }
}
