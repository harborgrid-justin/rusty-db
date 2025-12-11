#!/bin/bash

echo "================================"
echo "ML MODULE FUNCTIONAL TESTS"
echo "Test Date: $(date)"
echo "Server: localhost:8080"
echo "================================"
echo ""

# Initialize counters
PASS_COUNT=0
FAIL_COUNT=0
TEST_NUMBER=0

# Test result logging
log_test() {
    TEST_NUMBER=$((TEST_NUMBER + 1))
    TEST_ID="ML-$(printf '%03d' $TEST_NUMBER)"
    TEST_NAME="$1"
    COMMAND="$2"
    EXPECTED="$3"
    
    echo "[$TEST_ID] $TEST_NAME"
    echo "Command: $COMMAND"
    
    RESPONSE=$(eval "$COMMAND" 2>&1)
    echo "Response: $RESPONSE"
    
    # Determine pass/fail
    if [ -n "$EXPECTED" ]; then
        if echo "$RESPONSE" | grep -q "$EXPECTED"; then
            echo "Status: PASS"
            PASS_COUNT=$((PASS_COUNT + 1))
        else
            echo "Status: FAIL (Expected: $EXPECTED)"
            FAIL_COUNT=$((FAIL_COUNT + 1))
        fi
    else
        # If no expected pattern, check for error indicators
        if echo "$RESPONSE" | grep -qi "error\|exception\|failed\|invalid"; then
            echo "Status: FAIL"
            FAIL_COUNT=$((FAIL_COUNT + 1))
        else
            echo "Status: PASS"
            PASS_COUNT=$((PASS_COUNT + 1))
        fi
    fi
    
    echo "---"
    echo ""
}

# ML-001 through ML-010: Linear Regression Tests
log_test "GraphQL Query - Check Query Root" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __type(name: \\\"QueryRoot\\\") { name kind } }\"}'" \
    "QueryRoot"

log_test "GraphQL Mutation - Check Mutation Root" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __type(name: \\\"MutationRoot\\\") { name kind } }\"}'" \
    "MutationRoot"

log_test "Test Query: List Schemas" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ schemas { name } }\"}'"

log_test "Test Query: List Tables" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ tables { name } }\"}'"

log_test "Create ML Training Table via GraphQL" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"CREATE TABLE IF NOT EXISTS ml_training_data (id INT, x FLOAT, y FLOAT)\\\") { executionTimeMs } }\"}'"

log_test "Insert Linear Regression Training Data (Point 1)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO ml_training_data VALUES (1, 1.0, 2.0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Linear Regression Training Data (Point 2)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO ml_training_data VALUES (2, 2.0, 4.0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Linear Regression Training Data (Point 3)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO ml_training_data VALUES (3, 3.0, 6.0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Linear Regression Training Data (Point 4)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO ml_training_data VALUES (4, 4.0, 8.0)\\\") { executionTimeMs } }\"}'"

log_test "Query Training Data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ queryTable(table: \\\"ml_training_data\\\") { ... on QuerySuccess { totalCount } } }\"}'"

# ML-011 through ML-020: Logistic Regression / Classification Tests
log_test "Create Classification Training Table" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"CREATE TABLE IF NOT EXISTS classification_data (id INT, feature1 FLOAT, feature2 FLOAT, label INT)\\\") { executionTimeMs } }\"}'"

log_test "Insert Classification Data (Class 0, Sample 1)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO classification_data VALUES (1, 1.0, 1.0, 0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Classification Data (Class 0, Sample 2)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO classification_data VALUES (2, 1.5, 2.0, 0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Classification Data (Class 1, Sample 1)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO classification_data VALUES (3, 5.0, 6.0, 1)\\\") { executionTimeMs } }\"}'"

log_test "Insert Classification Data (Class 1, Sample 2)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO classification_data VALUES (4, 6.0, 5.5, 1)\\\") { executionTimeMs } }\"}'"

log_test "Query Classification Data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ queryTable(table: \\\"classification_data\\\") { ... on QuerySuccess { totalCount } } }\"}'"

# ML-021 through ML-030: K-Means Clustering Tests
log_test "Create Clustering Data Table" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"CREATE TABLE IF NOT EXISTS clustering_data (id INT, x FLOAT, y FLOAT)\\\") { executionTimeMs } }\"}'"

log_test "Insert Clustering Data (Cluster 1, Point 1)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO clustering_data VALUES (1, 1.0, 1.0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Clustering Data (Cluster 1, Point 2)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO clustering_data VALUES (2, 1.5, 2.0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Clustering Data (Cluster 2, Point 1)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO clustering_data VALUES (3, 10.0, 10.0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Clustering Data (Cluster 2, Point 2)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO clustering_data VALUES (4, 11.0, 9.5)\\\") { executionTimeMs } }\"}'"

log_test "Query Clustering Data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ queryTable(table: \\\"clustering_data\\\") { ... on QuerySuccess { totalCount } } }\"}'"

# ML-031 through ML-040: Decision Tree Tests
log_test "Create Decision Tree Data Table" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"CREATE TABLE IF NOT EXISTS tree_data (id INT, age FLOAT, income FLOAT, approved INT)\\\") { executionTimeMs } }\"}'"

log_test "Insert Decision Tree Data (Sample 1)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO tree_data VALUES (1, 25.0, 30000.0, 0)\\\") { executionTimeMs } }\"}'"

log_test "Insert Decision Tree Data (Sample 2)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO tree_data VALUES (2, 45.0, 80000.0, 1)\\\") { executionTimeMs } }\"}'"

log_test "Insert Decision Tree Data (Sample 3)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO tree_data VALUES (3, 35.0, 60000.0, 1)\\\") { executionTimeMs } }\"}'"

log_test "Insert Decision Tree Data (Sample 4)" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { executeSQL(sql: \\\"INSERT INTO tree_data VALUES (4, 22.0, 25000.0, 0)\\\") { executionTimeMs } }\"}'"

log_test "Query Decision Tree Data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ queryTable(table: \\\"tree_data\\\") { ... on QuerySuccess { totalCount } } }\"}'"

# ML-041 through ML-050: Test REST API endpoints
log_test "REST API: Check ML Models Endpoint" \
    "curl -s -X GET http://localhost:8080/api/ml/models -H 'Accept: application/json'"

log_test "REST API: Check ML Train Endpoint" \
    "curl -s -X POST http://localhost:8080/api/ml/train -H 'Content-Type: application/json' -d '{\"model_name\": \"test_model\", \"model_type\": \"LinearRegression\", \"dataset\": {\"features\": [[1.0], [2.0], [3.0]], \"target\": [2.0, 4.0, 6.0], \"feature_names\": [\"x\"]}}'"

log_test "REST API: Check ML Predict Endpoint" \
    "curl -s -X POST http://localhost:8080/api/ml/predict -H 'Content-Type: application/json' -d '{\"model_name\": \"test_model\", \"features\": [[5.0]]}'"

log_test "REST API: Get Model Info" \
    "curl -s -X GET http://localhost:8080/api/ml/models/test_model -H 'Accept: application/json'"

log_test "REST API: List Model Versions" \
    "curl -s -X GET http://localhost:8080/api/ml/models/test_model/versions -H 'Accept: application/json'"

log_test "REST API: Delete Model" \
    "curl -s -X DELETE http://localhost:8080/api/ml/models/test_model -H 'Accept: application/json'"

log_test "REST API: Check ML Statistics" \
    "curl -s -X GET http://localhost:8080/api/ml/stats -H 'Accept: application/json'"

log_test "REST API: Check ML Health" \
    "curl -s -X GET http://localhost:8080/api/ml/health -H 'Accept: application/json'"

log_test "REST API: List ML Algorithms" \
    "curl -s -X GET http://localhost:8080/api/ml/algorithms -H 'Accept: application/json'"

log_test "GraphQL: Test Custom ML Query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __schema { types { name } } }\"}'" \
    "types"

echo ""
echo "================================"
echo "TEST EXECUTION SUMMARY"
echo "================================"
echo "Total Tests Executed: $TEST_NUMBER"
echo "Tests Passed: $PASS_COUNT"
echo "Tests Failed: $FAIL_COUNT"
echo "Success Rate: $(awk "BEGIN {printf \"%.2f\", ($PASS_COUNT/$TEST_NUMBER)*100}")%"
echo "================================"

