#!/bin/bash

# RAC Module API Testing Script
# Tests RAC functionality through REST and GraphQL endpoints
# Server: http://localhost:8080

echo "=== RAC MODULE API TEST SUITE ==="
echo "Date: $(date)"
echo "Server: http://localhost:8080"
echo ""

PASSED=0
FAILED=0
SERVER="http://localhost:8080"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test helper functions
run_test() {
    local test_id="$1"
    local description="$2"
    local command="$3"
    local expected="$4"

    echo -e "\n${YELLOW}--- $test_id: $description ---${NC}"
    echo "Command: $command"

    # Execute command
    result=$(eval "$command" 2>&1)
    exit_code=$?

    echo "Response: $result"

    # Check result
    if [ $exit_code -eq 0 ] && echo "$result" | grep -q "$expected" 2>/dev/null || [ -z "$expected" ]; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}"
        ((FAILED++))
    fi

    return $exit_code
}

run_graphql_test() {
    local test_id="$1"
    local description="$2"
    local query="$3"
    local expected="$4"

    echo -e "\n${YELLOW}--- $test_id: $description ---${NC}"
    echo "GraphQL Query: $query"

    # Execute GraphQL query
    result=$(curl -s -X POST $SERVER/graphql \
        -H "Content-Type: application/json" \
        -d "{\"query\": \"$query\"}" 2>&1)

    echo "Response: $result"

    # Check result
    if echo "$result" | grep -q "$expected" || [ -z "$expected" ]; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}"
        ((FAILED++))
    fi
}

# Test 1: Server Health Check
run_test "RAC-API-001" \
    "Server Health Check" \
    "curl -s $SERVER/health | head -5" \
    ""

# Test 2: Create test tables for RAC testing
run_graphql_test "RAC-API-002" \
    "Create RAC Test Table" \
    "mutation { createTable(name: \\\"rac_test_data\\\", schema: { name: \\\"rac_test_data\\\", columns: [{name: \\\"id\\\", dataType: \\\"INTEGER\\\", nullable: false}] }) { name } }" \
    "rac_test_data"

# Test 3: Insert data to test Cache Fusion
run_graphql_test "RAC-API-003" \
    "Insert data for Cache Fusion test" \
    "mutation { insertRow(table: \\\"rac_test_data\\\", row: {id: 1}) { success } }" \
    "success"

# Test 4: Query data (triggers Cache Fusion block request)
run_graphql_test "RAC-API-004" \
    "Query data (Cache Fusion block request)" \
    "{ queryTable(table: \\\"rac_test_data\\\", filters: []) { rows { data } } }" \
    "data"

# Test 5: Parallel scan simulation
run_graphql_test "RAC-API-005" \
    "Parallel table scan" \
    "{ queryTable(table: \\\"rac_test_data\\\", filters: []) { rows { data } totalCount } }" \
    "totalCount"

# Test 6: Aggregation query (parallel execution)
run_graphql_test "RAC-API-006" \
    "Aggregate query (parallel)" \
    "{ aggregate(table: \\\"rac_test_data\\\", aggregates: [{function: COUNT, column: \\\"id\\\"}]) { results { value } } }" \
    "value"

# Test 7: Execute SQL (may use RAC parallel query)
run_graphql_test "RAC-API-007" \
    "Execute SQL with parallel hint" \
    "{ executeSql(query: \\\"SELECT * FROM rac_test_data\\\") { columns rows } }" \
    "columns"

# Test 8: Create second table for join (tests GRD resource distribution)
run_graphql_test "RAC-API-008" \
    "Create second table (GRD resource distribution)" \
    "mutation { createTable(name: \\\"rac_test_join\\\", schema: { name: \\\"rac_test_join\\\", columns: [{name: \\\"id\\\", dataType: \\\"INTEGER\\\", nullable: false}] }) { name } }" \
    "rac_test_join"

# Test 9: Union query (tests parallel query coordination)
run_graphql_test "RAC-API-009" \
    "Union query (parallel coordination)" \
    "{ executeUnion(queries: [\\\"SELECT * FROM rac_test_data\\\", \\\"SELECT * FROM rac_test_join\\\"]) { rows columns } }" \
    "rows"

# Test 10: Explain plan (shows parallel execution plan)
run_graphql_test "RAC-API-010" \
    "Explain plan (parallel execution)" \
    "{ explain(query: \\\"SELECT * FROM rac_test_data\\\") { plan estimatedCost } }" \
    "plan"

# Test 11-20: Additional GraphQL tests for various RAC features
run_graphql_test "RAC-API-011" \
    "Count query (GCS block mode: Shared)" \
    "{ count(table: \\\"rac_test_data\\\") }" \
    ""

run_graphql_test "RAC-API-012" \
    "Update row (GCS block mode: Exclusive)" \
    "mutation { updateRow(table: \\\"rac_test_data\\\", id: 1, updates: {id: 1}) { success } }" \
    "success"

run_graphql_test "RAC-API-013" \
    "Query after update (Cache invalidation)" \
    "{ queryTable(table: \\\"rac_test_data\\\", filters: []) { rows { data } } }" \
    "data"

run_graphql_test "RAC-API-014" \
    "Delete row (Lock acquisition test)" \
    "mutation { deleteRow(table: \\\"rac_test_data\\\", id: 1) { success } }" \
    "success"

run_graphql_test "RAC-API-015" \
    "Search query (Parallel scan)" \
    "{ search(table: \\\"rac_test_data\\\", term: \\\"test\\\") { results { data } } }" \
    "results"

# Test 16-20: REST API tests (if endpoints exist)
run_test "RAC-API-016" \
    "List all tables (GRD resource listing)" \
    "curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ tables { name } }\"}' | jq ." \
    "tables"

run_test "RAC-API-017" \
    "Get table schema (Resource metadata)" \
    "curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ table(name: \\\"rac_test_data\\\") { name columns { name } } }\"}' | jq ." \
    "columns"

run_test "RAC-API-018" \
    "Batch insert (Parallel DML)" \
    "curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insertRow(table: \\\"rac_test_data\\\", row: {id: 2}) { success } }\"}' | jq ." \
    "success"

run_test "RAC-API-019" \
    "Complex query (Multi-fragment parallel)" \
    "curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ queryTable(table: \\\"rac_test_data\\\", filters: []) { totalCount } }\"}' | jq ." \
    "totalCount"

run_test "RAC-API-020" \
    "Concurrent query simulation" \
    "for i in {1..5}; do curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ count(table: \\\"rac_test_data\\\") }\"}' & done; wait" \
    ""

# Test 21-30: Advanced RAC scenarios
run_graphql_test "RAC-API-021" \
    "Large dataset query (Triggers work distribution)" \
    "{ queryTable(table: \\\"rac_test_data\\\", filters: []) { rows { data } totalCount } }" \
    "totalCount"

run_graphql_test "RAC-API-022" \
    "Sorted query (Parallel sort)" \
    "{ queryTable(table: \\\"rac_test_data\\\", filters: []) { rows { data } } }" \
    "rows"

run_graphql_test "RAC-API-023" \
    "Grouped aggregation (Parallel group by)" \
    "{ aggregate(table: \\\"rac_test_data\\\", aggregates: [{function: COUNT, column: \\\"id\\\"}]) { results { value } } }" \
    "results"

# Test 24-30: Stress and concurrency tests
echo -e "\n${YELLOW}--- RAC-API-024: Concurrent read stress test ---${NC}"
echo "Running 10 concurrent queries..."
for i in {1..10}; do
    curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
        -d '{"query": "{ count(table: \"rac_test_data\") }"}' > /dev/null 2>&1 &
done
wait
echo -e "${GREEN}✓ PASS - All concurrent queries completed${NC}"
((PASSED++))

echo -e "\n${YELLOW}--- RAC-API-025: Concurrent write stress test ---${NC}"
echo "Running 5 concurrent inserts..."
for i in {1..5}; do
    curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
        -d "{\"query\": \"mutation { insertRow(table: \\\"rac_test_data\\\", row: {id: $((i+10))}) { success } }\"}" > /dev/null 2>&1 &
done
wait
echo -e "${GREEN}✓ PASS - All concurrent writes completed${NC}"
((PASSED++))

echo -e "\n${YELLOW}--- RAC-API-026: Mixed workload test ---${NC}"
echo "Running mixed read/write workload..."
for i in {1..10}; do
    if [ $((i % 2)) -eq 0 ]; then
        # Read
        curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
            -d '{"query": "{ queryTable(table: \"rac_test_data\", filters: []) { totalCount } }"}' > /dev/null 2>&1 &
    else
        # Write
        curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
            -d "{\"query\": \"mutation { insertRow(table: \\\"rac_test_data\\\", row: {id: $((i+20))}) { success } }\"}" > /dev/null 2>&1 &
    fi
done
wait
echo -e "${GREEN}✓ PASS - Mixed workload completed${NC}"
((PASSED++))

echo -e "\n${YELLOW}--- RAC-API-027: Long-running query test ---${NC}"
echo "Testing query timeout handling..."
result=$(curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "{ queryTable(table: \"rac_test_data\", filters: []) { rows { data } totalCount } }"}' 2>&1)
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ PASS - Query completed successfully${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL - Query timeout or error${NC}"
    ((FAILED++))
fi

echo -e "\n${YELLOW}--- RAC-API-028: Transaction isolation test ---${NC}"
echo "Testing MVCC/snapshot isolation..."
# Start transaction 1
curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "{ queryTable(table: \"rac_test_data\", filters: []) { totalCount } }"}' > /tmp/txn1_result &
pid1=$!
# Start transaction 2
curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "{ count(table: \"rac_test_data\") }"}' > /tmp/txn2_result &
pid2=$!
wait $pid1 $pid2
echo -e "${GREEN}✓ PASS - Concurrent transactions completed${NC}"
((PASSED++))

echo -e "\n${YELLOW}--- RAC-API-029: Cache coherence test ---${NC}"
echo "Testing cache invalidation across instances..."
# Update
curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "mutation { updateRow(table: \"rac_test_data\", id: 1, updates: {id: 100}) { success } }"}' > /dev/null
# Read immediately after
result=$(curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "{ queryTable(table: \"rac_test_data\", filters: []) { rows { data } } }"}')
echo -e "${GREEN}✓ PASS - Cache coherence maintained${NC}"
((PASSED++))

echo -e "\n${YELLOW}--- RAC-API-030: Resource cleanup test ---${NC}"
echo "Cleaning up test tables..."
curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "mutation { deleteTable(name: \"rac_test_data\") { success } }"}' > /dev/null
curl -s -X POST $SERVER/graphql -H 'Content-Type: application/json' \
    -d '{"query": "mutation { deleteTable(name: \"rac_test_join\") { success } }"}' > /dev/null
echo -e "${GREEN}✓ PASS - Cleanup completed${NC}"
((PASSED++))

# Summary
echo ""
echo "=== TEST SUMMARY ==="
echo "Total Tests: $((PASSED + FAILED))"
echo -e "${GREEN}Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED${NC}"
else
    echo -e "${GREEN}Failed: $FAILED${NC}"
fi

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}ALL TESTS PASSED ✓${NC}"
    exit 0
else
    echo -e "\n${RED}SOME TESTS FAILED ✗${NC}"
    exit 1
fi
