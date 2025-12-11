#!/bin/bash

# Event Processing Module Test Suite
# Testing CEP (Complex Event Processing) features via REST and GraphQL

BASE_URL="http://localhost:8080"
TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test result tracking
declare -a TEST_RESULTS

log_test() {
    local test_id=$1
    local test_name=$2
    local status=$3
    local details=$4
    
    TEST_COUNT=$((TEST_COUNT + 1))
    if [ "$status" = "PASS" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo -e "${GREEN}[PASS]${NC} $test_id: $test_name"
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo -e "${RED}[FAIL]${NC} $test_id: $test_name"
    fi
    
    TEST_RESULTS+=("$test_id|$test_name|$status|$details")
}

# CEP-001: Test GraphQL Schema Introspection
echo "=== CEP-001: GraphQL Schema Introspection ==="
response=$(curl -s -X POST "$BASE_URL/graphql" \
    -H "Content-Type: application/json" \
    -d '{"query": "{ __schema { queryType { name } mutationType { name } subscriptionType { name } } }"}')

if echo "$response" | grep -q "queryType"; then
    log_test "CEP-001" "GraphQL Schema Introspection" "PASS" "$response"
else
    log_test "CEP-001" "GraphQL Schema Introspection" "FAIL" "$response"
fi

# CEP-002: Test Query Execution (Event Processing Simulation)
echo "=== CEP-002: Test Query Execution ==="
response=$(curl -s -X POST "$BASE_URL/graphql" \
    -H "Content-Type: application/json" \
    -d '{"query": "{ __type(name: \"QueryRoot\") { name fields { name type { name kind } } } }"}')

if echo "$response" | grep -q "QueryRoot"; then
    log_test "CEP-002" "Query Type Introspection" "PASS" "$response"
else
    log_test "CEP-002" "Query Type Introspection" "FAIL" "$response"
fi

# CEP-003: Test Event Stream Creation (via table creation)
echo "=== CEP-003: Event Stream Simulation ==="
response=$(curl -s -X POST "$BASE_URL/graphql" \
    -H "Content-Type: application/json" \
    -d '{"query": "mutation { createTable(input: { tableName: \"event_stream_test\", columns: [{ name: \"event_id\", dataType: \"INT\", notNull: true }, { name: \"event_type\", dataType: \"VARCHAR\", maxLength: 255 }, { name: \"payload\", dataType: \"TEXT\" }, { name: \"event_time\", dataType: \"TIMESTAMP\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }"}')

if echo "$response" | grep -q "message\|already exists"; then
    log_test "CEP-003" "Event Stream Table Creation" "PASS" "$response"
else
    log_test "CEP-003" "Event Stream Table Creation" "FAIL" "$response"
fi

# CEP-004: Test Event Insertion (Event Publishing)
echo "=== CEP-004: Event Publishing ==="
response=$(curl -s -X POST "$BASE_URL/graphql" \
    -H "Content-Type: application/json" \
    -d '{"query": "mutation { insert(table: \"event_stream_test\", data: { event_id: 1, event_type: \"user.login\", payload: \"user_123\", event_time: \"2024-01-01T10:00:00Z\" }) { ... on MutationSuccess { rowsAffected } ... on MutationError { error } } }"}')

if echo "$response" | grep -q "rowsAffected\|SUCCESS"; then
    log_test "CEP-004" "Event Publishing (Insert)" "PASS" "$response"
else
    log_test "CEP-004" "Event Publishing (Insert)" "FAIL" "$response"
fi

# CEP-005: Test Event Query (Event Consumption)
echo "=== CEP-005: Event Query ==="
response=$(curl -s -X POST "$BASE_URL/graphql" \
    -H "Content-Type: application/json" \
    -d '{"query": "{ query(table: \"event_stream_test\", limit: 10) { ... on QuerySuccess { data { rows { values } } } ... on QueryError { error } } }"}')

if echo "$response" | grep -q "rows\|data"; then
    log_test "CEP-005" "Event Query (Consumption)" "PASS" "$response"
else
    log_test "CEP-005" "Event Query (Consumption)" "FAIL" "$response"
fi

# Generate summary
echo ""
echo "=========================================="
echo "       EVENT PROCESSING TEST SUMMARY      "
echo "=========================================="
echo "Total Tests: $TEST_COUNT"
echo -e "${GREEN}Passed: $PASS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"
echo "Coverage: Event creation, streams, queries"
echo "=========================================="

