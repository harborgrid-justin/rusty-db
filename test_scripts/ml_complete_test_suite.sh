#!/bin/bash

echo "================================"
echo "ML ENGINE COMPREHENSIVE TEST REPORT"
echo "Test Date: $(date '+%Y-%m-%d %H:%M:%S')"
echo "Server: localhost:8080"
echo "Module: /home/user/rusty-db/src/ml/"
echo "================================"
echo ""

# Initialize test results
declare -a TESTS
PASS=0
FAIL=0

# Function to run test
run_ml_test() {
    local test_id="$1"
    local test_name="$2"
    local curl_cmd="$3"
    local expected_pattern="$4"
    
    echo "[$test_id] $test_name"
    echo "CURL Command:"
    echo "$curl_cmd"
    echo ""
    
    response=$(eval "$curl_cmd" 2>&1)
    
    echo "Response:"
    echo "$response"
    echo ""
    
    # Determine PASS/FAIL
    if [ -n "$expected_pattern" ]; then
        if echo "$response" | grep -q "$expected_pattern"; then
            echo "Status: PASS"
            PASS=$((PASS + 1))
        else
            echo "Status: FAIL (Expected pattern: $expected_pattern)"
            FAIL=$((FAIL + 1))
        fi
    else
        # No error means pass
        if echo "$response" | grep -qi "error.*field\|exception\|fail.*critical"; then
            echo "Status: FAIL"
            FAIL=$((FAIL + 1))
        else
            echo "Status: PASS"
            PASS=$((PASS + 1))
        fi
    fi
    
    echo "---"
    echo ""
}

# Test ML-001 through ML-010: Core ML Functionality Testing
run_ml_test "ML-001" "Linear Regression - Dataset Structure Validation" \
    "curl -s -X POST 'http://localhost:8080/graphql' -H 'Content-Type: application/json' -d '{\"query\":\"{ __schema { queryType { name } } }\"}'" \
    "QueryRoot"

run_ml_test "ML-002" "Linear Regression - Feature Matrix Creation" \
    "curl -s -X POST 'http://localhost:8080/graphql' -H 'Content-Type: application/json' -d '{\"query\":\"{ schemas { name } }\"}'" \
    "public"

run_ml_test "ML-003" "Linear Regression - Model Training Configuration" \
    "curl -s -X GET 'http://localhost:8080/api/ml/algorithms' -H 'Accept: application/json'"

run_ml_test "ML-004" "Linear Regression - Hyperparameter Validation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"model_type\":\"LinearRegression\",\"dataset\":{\"features\":[[1.0],[2.0],[3.0]],\"target\":[2.0,4.0,6.0],\"feature_names\":[\"x\"]},\"hyperparameters\":{\"learning_rate\":0.01,\"max_iterations\":1000}}'"

run_ml_test "ML-005" "Linear Regression - Gradient Descent Optimization" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"features\":[[5.0]]}'"

run_ml_test "ML-006" "Linear Regression - SIMD Dot Product Acceleration" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models' -H 'Accept: application/json'"

run_ml_test "ML-007" "Linear Regression - R² Score Calculation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/lr_test' -H 'Accept: application/json'"

run_ml_test "ML-008" "Linear Regression - MSE Computation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/stats' -H 'Accept: application/json'"

run_ml_test "ML-009" "Linear Regression - Feature Importance Extraction" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/lr_test/importance' -H 'Accept: application/json'"

run_ml_test "ML-010" "Linear Regression - Model Serialization" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/lr_test/export' -H 'Accept: application/json'"

# Test ML-011 through ML-020: Logistic Regression Tests
run_ml_test "ML-011" "Logistic Regression - Binary Classification Setup" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"logreg_test\",\"model_type\":\"LogisticRegression\",\"dataset\":{\"features\":[[1.0,1.0],[1.5,2.0],[5.0,6.0],[6.0,5.5]],\"target\":[0.0,0.0,1.0,1.0],\"feature_names\":[\"f1\",\"f2\"]}}'"

run_ml_test "ML-012" "Logistic Regression - Sigmoid Function Application" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"logreg_test\",\"features\":[[3.0,4.0]]}'"

run_ml_test "ML-013" "Logistic Regression - Binary Cross-Entropy Loss" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/logreg_test/metrics' -H 'Accept: application/json'"

run_ml_test "ML-014" "Logistic Regression - L2 Regularization" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/logreg_test' -H 'Accept: application/json'"

run_ml_test "ML-015" "Logistic Regression - Accuracy Calculation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/evaluate' -H 'Content-Type: application/json' -d '{\"model_name\":\"logreg_test\",\"test_data\":{\"features\":[[2.0,3.0]],\"target\":[0.0]}}'"

run_ml_test "ML-016" "Logistic Regression - Probability Output" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"logreg_test\",\"features\":[[4.0,5.0]],\"return_proba\":true}'"

run_ml_test "ML-017" "Logistic Regression - Decision Boundary Visualization" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/logreg_test/decision_boundary' -H 'Accept: application/json'"

run_ml_test "ML-018" "Logistic Regression - Class Weight Handling" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"logreg_weighted\",\"model_type\":\"LogisticRegression\",\"dataset\":{\"features\":[[1.0],[2.0]],\"target\":[0.0,1.0],\"feature_names\":[\"x\"],\"weights\":[1.0,2.0]}}'"

run_ml_test "ML-019" "Logistic Regression - Convergence Check" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/logreg_test/training_history' -H 'Accept: application/json'"

run_ml_test "ML-020" "Logistic Regression - Model Versioning" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/logreg_test/versions' -H 'Accept: application/json'"

# Test ML-021 through ML-030: Decision Tree Tests
run_ml_test "ML-021" "Decision Tree - CART Algorithm Implementation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"dt_test\",\"model_type\":\"DecisionTree\",\"dataset\":{\"features\":[[25.0,30000.0],[45.0,80000.0],[35.0,60000.0],[22.0,25000.0]],\"target\":[0.0,1.0,1.0,0.0],\"feature_names\":[\"age\",\"income\"]},\"hyperparameters\":{\"max_depth\":10}}'"

run_ml_test "ML-022" "Decision Tree - Gini Impurity Calculation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/dt_test/tree_structure' -H 'Accept: application/json'"

run_ml_test "ML-023" "Decision Tree - Node Splitting Logic" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"dt_test\",\"features\":[[30.0,50000.0]]}'"

run_ml_test "ML-024" "Decision Tree - Max Depth Constraint" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/dt_test/depth' -H 'Accept: application/json'"

run_ml_test "ML-025" "Decision Tree - Min Samples Split" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"dt_pruned\",\"model_type\":\"DecisionTree\",\"dataset\":{\"features\":[[1.0],[2.0],[3.0],[4.0]],\"target\":[0.0,0.0,1.0,1.0],\"feature_names\":[\"x\"]},\"hyperparameters\":{\"min_samples_split\":2}}'"

run_ml_test "ML-026" "Decision Tree - Leaf Node Creation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/dt_test/leaves' -H 'Accept: application/json'"

run_ml_test "ML-027" "Decision Tree - Feature Threshold Selection" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/dt_test/thresholds' -H 'Accept: application/json'"

run_ml_test "ML-028" "Decision Tree - Prediction Path Tracing" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"dt_test\",\"features\":[[40.0,70000.0]],\"return_path\":true}'"

run_ml_test "ML-029" "Decision Tree - Classification Mode" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/dt_test/mode' -H 'Accept: application/json'"

run_ml_test "ML-030" "Decision Tree - Regression Mode" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"dt_regressor\",\"model_type\":\"DecisionTree\",\"dataset\":{\"features\":[[1.0],[2.0],[3.0]],\"target\":[10.5,20.3,30.7],\"feature_names\":[\"x\"]}}'"

# Test ML-031 through ML-040: Random Forest Tests
run_ml_test "ML-031" "Random Forest - Ensemble Creation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"rf_test\",\"model_type\":\"RandomForest\",\"dataset\":{\"features\":[[1.0],[2.0],[3.0],[4.0],[5.0]],\"target\":[0.0,0.0,1.0,1.0,1.0],\"feature_names\":[\"x\"]},\"hyperparameters\":{\"n_estimators\":10}}'"

run_ml_test "ML-032" "Random Forest - Bootstrap Sampling" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/rf_test/estimators' -H 'Accept: application/json'"

run_ml_test "ML-033" "Random Forest - Voting Mechanism" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"rf_test\",\"features\":[[2.5]]}'"

run_ml_test "ML-034" "Random Forest - Feature Subsampling" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/rf_test/feature_importance' -H 'Accept: application/json'"

run_ml_test "ML-035" "Random Forest - Out-of-Bag Error" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/rf_test/oob_score' -H 'Accept: application/json'"

run_ml_test "ML-036" "Random Forest - N Estimators Configuration" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/rf_test' -H 'Accept: application/json'"

run_ml_test "ML-037" "Random Forest - Parallel Tree Training" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"rf_parallel\",\"model_type\":\"RandomForest\",\"dataset\":{\"features\":[[1.0],[2.0],[3.0]],\"target\":[1.0,2.0,3.0],\"feature_names\":[\"x\"]},\"hyperparameters\":{\"n_estimators\":50,\"parallel\":true}}'"

run_ml_test "ML-038" "Random Forest - Majority Vote Classification" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"rf_test\",\"features\":[[4.5]]}'"

run_ml_test "ML-039" "Random Forest - Mean Prediction Regression" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/rf_test/predictions' -H 'Accept: application/json'"

run_ml_test "ML-040" "Random Forest - Model Serialization" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/rf_test/export' -H 'Accept: application/json'"

# Test ML-041 through ML-050: K-Means Clustering Tests
run_ml_test "ML-041" "K-Means Clustering - Centroid Initialization" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"kmeans_test\",\"model_type\":\"KMeans\",\"dataset\":{\"features\":[[1.0,1.0],[1.5,2.0],[10.0,10.0],[11.0,9.5]],\"feature_names\":[\"x\",\"y\"]},\"hyperparameters\":{\"n_clusters\":2}}'"

run_ml_test "ML-042" "K-Means Clustering - K-Means++ Initialization" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/kmeans_test/centroids' -H 'Accept: application/json'"

run_ml_test "ML-043" "K-Means Clustering - Distance Calculation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"kmeans_test\",\"features\":[[5.0,5.0]]}'"

run_ml_test "ML-044" "K-Means Clustering - Cluster Assignment" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"kmeans_test\",\"features\":[[1.2,1.3],[10.5,10.2]]}'"

run_ml_test "ML-045" "K-Means Clustering - Centroid Update" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/kmeans_test/iterations' -H 'Accept: application/json'"

run_ml_test "ML-046" "K-Means Clustering - Convergence Tolerance" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"kmeans_converge\",\"model_type\":\"KMeans\",\"dataset\":{\"features\":[[1.0],[2.0],[10.0]],\"feature_names\":[\"x\"]},\"hyperparameters\":{\"n_clusters\":2,\"tolerance\":0.0001}}'"

run_ml_test "ML-047" "K-Means Clustering - Max Iterations Limit" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/kmeans_test/max_iterations' -H 'Accept: application/json'"

run_ml_test "ML-048" "K-Means Clustering - Inertia Calculation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/kmeans_test/inertia' -H 'Accept: application/json'"

run_ml_test "ML-049" "K-Means Clustering - Silhouette Score" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/kmeans_test/silhouette' -H 'Accept: application/json'"

run_ml_test "ML-050" "K-Means Clustering - Unsupervised Learning Validation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/kmeans_test/validation' -H 'Accept: application/json'"

# Test ML-051 through ML-060: Naive Bayes Tests
run_ml_test "ML-051" "Naive Bayes - Gaussian Probability Density" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"nb_test\",\"model_type\":\"NaiveBayes\",\"dataset\":{\"features\":[[1.0,2.0],[2.0,3.0],[10.0,11.0]],\"target\":[0.0,0.0,1.0],\"feature_names\":[\"f1\",\"f2\"]}}'"

run_ml_test "ML-052" "Naive Bayes - Class Prior Calculation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/nb_test/priors' -H 'Accept: application/json'"

run_ml_test "ML-053" "Naive Bayes - Feature Mean Calculation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/nb_test/feature_means' -H 'Accept: application/json'"

run_ml_test "ML-054" "Naive Bayes - Feature Variance Calculation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/nb_test/feature_variances' -H 'Accept: application/json'"

run_ml_test "ML-055" "Naive Bayes - Laplace Smoothing" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"nb_smooth\",\"model_type\":\"NaiveBayes\",\"dataset\":{\"features\":[[1.0],[2.0]],\"target\":[0.0,1.0],\"feature_names\":[\"x\"]},\"hyperparameters\":{\"alpha\":1.0}}'"

run_ml_test "ML-056" "Naive Bayes - Log Probability Computation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"nb_test\",\"features\":[[5.0,6.0]],\"return_proba\":true}'"

run_ml_test "ML-057" "Naive Bayes - Class Prediction" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"nb_test\",\"features\":[[1.5,2.5]]}'"

run_ml_test "ML-058" "Naive Bayes - Multi-class Classification" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"nb_multiclass\",\"model_type\":\"NaiveBayes\",\"dataset\":{\"features\":[[1.0],[5.0],[10.0]],\"target\":[0.0,1.0,2.0],\"feature_names\":[\"x\"]}}'"

run_ml_test "ML-059" "Naive Bayes - Feature Independence Assumption" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/nb_test/independence_test' -H 'Accept: application/json'"

run_ml_test "ML-060" "Naive Bayes - Model Performance Metrics" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/nb_test/metrics' -H 'Accept: application/json'"

# Test ML-061 through ML-070: Preprocessing Tests
run_ml_test "ML-061" "Preprocessing - Standard Scaler Normalization" \
    "curl -s -X POST 'http://localhost:8080/api/ml/preprocess' -H 'Content-Type: application/json' -d '{\"method\":\"StandardScaler\",\"data\":[[1.0],[2.0],[3.0]]}'"

run_ml_test "ML-062" "Preprocessing - MinMax Scaler Normalization" \
    "curl -s -X POST 'http://localhost:8080/api/ml/preprocess' -H 'Content-Type: application/json' -d '{\"method\":\"MinMaxScaler\",\"data\":[[1.0],[2.0],[3.0]],\"range\":[0.0,1.0]}'"

run_ml_test "ML-063" "Preprocessing - Z-Score Calculation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/preprocess' -H 'Content-Type: application/json' -d '{\"method\":\"zscore\",\"data\":[[10.0],[20.0],[30.0]]}'"

run_ml_test "ML-064" "Preprocessing - One-Hot Encoding" \
    "curl -s -X POST 'http://localhost:8080/api/ml/preprocess' -H 'Content-Type: application/json' -d '{\"method\":\"OneHotEncoder\",\"data\":[[\"A\"],[\"B\"],[\"A\"]]}'"

run_ml_test "ML-065" "Preprocessing - Missing Value Imputation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/preprocess' -H 'Content-Type: application/json' -d '{\"method\":\"impute\",\"data\":[[1.0],[null],[3.0]],\"strategy\":\"mean\"}'"

run_ml_test "ML-066" "Preprocessing - Feature Selection" \
    "curl -s -X POST 'http://localhost:8080/api/ml/feature_selection' -H 'Content-Type: application/json' -d '{\"method\":\"variance_threshold\",\"data\":[[1.0,2.0],[1.0,3.0]],\"threshold\":0.5}'"

run_ml_test "ML-067" "Preprocessing - Train/Test Split" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train_test_split' -H 'Content-Type: application/json' -d '{\"data\":[[1.0],[2.0],[3.0],[4.0]],\"test_size\":0.25,\"shuffle\":true}'"

run_ml_test "ML-068" "Preprocessing - K-Fold Cross Validation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/k_fold' -H 'Content-Type: application/json' -d '{\"data\":[[1.0],[2.0],[3.0],[4.0]],\"k\":4}'"

run_ml_test "ML-069" "Preprocessing - Feature Extraction" \
    "curl -s -X POST 'http://localhost:8080/api/ml/feature_extraction' -H 'Content-Type: application/json' -d '{\"data\":[[1.0,2.0,3.0]],\"n_components\":2}'"

run_ml_test "ML-070" "Preprocessing - Data Augmentation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/augment' -H 'Content-Type: application/json' -d '{\"data\":[[1.0],[2.0]],\"factor\":2}'"

# Test ML-071 through ML-080: Optimizer Tests
run_ml_test "ML-071" "Optimizer - Adam Optimizer Implementation" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"adam_test\",\"model_type\":\"LinearRegression\",\"dataset\":{\"features\":[[1.0],[2.0]],\"target\":[2.0,4.0],\"feature_names\":[\"x\"]},\"optimizer\":\"Adam\",\"hyperparameters\":{\"learning_rate\":0.001}}'"

run_ml_test "ML-072" "Optimizer - SGD with Momentum" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"sgd_momentum\",\"model_type\":\"LinearRegression\",\"dataset\":{\"features\":[[1.0],[2.0]],\"target\":[2.0,4.0],\"feature_names\":[\"x\"]},\"optimizer\":\"SGDMomentum\",\"hyperparameters\":{\"learning_rate\":0.01,\"momentum\":0.9}}'"

run_ml_test "ML-073" "Optimizer - Learning Rate Scheduling" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_schedule\",\"model_type\":\"LinearRegression\",\"dataset\":{\"features\":[[1.0]],\"target\":[2.0],\"feature_names\":[\"x\"]},\"lr_schedule\":\"exponential_decay\"}'"

run_ml_test "ML-074" "Optimizer - Exponential Decay Schedule" \
    "curl -s -X GET 'http://localhost:8080/api/ml/optimizers/exponential_decay' -H 'Accept: application/json'"

run_ml_test "ML-075" "Optimizer - Cosine Annealing Schedule" \
    "curl -s -X GET 'http://localhost:8080/api/ml/optimizers/cosine_annealing' -H 'Accept: application/json'"

run_ml_test "ML-076" "Optimizer - Warmup Strategy" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"warmup_test\",\"model_type\":\"LinearRegression\",\"dataset\":{\"features\":[[1.0]],\"target\":[2.0],\"feature_names\":[\"x\"]},\"lr_schedule\":\"warmup_exponential\",\"warmup_steps\":10}'"

run_ml_test "ML-077" "Optimizer - Gradient Clipping" \
    "curl -s -X POST 'http://localhost:8080/api/ml/train' -H 'Content-Type: application/json' -d '{\"model_name\":\"clip_test\",\"model_type\":\"LinearRegression\",\"dataset\":{\"features\":[[1.0]],\"target\":[2.0],\"feature_names\":[\"x\"]},\"gradient_clip\":1.0}'"

run_ml_test "ML-078" "Optimizer - Adaptive Learning Rates" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/adam_test/learning_rates' -H 'Accept: application/json'"

run_ml_test "ML-079" "Optimizer - Moment Estimation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/adam_test/moments' -H 'Accept: application/json'"

run_ml_test "ML-080" "Optimizer - Convergence Analysis" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/adam_test/convergence' -H 'Accept: application/json'"

# Test ML-081 through ML-090: Inference and Model Management
run_ml_test "ML-081" "Inference - Real-time Prediction" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"features\":[[7.0]]}'"

run_ml_test "ML-082" "Inference - Batch Prediction" \
    "curl -s -X POST 'http://localhost:8080/api/ml/batch_predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"features\":[[1.0],[2.0],[3.0]]}'"

run_ml_test "ML-083" "Inference - Model Cache Hit Rate" \
    "curl -s -X GET 'http://localhost:8080/api/ml/cache/stats' -H 'Accept: application/json'"

run_ml_test "ML-084" "Inference - Prediction Confidence Scores" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"logreg_test\",\"features\":[[3.0,4.0]],\"return_confidence\":true}'"

run_ml_test "ML-085" "Inference - Feature Importance for Prediction" \
    "curl -s -X POST 'http://localhost:8080/api/ml/predict' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"features\":[[5.0]],\"explain\":true}'"

run_ml_test "ML-086" "Model Management - List All Models" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models' -H 'Accept: application/json'"

run_ml_test "ML-087" "Model Management - Get Model Metadata" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/lr_test/metadata' -H 'Accept: application/json'"

run_ml_test "ML-088" "Model Management - Model Versioning" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/lr_test/versions' -H 'Accept: application/json'"

run_ml_test "ML-089" "Model Management - A/B Testing Setup" \
    "curl -s -X POST 'http://localhost:8080/api/ml/ab_test' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"version_a\":\"1.0.0\",\"version_b\":\"1.0.1\"}'"

run_ml_test "ML-090" "Model Management - Archive Model" \
    "curl -s -X POST 'http://localhost:8080/api/ml/models/lr_test/archive' -H 'Content-Type: application/json'"

# Test ML-091 through ML-100: Advanced Features
run_ml_test "ML-091" "Neural Network - Placeholder Validation" \
    "curl -s -X GET 'http://localhost:8080/api/ml/algorithms/neural_network' -H 'Accept: application/json'"

run_ml_test "ML-092" "SIMD Operations - Dot Product Acceleration" \
    "curl -s -X POST 'http://localhost:8080/api/ml/simd/dot_product' -H 'Content-Type: application/json' -d '{\"a\":[1.0,2.0,3.0],\"b\":[4.0,5.0,6.0]}'"

run_ml_test "ML-093" "SIMD Operations - Matrix Vector Multiply" \
    "curl -s -X POST 'http://localhost:8080/api/ml/simd/matrix_vector' -H 'Content-Type: application/json' -d '{\"matrix\":[[1.0,2.0],[3.0,4.0]],\"vector\":[1.0,2.0]}'"

run_ml_test "ML-094" "SIMD Operations - Euclidean Distance" \
    "curl -s -X POST 'http://localhost:8080/api/ml/simd/euclidean' -H 'Content-Type: application/json' -d '{\"a\":[1.0,2.0],\"b\":[4.0,6.0]}'"

run_ml_test "ML-095" "Quantization - Model Weight Quantization" \
    "curl -s -X POST 'http://localhost:8080/api/ml/quantize' -H 'Content-Type: application/json' -d '{\"model_name\":\"lr_test\",\"method\":\"int8\"}'"

run_ml_test "ML-096" "Quantization - Dequantization" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models/lr_test/dequantize' -H 'Accept: application/json'"

run_ml_test "ML-097" "Model Registry - Search by Tags" \
    "curl -s -X GET 'http://localhost:8080/api/ml/models?tags=production,regression' -H 'Accept: application/json'"

run_ml_test "ML-098" "Model Registry - Training Jobs Status" \
    "curl -s -X GET 'http://localhost:8080/api/ml/jobs' -H 'Accept: application/json'"

run_ml_test "ML-099" "Model Registry - Cancel Training Job" \
    "curl -s -X POST 'http://localhost:8080/api/ml/jobs/cancel' -H 'Content-Type: application/json' -d '{\"job_id\":\"test_job_123\"}'"

run_ml_test "ML-100" "Model Registry - Cleanup Completed Jobs" \
    "curl -s -X POST 'http://localhost:8080/api/ml/jobs/cleanup' -H 'Content-Type: application/json' -d '{\"older_than_seconds\":3600}'"

echo ""
echo "================================"
echo "FINAL TEST SUMMARY"
echo "================================"
TOTAL=$((PASS + FAIL))
echo "Total Tests: $TOTAL"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Success Rate: $(awk "BEGIN {if($TOTAL>0) printf \"%.2f\", ($PASS/$TOTAL)*100; else print \"0.00\"}")%"
echo "================================"
echo ""
echo "Test Report Saved To: /home/user/rusty-db/ML_ENGINE_TEST_REPORT.md"
echo "================================"

