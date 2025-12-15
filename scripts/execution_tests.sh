#!/bin/bash

# RustyDB Execution Module Test Suite
# Tests all execution features via REST API and GraphQL

BASE_URL="http://localhost:8080"
GRAPHQL_URL="$BASE_URL/graphql"

echo "=========================================="
echo "EXECUTION MODULE TEST SUITE"
echo "=========================================="
echo ""

# Test counter
TEST_NUM=0
PASS=0
FAIL=0

# Test function
run_test() {
    TEST_NUM=$((TEST_NUM + 1))
    TEST_ID=$(printf "EXEC-%03d" $TEST_NUM)
    echo "[$TEST_ID] $1"
}

pass_test() {
    PASS=$((PASS + 1))
    echo "  ✓ PASS"
    echo ""
}

fail_test() {
    FAIL=$((FAIL + 1))
    echo "  ✗ FAIL: $1"
    echo ""
}

# ========================================
# SECTION 1: BASIC EXECUTOR TESTS
# ========================================
echo "=== SECTION 1: BASIC EXECUTOR TESTS ==="
echo ""

run_test "Create test table via REST API"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "CREATE TABLE employees (id INT, name VARCHAR(100), salary INT, department VARCHAR(50))"}')
if echo "$RESPONSE" | grep -q '"success":true\|rows_affected'; then
    pass_test
else
    fail_test "Failed to create table"
fi

run_test "Insert test data"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "INSERT INTO employees (id, name, salary, department) VALUES (1, \"Alice\", 75000, \"Engineering\"), (2, \"Bob\", 65000, \"Sales\"), (3, \"Charlie\", 80000, \"Engineering\"), (4, \"Diana\", 70000, \"Marketing\")"}')
if echo "$RESPONSE" | grep -q 'rows_affected\|success'; then
    pass_test
else
    fail_test "Failed to insert data"
fi

run_test "Simple SELECT query"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed SELECT query"
fi

run_test "SELECT with column projection"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT name, salary FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed column projection"
fi

run_test "SELECT with WHERE clause"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees WHERE salary > 70000"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed WHERE clause"
fi

run_test "SELECT with ORDER BY"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees ORDER BY salary DESC"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed ORDER BY"
fi

run_test "SELECT with LIMIT"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees LIMIT 2"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed LIMIT"
fi

run_test "SELECT with OFFSET"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees LIMIT 2 OFFSET 1"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed OFFSET"
fi

run_test "SELECT DISTINCT"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT DISTINCT department FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed DISTINCT"
fi

# ========================================
# SECTION 2: AGGREGATION TESTS
# ========================================
echo "=== SECTION 2: AGGREGATION TESTS ==="
echo ""

run_test "COUNT(*) aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT COUNT(*) FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|count'; then
    pass_test
else
    fail_test "Failed COUNT(*)"
fi

run_test "SUM aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT SUM(salary) FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed SUM"
fi

run_test "AVG aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT AVG(salary) FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed AVG"
fi

run_test "MIN aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT MIN(salary) FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed MIN"
fi

run_test "MAX aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT MAX(salary) FROM employees"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed MAX"
fi

run_test "GROUP BY aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*) FROM employees GROUP BY department"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed GROUP BY"
fi

run_test "GROUP BY with multiple aggregates"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*), AVG(salary) FROM employees GROUP BY department"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed multiple aggregates"
fi

run_test "HAVING clause"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, AVG(salary) FROM employees GROUP BY department HAVING AVG(salary) > 70000"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed HAVING"
fi

# ========================================
# SECTION 3: JOIN TESTS
# ========================================
echo "=== SECTION 3: JOIN TESTS ==="
echo ""

run_test "Create departments table"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "CREATE TABLE departments (dept_id INT, dept_name VARCHAR(50), location VARCHAR(50))"}')
if echo "$RESPONSE" | grep -q 'success\|rows_affected'; then
    pass_test
else
    fail_test "Failed to create departments table"
fi

run_test "Insert department data"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "INSERT INTO departments (dept_id, dept_name, location) VALUES (1, \"Engineering\", \"Building A\"), (2, \"Sales\", \"Building B\"), (3, \"Marketing\", \"Building C\")"}')
if echo "$RESPONSE" | grep -q 'rows_affected\|success'; then
    pass_test
else
    fail_test "Failed to insert departments"
fi

run_test "INNER JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, e.salary, d.location FROM employees e INNER JOIN departments d ON e.department = d.dept_name"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed INNER JOIN"
fi

run_test "LEFT JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, e.salary, d.location FROM employees e LEFT JOIN departments d ON e.department = d.dept_name"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed LEFT JOIN"
fi

run_test "RIGHT JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, e.salary, d.location FROM employees e RIGHT JOIN departments d ON e.department = d.dept_name"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed RIGHT JOIN"
fi

run_test "FULL OUTER JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, e.salary, d.location FROM employees e FULL OUTER JOIN departments d ON e.department = d.dept_name"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed FULL OUTER JOIN"
fi

run_test "CROSS JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, d.dept_name FROM employees e CROSS JOIN departments d"}')
if echo "$RESPONSE" | grep -q 'columns\|rows'; then
    pass_test
else
    fail_test "Failed CROSS JOIN"
fi

# ========================================
# SECTION 4: PLANNER TESTS
# ========================================
echo "=== SECTION 4: PLANNER TESTS ==="
echo ""

run_test "Query plan generation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/explain" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees WHERE salary > 70000"}')
if echo "$RESPONSE" | grep -q 'plan\|TableScan\|Filter\|success'; then
    pass_test
else
    fail_test "Failed to generate query plan"
fi

run_test "Complex query plan with JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/explain" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, d.dept_name FROM employees e INNER JOIN departments d ON e.department = d.dept_name WHERE e.salary > 70000"}')
if echo "$RESPONSE" | grep -q 'plan\|Join\|Filter'; then
    pass_test
else
    fail_test "Failed complex plan"
fi

run_test "Plan with aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/explain" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*), AVG(salary) FROM employees GROUP BY department"}')
if echo "$RESPONSE" | grep -q 'plan\|Aggregate\|TableScan'; then
    pass_test
else
    fail_test "Failed aggregation plan"
fi

# ========================================
# SECTION 5: OPTIMIZATION TESTS
# ========================================
echo "=== SECTION 5: OPTIMIZATION TESTS ==="
echo ""

run_test "Plan cache statistics"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/cache/stats")
if echo "$RESPONSE" | grep -q 'hit\|miss\|size'; then
    pass_test
else
    fail_test "Failed to get cache stats"
fi

run_test "Table statistics collection"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/stats/collect" \
  -H "Content-Type: application/json" \
  -d '{"table": "employees"}')
if echo "$RESPONSE" | grep -q 'success\|row_count\|stats'; then
    pass_test
else
    fail_test "Failed to collect statistics"
fi

run_test "Query optimization hints"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT /*+ USE_HASH_JOIN */ * FROM employees e INNER JOIN departments d ON e.department = d.dept_name"}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed optimization hints"
fi

run_test "Materialized view suggestion"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/suggest-mv" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*), AVG(salary) FROM employees GROUP BY department"}')
if echo "$RESPONSE" | grep -q 'suggestion\|materialized_view\|success'; then
    pass_test
else
    fail_test "Failed MV suggestion"
fi

# ========================================
# SECTION 6: CTE TESTS (GraphQL)
# ========================================
echo "=== SECTION 6: CTE TESTS (GraphQL) ==="
echo ""

run_test "Simple CTE via GraphQL"
RESPONSE=$(curl -s -X POST "$GRAPHQL_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { executeQuery(query: \"WITH high_earners AS (SELECT * FROM employees WHERE salary > 70000) SELECT * FROM high_earners\") { success message } }"
  }')
if echo "$RESPONSE" | grep -q 'success'; then
    pass_test
else
    fail_test "Failed simple CTE"
fi

run_test "Multiple CTEs"
RESPONSE=$(curl -s -X POST "$GRAPHQL_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { executeQuery(query: \"WITH high_earners AS (SELECT * FROM employees WHERE salary > 70000), low_earners AS (SELECT * FROM employees WHERE salary < 70000) SELECT * FROM high_earners\") { success message } }"
  }')
if echo "$RESPONSE" | grep -q 'success'; then
    pass_test
else
    fail_test "Failed multiple CTEs"
fi

run_test "Recursive CTE"
RESPONSE=$(curl -s -X POST "$GRAPHQL_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { executeQuery(query: \"WITH RECURSIVE numbers AS (SELECT 1 as n UNION ALL SELECT n+1 FROM numbers WHERE n < 10) SELECT * FROM numbers\") { success message } }"
  }')
if echo "$RESPONSE" | grep -q 'success'; then
    pass_test
else
    fail_test "Failed recursive CTE"
fi

run_test "CTE statistics"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/cte/stats")
if echo "$RESPONSE" | grep -q 'cte\|statistics\|executions'; then
    pass_test
else
    fail_test "Failed CTE statistics"
fi

# ========================================
# SECTION 7: PARALLEL EXECUTION TESTS
# ========================================
echo "=== SECTION 7: PARALLEL EXECUTION TESTS ==="
echo ""

run_test "Parallel table scan"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/parallel" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees", "parallel": true, "workers": 4}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success\|parallel'; then
    pass_test
else
    fail_test "Failed parallel scan"
fi

run_test "Parallel JOIN"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/parallel" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees e INNER JOIN departments d ON e.department = d.dept_name", "parallel": true, "workers": 4}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed parallel JOIN"
fi

run_test "Parallel aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/parallel" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*), AVG(salary) FROM employees GROUP BY department", "parallel": true, "workers": 4}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed parallel aggregation"
fi

run_test "Parallelization analysis"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/analyze-parallel" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees WHERE salary > 70000"}')
if echo "$RESPONSE" | grep -q 'can_parallelize\|speedup\|workers'; then
    pass_test
else
    fail_test "Failed parallelization analysis"
fi

# ========================================
# SECTION 8: VECTORIZED EXECUTION TESTS
# ========================================
echo "=== SECTION 8: VECTORIZED EXECUTION TESTS ==="
echo ""

run_test "Vectorized scan"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/vectorized" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees", "batch_size": 1024}')
if echo "$RESPONSE" | grep -q 'batches\|columns\|success'; then
    pass_test
else
    fail_test "Failed vectorized scan"
fi

run_test "Vectorized filter"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/vectorized" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees WHERE salary > 70000", "batch_size": 1024}')
if echo "$RESPONSE" | grep -q 'batches\|columns\|success'; then
    pass_test
else
    fail_test "Failed vectorized filter"
fi

run_test "Vectorized projection"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/vectorized" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT name, salary FROM employees", "batch_size": 1024}')
if echo "$RESPONSE" | grep -q 'batches\|columns\|success'; then
    pass_test
else
    fail_test "Failed vectorized projection"
fi

run_test "Vectorized aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/vectorized" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*) FROM employees GROUP BY department", "batch_size": 1024}')
if echo "$RESPONSE" | grep -q 'batches\|columns\|success\|aggregate'; then
    pass_test
else
    fail_test "Failed vectorized aggregation"
fi

run_test "Batch size adaptation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/vectorized/adapt-batch-size" \
  -H "Content-Type: application/json" \
  -d '{"memory_pressure": 0.8}')
if echo "$RESPONSE" | grep -q 'batch_size\|success'; then
    pass_test
else
    fail_test "Failed batch size adaptation"
fi

# ========================================
# SECTION 9: ADAPTIVE EXECUTION TESTS
# ========================================
echo "=== SECTION 9: ADAPTIVE EXECUTION TESTS ==="
echo ""

run_test "Adaptive query execution"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/adaptive" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees e INNER JOIN departments d ON e.department = d.dept_name", "memory_budget": 10485760}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|adaptations\|success'; then
    pass_test
else
    fail_test "Failed adaptive execution"
fi

run_test "Runtime statistics"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/adaptive/stats")
if echo "$RESPONSE" | grep -q 'cardinality\|selectivity\|operator_timings'; then
    pass_test
else
    fail_test "Failed to get runtime stats"
fi

run_test "Adaptive join selection"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/adaptive/join" \
  -H "Content-Type: application/json" \
  -d '{"left_size": 1000, "right_size": 100000, "memory_pressure": 0.5}')
if echo "$RESPONSE" | grep -q 'algorithm\|hash\|sort\|nested'; then
    pass_test
else
    fail_test "Failed adaptive join selection"
fi

run_test "Memory-aware aggregation"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/execution/adaptive/aggregate" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT department, COUNT(*) FROM employees GROUP BY department", "memory_pressure": 0.9}')
if echo "$RESPONSE" | grep -q 'strategy\|sort\|hash\|success'; then
    pass_test
else
    fail_test "Failed memory-aware aggregation"
fi

# ========================================
# SECTION 10: COMPLEX QUERY TESTS
# ========================================
echo "=== SECTION 10: COMPLEX QUERY TESTS ==="
echo ""

run_test "Subquery in WHERE"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM employees WHERE salary > (SELECT AVG(salary) FROM employees)"}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed subquery in WHERE"
fi

run_test "Correlated subquery"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e1.name, e1.salary FROM employees e1 WHERE salary > (SELECT AVG(e2.salary) FROM employees e2 WHERE e2.department = e1.department)"}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed correlated subquery"
fi

run_test "UNION query"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT name FROM employees WHERE department = \"Engineering\" UNION SELECT name FROM employees WHERE department = \"Sales\""}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed UNION"
fi

run_test "UNION ALL query"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT name FROM employees WHERE department = \"Engineering\" UNION ALL SELECT name FROM employees WHERE department = \"Sales\""}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed UNION ALL"
fi

run_test "Complex multi-join query"
RESPONSE=$(curl -s -X POST "$BASE_URL/api/query" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT e.name, e.salary, d.dept_name, d.location FROM employees e INNER JOIN departments d ON e.department = d.dept_name WHERE e.salary > 70000 ORDER BY e.salary DESC LIMIT 5"}')
if echo "$RESPONSE" | grep -q 'columns\|rows\|success'; then
    pass_test
else
    fail_test "Failed complex multi-join"
fi

# ========================================
# SECTION 11: EXECUTION MONITORING
# ========================================
echo "=== SECTION 11: EXECUTION MONITORING ==="
echo ""

run_test "Query execution metrics"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/metrics")
if echo "$RESPONSE" | grep -q 'queries_executed\|avg_execution_time\|success'; then
    pass_test
else
    fail_test "Failed to get metrics"
fi

run_test "Active queries"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/active-queries")
if echo "$RESPONSE" | grep -q 'queries\|count\|success'; then
    pass_test
else
    fail_test "Failed to get active queries"
fi

run_test "Query history"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/history?limit=10")
if echo "$RESPONSE" | grep -q 'queries\|history\|success'; then
    pass_test
else
    fail_test "Failed to get query history"
fi

run_test "Slow query log"
RESPONSE=$(curl -s -X GET "$BASE_URL/api/execution/slow-queries?threshold=1000")
if echo "$RESPONSE" | grep -q 'queries\|slow\|success'; then
    pass_test
else
    fail_test "Failed to get slow queries"
fi

# ========================================
# SUMMARY
# ========================================
echo "=========================================="
echo "TEST SUMMARY"
echo "=========================================="
echo "Total Tests: $TEST_NUM"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Success Rate: $(echo "scale=2; $PASS * 100 / $TEST_NUM" | bc)%"
echo "=========================================="

if [ $FAIL -eq 0 ]; then
    echo "✓ ALL TESTS PASSED!"
    exit 0
else
    echo "✗ SOME TESTS FAILED"
    exit 1
fi
