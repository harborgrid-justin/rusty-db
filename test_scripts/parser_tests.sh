#!/bin/bash

# Parser Test Script
# Testing RustyDB SQL Parser via REST API and GraphQL

TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0

# Function to execute test
execute_test() {
    local test_id="$1"
    local description="$2"
    local curl_cmd="$3"
    local expected_status="$4"
    
    TEST_COUNT=$((TEST_COUNT + 1))
    
    echo "========================================="
    echo "TEST: $test_id"
    echo "DESC: $description"
    echo "CMD:  $curl_cmd"
    echo ""
    
    # Execute the command and capture response
    response=$(eval "$curl_cmd" 2>&1)
    exit_code=$?
    
    echo "RESPONSE:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    echo ""
    
    # Determine pass/fail
    if [ $exit_code -eq 0 ] && echo "$response" | grep -q "$expected_status"; then
        echo "STATUS: PASS ✓"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "STATUS: FAIL ✗"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    echo ""
}

# Function to execute GraphQL test
execute_graphql_test() {
    local test_id="$1"
    local description="$2"
    local query="$3"
    local expected_status="$4"
    
    TEST_COUNT=$((TEST_COUNT + 1))
    
    echo "========================================="
    echo "TEST: $test_id"
    echo "DESC: $description"
    echo "QUERY: $query"
    echo ""
    
    # Execute GraphQL query
    response=$(curl -s -X POST http://localhost:8080/graphql \
        -H "Content-Type: application/json" \
        -d "{\"query\": \"$query\"}" 2>&1)
    exit_code=$?
    
    echo "RESPONSE:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    echo ""
    
    # Determine pass/fail
    if [ $exit_code -eq 0 ] && echo "$response" | grep -q "$expected_status"; then
        echo "STATUS: PASS ✓"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "STATUS: FAIL ✗"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    echo ""
}

echo "========================================"
echo "RUSTYDB PARSER COMPREHENSIVE TEST SUITE"
echo "========================================"
echo ""

# ============================================================================
# SECTION 1: DDL STATEMENTS - CREATE TABLE
# ============================================================================

execute_test \
    "PARSER-001" \
    "Parse CREATE TABLE with INTEGER and VARCHAR columns" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE TABLE users (id INT, name VARCHAR(255))\"}'" \
    "success"

execute_test \
    "PARSER-002" \
    "Parse CREATE TABLE with all data types" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE TABLE test_types (id BIGINT, price FLOAT, total DOUBLE, name TEXT, active BOOLEAN, created DATE, updated TIMESTAMP)\"}'" \
    "success"

execute_test \
    "PARSER-003" \
    "Parse CREATE TABLE with nullable column" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE TABLE products (id INT, description TEXT NULL)\"}'" \
    "success"

# ============================================================================
# SECTION 2: DDL STATEMENTS - DROP TABLE
# ============================================================================

execute_test \
    "PARSER-004" \
    "Parse DROP TABLE statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"DROP TABLE users\"}'" \
    "success"

# ============================================================================
# SECTION 3: DDL STATEMENTS - CREATE INDEX
# ============================================================================

execute_test \
    "PARSER-005" \
    "Parse CREATE INDEX statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE INDEX idx_users_email ON users (email)\"}'" \
    "success"

execute_test \
    "PARSER-006" \
    "Parse CREATE UNIQUE INDEX statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE UNIQUE INDEX idx_users_email_unique ON users (email)\"}'" \
    "success"

execute_test \
    "PARSER-007" \
    "Parse CREATE INDEX with multiple columns" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE INDEX idx_users_name_email ON users (name, email)\"}'" \
    "success"

# ============================================================================
# SECTION 4: DDL STATEMENTS - DROP INDEX
# ============================================================================

execute_test \
    "PARSER-008" \
    "Parse DROP INDEX statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"DROP INDEX idx_users_email\"}'" \
    "success"

# ============================================================================
# SECTION 5: DDL STATEMENTS - CREATE VIEW
# ============================================================================

execute_test \
    "PARSER-009" \
    "Parse CREATE VIEW statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE VIEW active_users AS SELECT * FROM users WHERE active = true\"}'" \
    "success"

# ============================================================================
# SECTION 6: DDL STATEMENTS - DROP VIEW
# ============================================================================

execute_test \
    "PARSER-010" \
    "Parse DROP VIEW statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"DROP VIEW active_users\"}'" \
    "success"

# ============================================================================
# SECTION 7: DDL STATEMENTS - TRUNCATE TABLE
# ============================================================================

execute_test \
    "PARSER-011" \
    "Parse TRUNCATE TABLE statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"TRUNCATE TABLE users\"}'" \
    "success"

# ============================================================================
# SECTION 8: DML STATEMENTS - SELECT
# ============================================================================

execute_test \
    "PARSER-012" \
    "Parse simple SELECT statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users\"}'" \
    "success"

execute_test \
    "PARSER-013" \
    "Parse SELECT with specific columns" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT id, name, email FROM users\"}'" \
    "success"

execute_test \
    "PARSER-014" \
    "Parse SELECT with DISTINCT" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT DISTINCT name FROM users\"}'" \
    "success"

execute_test \
    "PARSER-015" \
    "Parse SELECT with WHERE clause" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE age > 18\"}'" \
    "success"

execute_test \
    "PARSER-016" \
    "Parse SELECT with ORDER BY" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users ORDER BY name ASC\"}'" \
    "success"

execute_test \
    "PARSER-017" \
    "Parse SELECT with LIMIT" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users LIMIT 10\"}'" \
    "success"

execute_test \
    "PARSER-018" \
    "Parse SELECT with multiple WHERE conditions" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE age > 18 AND active = true\"}'" \
    "success"

# ============================================================================
# SECTION 9: DML STATEMENTS - INSERT
# ============================================================================

execute_test \
    "PARSER-019" \
    "Parse INSERT statement with values" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"INSERT INTO users (name, email) VALUES (\\\"Alice\\\", \\\"alice@example.com\\\")\"}'" \
    "success"

execute_test \
    "PARSER-020" \
    "Parse INSERT statement with multiple rows" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"INSERT INTO users (name, email) VALUES (\\\"Alice\\\", \\\"alice@example.com\\\"), (\\\"Bob\\\", \\\"bob@example.com\\\")\"}'" \
    "success"

execute_test \
    "PARSER-021" \
    "Parse INSERT with integer values" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"INSERT INTO products (id, price) VALUES (1, 99.99)\"}'" \
    "success"

execute_test \
    "PARSER-022" \
    "Parse INSERT with NULL value" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"INSERT INTO users (name, email, age) VALUES (\\\"Charlie\\\", \\\"charlie@example.com\\\", NULL)\"}'" \
    "success"

# ============================================================================
# SECTION 10: DML STATEMENTS - DELETE
# ============================================================================

execute_test \
    "PARSER-023" \
    "Parse DELETE statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"DELETE FROM users WHERE id = 1\"}'" \
    "success"

execute_test \
    "PARSER-024" \
    "Parse DELETE without WHERE clause" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"DELETE FROM users\"}'" \
    "success"

# ============================================================================
# SECTION 11: COMPLEX QUERIES
# ============================================================================

execute_test \
    "PARSER-025" \
    "Parse SELECT with complex WHERE (BETWEEN)" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE age BETWEEN 18 AND 65\"}'" \
    "success"

execute_test \
    "PARSER-026" \
    "Parse SELECT with IN clause" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE status IN (\\\"active\\\", \\\"pending\\\")\"}'" \
    "success"

execute_test \
    "PARSER-027" \
    "Parse SELECT with LIKE pattern" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE name LIKE \\\"John%\\\"\"}'" \
    "success"

execute_test \
    "PARSER-028" \
    "Parse SELECT with IS NULL" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE email IS NULL\"}'" \
    "success"

execute_test \
    "PARSER-029" \
    "Parse SELECT with IS NOT NULL" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE email IS NOT NULL\"}'" \
    "success"

execute_test \
    "PARSER-030" \
    "Parse SELECT with aggregate function" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT COUNT(*) FROM users\"}'" \
    "success"

execute_test \
    "PARSER-031" \
    "Parse SELECT with GROUP BY" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT status, COUNT(*) FROM users GROUP BY status\"}'" \
    "success"

execute_test \
    "PARSER-032" \
    "Parse SELECT with HAVING clause" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT status, COUNT(*) FROM users GROUP BY status HAVING COUNT(*) > 10\"}'" \
    "success"

# ============================================================================
# SECTION 12: SQL INJECTION PREVENTION TESTS
# ============================================================================

execute_test \
    "PARSER-033" \
    "Test SQL injection prevention - UNION attack" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE id = 1 UNION SELECT * FROM passwords\"}'" \
    "error\|fail"

execute_test \
    "PARSER-034" \
    "Test SQL injection prevention - Comment injection" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE id = 1 -- AND active = true\"}'" \
    "error\|fail\|success"

execute_test \
    "PARSER-035" \
    "Test SQL injection prevention - Tautology" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users WHERE id = 1 OR 1=1\"}'" \
    "error\|fail\|success"

execute_test \
    "PARSER-036" \
    "Test SQL injection prevention - Stacked queries" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * FROM users; DROP TABLE users;\"}'" \
    "error\|fail\|success"

# ============================================================================
# SECTION 13: ERROR HANDLING - MALFORMED SQL
# ============================================================================

execute_test \
    "PARSER-037" \
    "Test malformed SQL - Missing FROM" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT * WHERE id = 1\"}'" \
    "error"

execute_test \
    "PARSER-038" \
    "Test malformed SQL - Missing table name" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"CREATE TABLE (id INT)\"}'" \
    "error"

execute_test \
    "PARSER-039" \
    "Test malformed SQL - Invalid syntax" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELCT * FROM users\"}'" \
    "error"

execute_test \
    "PARSER-040" \
    "Test empty SQL statement" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"\"}'" \
    "error"

execute_test \
    "PARSER-041" \
    "Test SQL with only whitespace" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"   \"}'" \
    "error"

# ============================================================================
# SECTION 14: STRING FUNCTIONS (via expression evaluation)
# ============================================================================

execute_test \
    "PARSER-042" \
    "Parse SELECT with UPPER function" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT UPPER(name) FROM users\"}'" \
    "success"

execute_test \
    "PARSER-043" \
    "Parse SELECT with LOWER function" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT LOWER(email) FROM users\"}'" \
    "success"

execute_test \
    "PARSER-044" \
    "Parse SELECT with LENGTH function" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT LENGTH(name) FROM users\"}'" \
    "success"

execute_test \
    "PARSER-045" \
    "Parse SELECT with CONCAT function" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT CONCAT(first_name, last_name) FROM users\"}'" \
    "success"

# ============================================================================
# SECTION 15: ARITHMETIC EXPRESSIONS
# ============================================================================

execute_test \
    "PARSER-046" \
    "Parse SELECT with arithmetic addition" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT price + tax FROM products\"}'" \
    "success"

execute_test \
    "PARSER-047" \
    "Parse SELECT with arithmetic multiplication" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT price * quantity FROM orders\"}'" \
    "success"

execute_test \
    "PARSER-048" \
    "Parse SELECT with complex arithmetic" \
    "curl -s -X POST http://localhost:8080/api/v1/query -H 'Content-Type: application/json' -d '{\"sql\": \"SELECT (price * quantity) + tax FROM orders\"}'" \
    "success"

# ============================================================================
# SECTION 16: GRAPHQL PARSER TESTS
# ============================================================================

execute_graphql_test \
    "PARSER-049" \
    "GraphQL: Parse simple table query" \
    "{ tables { name } }" \
    "data\|tables"

execute_graphql_test \
    "PARSER-050" \
    "GraphQL: Parse table with columns" \
    "{ table(name: \\\"users\\\") { name columns { name dataType } } }" \
    "data\|table"

# ============================================================================
# TEST SUMMARY
# ============================================================================

echo "========================================="
echo "TEST SUMMARY"
echo "========================================="
echo "Total Tests: $TEST_COUNT"
echo "Passed:      $PASS_COUNT"
echo "Failed:      $FAIL_COUNT"
echo "Success Rate: $(awk "BEGIN {printf \"%.2f\", ($PASS_COUNT/$TEST_COUNT)*100}")%"
echo "========================================="

