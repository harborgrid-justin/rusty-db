#!/bin/bash

# Memory Module Testing Script
echo "========================================"
echo "MEMORY MODULE COMPREHENSIVE TEST SUITE"
echo "========================================"
echo ""

# Test counter
TEST_NUM=1

# Test function
run_test() {
    local test_id=$1
    local description=$2
    local curl_cmd=$3
    
    echo "[$test_id] $description"
    echo "Command: $curl_cmd"
    echo "Response:"
    eval "$curl_cmd"
    echo ""
    echo "Status: Executed"
    echo "----------------------------------------"
    echo ""
}

# SECTION 1: SYSTEM HEALTH & METRICS TESTS
echo "=== SECTION 1: SYSTEM HEALTH & METRICS ==="
echo ""

run_test "MEMORY-001" "Test system health endpoint" \
    "curl -s http://localhost:8080/api/v1/admin/health | python3 -m json.tool"

run_test "MEMORY-002" "Test general metrics endpoint" \
    "curl -s http://localhost:8080/api/v1/metrics | python3 -m json.tool"

run_test "MEMORY-003" "Test Prometheus metrics format" \
    "curl -s http://localhost:8080/api/v1/metrics/prometheus"

run_test "MEMORY-004" "Test performance statistics" \
    "curl -s http://localhost:8080/api/v1/stats/performance | python3 -m json.tool"

run_test "MEMORY-005" "Test session statistics" \
    "curl -s http://localhost:8080/api/v1/stats/sessions | python3 -m json.tool"

run_test "MEMORY-006" "Test query statistics" \
    "curl -s http://localhost:8080/api/v1/stats/queries | python3 -m json.tool"

# SECTION 2: GRAPHQL SCHEMA INTROSPECTION
echo "=== SECTION 2: GRAPHQL SCHEMA TESTS ==="
echo ""

run_test "MEMORY-007" "Test GraphQL schema introspection" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __schema { types { name } } }\"}' | python3 -m json.tool | head -100"

run_test "MEMORY-008" "Test GraphQL query type fields" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __type(name: \\\"QueryRoot\\\") { fields { name description } } }\"}' | python3 -m json.tool"

# SECTION 3: CONNECTION & SESSION TESTS
echo "=== SECTION 3: CONNECTION & SESSION MANAGEMENT ==="
echo ""

run_test "MEMORY-009" "Test active connections listing" \
    "curl -s http://localhost:8080/api/v1/connections | python3 -m json.tool"

run_test "MEMORY-010" "Test active sessions listing" \
    "curl -s http://localhost:8080/api/v1/sessions | python3 -m json.tool"

# SECTION 4: POOL MANAGEMENT TESTS
echo "=== SECTION 4: MEMORY POOL MANAGEMENT ==="
echo ""

run_test "MEMORY-011" "Test connection pools listing" \
    "curl -s http://localhost:8080/api/v1/pools | python3 -m json.tool"

run_test "MEMORY-012" "Test default pool information" \
    "curl -s http://localhost:8080/api/v1/pools/default | python3 -m json.tool"

run_test "MEMORY-013" "Test default pool statistics" \
    "curl -s http://localhost:8080/api/v1/pools/default/stats | python3 -m json.tool"

# SECTION 5: DATABASE OPERATIONS (Memory Stress Tests)
echo "=== SECTION 5: DATABASE OPERATIONS (MEMORY STRESS) ==="
echo ""

run_test "MEMORY-014" "Test simple SELECT query (buffer pool usage)" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT 1 as test\"}' | python3 -m json.tool"

run_test "MEMORY-015" "Test table creation (slab allocator)" \
    "curl -s -X POST http://localhost:8080/api/v1/tables/test_memory_table -H 'Content-Type: application/json' -d '{\"columns\": [{\"name\": \"id\", \"type\": \"INTEGER\"}, {\"name\": \"data\", \"type\": \"VARCHAR\"}]}' | python3 -m json.tool"

run_test "MEMORY-016" "Test batch query execution (arena allocator)" \
    "curl -s -X POST http://localhost:8080/api/v1/batch -H 'Content-Type: application/json' -d '{\"queries\": [{\"sql\": \"SELECT 1\"}, {\"sql\": \"SELECT 2\"}, {\"sql\": \"SELECT 3\"}]}' | python3 -m json.tool"

run_test "MEMORY-017" "Test transaction begin (context creation)" \
    "curl -s -X POST http://localhost:8080/api/v1/transactions -H 'Content-Type: application/json' -d '{}' | python3 -m json.tool"

# SECTION 6: GRAPHQL DATABASE QUERIES
echo "=== SECTION 6: GRAPHQL DATABASE QUERIES ==="
echo ""

run_test "MEMORY-018" "Test GraphQL schemas query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ schemas { name } }\"}' | python3 -m json.tool"

run_test "MEMORY-019" "Test GraphQL tables query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ tables(limit: 10) { name rowCount } }\"}' | python3 -m json.tool"

run_test "MEMORY-020" "Test GraphQL execute SQL" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ executeSql(sql: \\\"SELECT 1 as num\\\") { ... on QuerySuccess { totalCount executionTimeMs } } }\"}' | python3 -m json.tool"

# SECTION 7: LARGE QUERY TESTS (Large Object Allocator)
echo "=== SECTION 7: LARGE QUERY TESTS ==="
echo ""

run_test "MEMORY-021" "Test large result set query" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM (VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10)) AS t(n)\"}' | python3 -m json.tool"

run_test "MEMORY-022" "Test complex JOIN query (memory pressure)" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT a.n, b.n FROM (VALUES (1), (2), (3)) AS a(n) CROSS JOIN (VALUES (1), (2), (3)) AS b(n)\"}' | python3 -m json.tool"

# SECTION 8: CONCURRENT OPERATIONS
echo "=== SECTION 8: CONCURRENT OPERATIONS (MEMORY CONTENTION) ==="
echo ""

run_test "MEMORY-023" "Test concurrent query 1" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT 100 as concurrent_test_1\"}' | python3 -m json.tool"

run_test "MEMORY-024" "Test concurrent query 2" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT 200 as concurrent_test_2\"}' | python3 -m json.tool"

run_test "MEMORY-025" "Test concurrent query 3" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT 300 as concurrent_test_3\"}' | python3 -m json.tool"

# SECTION 9: TRANSACTION TESTS (Memory Context Management)
echo "=== SECTION 9: TRANSACTION MEMORY CONTEXTS ==="
echo ""

run_test "MEMORY-026" "Test transaction lifecycle - begin" \
    "curl -s -X POST http://localhost:8080/api/v1/transactions -H 'Content-Type: application/json' -d '{}' | python3 -m json.tool | tee /tmp/txn_response.json"

# Extract transaction ID if available
TXN_ID=$(cat /tmp/txn_response.json 2>/dev/null | python3 -c "import json, sys; data=json.load(sys.stdin); print(data.get('transaction_id', 'test_txn_1'))" 2>/dev/null || echo "test_txn_1")

run_test "MEMORY-027" "Test transaction commit (memory cleanup)" \
    "curl -s -X POST http://localhost:8080/api/v1/transactions/${TXN_ID}/commit -H 'Content-Type: application/json' -d '{}' | python3 -m json.tool"

run_test "MEMORY-028" "Test transaction rollback (memory release)" \
    "curl -s -X POST http://localhost:8080/api/v1/transactions/test_txn_2/rollback -H 'Content-Type: application/json' -d '{}' | python3 -m json.tool"

# SECTION 10: GRAPHQL ADVANCED QUERIES
echo "=== SECTION 10: GRAPHQL ADVANCED OPERATIONS ==="
echo ""

run_test "MEMORY-029" "Test GraphQL aggregation query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ count(table: \\\"test_table\\\") }\"}' | python3 -m json.tool"

run_test "MEMORY-030" "Test GraphQL explain query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ explain(table: \\\"test_table\\\") { plan estimatedCost } }\"}' | python3 -m json.tool"

# SECTION 11: CLUSTER & REPLICATION (Distributed Memory)
echo "=== SECTION 11: CLUSTER OPERATIONS ==="
echo ""

run_test "MEMORY-031" "Test cluster nodes listing" \
    "curl -s http://localhost:8080/api/v1/cluster/nodes | python3 -m json.tool"

run_test "MEMORY-032" "Test cluster topology" \
    "curl -s http://localhost:8080/api/v1/cluster/topology | python3 -m json.tool"

run_test "MEMORY-033" "Test replication status" \
    "curl -s http://localhost:8080/api/v1/cluster/replication | python3 -m json.tool"

run_test "MEMORY-034" "Test cluster configuration" \
    "curl -s http://localhost:8080/api/v1/cluster/config | python3 -m json.tool"

# SECTION 12: ALERT & MONITORING
echo "=== SECTION 12: ALERT & MONITORING SYSTEM ==="
echo ""

run_test "MEMORY-035" "Test alerts endpoint" \
    "curl -s http://localhost:8080/api/v1/alerts | python3 -m json.tool"

run_test "MEMORY-036" "Test logs endpoint" \
    "curl -s http://localhost:8080/api/v1/logs | python3 -m json.tool"

# SECTION 13: STRESS TESTS
echo "=== SECTION 13: MEMORY STRESS TESTS ==="
echo ""

run_test "MEMORY-037" "Stress test - Multiple sequential queries" \
    "for i in {1..5}; do curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT '${i}' as iteration\"}'; done | python3 -m json.tool"

run_test "MEMORY-038" "Stress test - Large string allocation" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT REPEAT(\\\"A\\\", 1000) as large_string\"}' | python3 -m json.tool"

run_test "MEMORY-039" "Stress test - Multiple table operations" \
    "curl -s -X POST http://localhost:8080/api/v1/batch -H 'Content-Type: application/json' -d '{\"queries\": [{\"sql\": \"SELECT 1\"}, {\"sql\": \"SELECT 2\"}, {\"sql\": \"SELECT 3\"}, {\"sql\": \"SELECT 4\"}, {\"sql\": \"SELECT 5\"}]}' | python3 -m json.tool"

run_test "MEMORY-040" "Test configuration retrieval (global memory settings)" \
    "curl -s http://localhost:8080/api/v1/admin/config | python3 -m json.tool"

echo ""
echo "========================================"
echo "MEMORY TEST SUITE COMPLETE"
echo "Total Tests: 40"
echo "========================================"
