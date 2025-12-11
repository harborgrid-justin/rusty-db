#!/bin/bash

echo "================================"
echo "ML MODULE COMPREHENSIVE TEST SUITE"
echo "================================"
echo ""

# Test counter
TEST_NUM=0
PASS=0
FAIL=0

run_test() {
    TEST_NUM=$((TEST_NUM + 1))
    TEST_ID="ML-$(printf '%03d' $TEST_NUM)"
    echo "[$TEST_ID] $1"
    echo "Command: $2"
    RESPONSE=$(eval "$2" 2>&1)
    echo "Response: $RESPONSE"
    
    if echo "$RESPONSE" | grep -qi "error\|fail\|404"; then
        echo "Status: FAIL"
        FAIL=$((FAIL + 1))
    else
        echo "Status: PASS"
        PASS=$((PASS + 1))
    fi
    echo "---"
    echo ""
}

# ML-001: Test server health
run_test "Server Health Check" \
    "curl -s http://localhost:8080/health"

# ML-002: Test GraphQL endpoint basic connectivity  
run_test "GraphQL Basic Connectivity" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __schema { queryType { name } } }\"}'"

# ML-003: Test API monitoring endpoint
run_test "API Monitoring Endpoint" \
    "curl -s http://localhost:8080/api/monitoring/stats"

# ML-004: Test API status
run_test "API Status Endpoint" \
    "curl -s http://localhost:8080/api/status"

# ML-005: Test available routes
run_test "List API Routes" \
    "curl -s http://localhost:8080/api/"

# ML-006: Test ML model list endpoint (REST)
run_test "List ML Models (REST)" \
    "curl -s http://localhost:8080/api/v1/ml/models"

# ML-007: Test ML train endpoint (REST)
run_test "Train ML Model Endpoint Check (REST)" \
    "curl -s -X OPTIONS http://localhost:8080/api/v1/ml/train"

# ML-008: Test ML predict endpoint (REST)
run_test "Predict ML Model Endpoint Check (REST)" \
    "curl -s -X OPTIONS http://localhost:8080/api/v1/ml/predict"

# ML-009: Try different ML API paths
run_test "ML API Path /ml" \
    "curl -s http://localhost:8080/ml"

# ML-010: Try different ML API paths /ml/models
run_test "ML API Path /ml/models" \
    "curl -s http://localhost:8080/ml/models"

echo "================================"
echo "TEST SUMMARY"
echo "================================"
echo "Total Tests: $TEST_NUM"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "================================"

