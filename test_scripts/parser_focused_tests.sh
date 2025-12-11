#!/bin/bash

# Parser-Focused Test Script
# Distinguishes between parser success and execution success

TEST_COUNT=0
PARSER_PASS=0
PARSER_FAIL=0

# Function to execute parser test
# Parser succeeds if: query_id exists OR execution_error occurs (means parsing succeeded)
# Parser fails if: SQL_PARSE_ERROR or INVALID_INPUT occurs
test_parser() {
    local test_id="$1"
    local description="$2"
    local sql="$3"
    
    TEST_COUNT=$((TEST_COUNT + 1))
    
    echo "==========================================="
    echo "TEST: $test_id"
    echo "DESC: $description"
    echo "SQL:  $sql"
    echo ""
    
    # Execute the query
    response=$(curl -s -X POST http://localhost:8080/api/v1/query \
        -H 'Content-Type: application/json' \
        -d "{\"sql\": \"$sql\"}" 2>&1)
    
    echo "RESPONSE:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    echo ""
    
    # Check if parser succeeded
    # Parser succeeds if: query_id exists (successful parse + execution) OR EXECUTION_ERROR (successful parse, failed execution)
    # Parser fails if: SQL_PARSE_ERROR or INVALID_INPUT (parsing failed)
    if echo "$response" | grep -q "SQL_PARSE_ERROR\|INVALID_INPUT"; then
        echo "PARSER STATUS: FAIL ✗ (Parser rejected the SQL)"
        PARSER_FAIL=$((PARSER_FAIL + 1))
    elif echo "$response" | grep -q "query_id\|EXECUTION_ERROR"; then
        echo "PARSER STATUS: PASS ✓ (Parser accepted the SQL)"
        PARSER_PASS=$((PARSER_PASS + 1))
    else
        echo "PARSER STATUS: UNKNOWN"
        PARSER_FAIL=$((PARSER_FAIL + 1))
    fi
    echo ""
}

echo "============================================="
echo "RUSTYDB SQL PARSER COMPREHENSIVE TEST REPORT"
echo "============================================="
echo ""
echo "Test Objective: Verify SQL parser can correctly parse"
echo "various SQL statements independent of execution success"
echo ""

# First, create test tables
echo "==========================================="
echo "SETUP: Creating Test Tables"
echo "==========================================="

curl -s -X POST http://localhost:8080/api/v1/query \
    -H 'Content-Type: application/json' \
    -d '{"sql": "CREATE TABLE test_users (id BIGINT, name TEXT, email TEXT, age BIGINT, active BOOLEAN, created DATE)"}' | jq '.'

curl -s -X POST http://localhost:8080/api/v1/query \
    -H 'Content-Type: application/json' \
    -d '{"sql": "CREATE TABLE test_products (id BIGINT, name TEXT, price FLOAT, tax FLOAT, quantity BIGINT, created TIMESTAMP)"}' | jq '.'

curl -s -X POST http://localhost:8080/api/v1/query \
    -H 'Content-Type: application/json' \
    -d '{"sql": "CREATE TABLE test_orders (id BIGINT, user_id BIGINT, product_id BIGINT, quantity BIGINT, total DOUBLE, status TEXT)"}' | jq '.'

echo ""
echo "============================================="
echo "SECTION 1: DDL STATEMENT PARSING"
echo "============================================="
echo ""

test_parser "PARSER-001" "CREATE TABLE with multiple data types" \
    "CREATE TABLE ddl_test1 (id BIGINT, name TEXT, active BOOLEAN, created DATE)"

test_parser "PARSER-002" "CREATE TABLE with TIMESTAMP and FLOAT" \
    "CREATE TABLE ddl_test2 (id BIGINT, price FLOAT, created TIMESTAMP)"

test_parser "PARSER-003" "DROP TABLE statement" \
    "DROP TABLE nonexistent_table"

test_parser "PARSER-004" "CREATE INDEX statement" \
    "CREATE INDEX idx_test ON test_users (email)"

test_parser "PARSER-005" "CREATE VIEW statement" \
    "CREATE VIEW test_view AS SELECT * FROM test_users WHERE active = true"

test_parser "PARSER-006" "TRUNCATE TABLE statement" \
    "TRUNCATE TABLE test_users"

echo ""
echo "============================================="
echo "SECTION 2: DML STATEMENT PARSING - SELECT"
echo "============================================="
echo ""

test_parser "PARSER-007" "Simple SELECT *" \
    "SELECT * FROM test_users"

test_parser "PARSER-008" "SELECT with specific columns" \
    "SELECT id, name, email FROM test_users"

test_parser "PARSER-009" "SELECT with WHERE clause" \
    "SELECT * FROM test_users WHERE age > 18"

test_parser "PARSER-010" "SELECT with AND condition" \
    "SELECT * FROM test_users WHERE age > 18 AND active = true"

test_parser "PARSER-011" "SELECT with OR condition" \
    "SELECT * FROM test_users WHERE age < 18 OR age > 65"

test_parser "PARSER-012" "SELECT with ORDER BY" \
    "SELECT * FROM test_users ORDER BY name ASC"

test_parser "PARSER-013" "SELECT with LIMIT" \
    "SELECT * FROM test_users LIMIT 10"

test_parser "PARSER-014" "SELECT with OFFSET and LIMIT" \
    "SELECT * FROM test_users LIMIT 10 OFFSET 5"

test_parser "PARSER-015" "SELECT with DISTINCT" \
    "SELECT DISTINCT name FROM test_users"

echo ""
echo "============================================="
echo "SECTION 3: DML STATEMENT PARSING - INSERT"
echo "============================================="
echo ""

test_parser "PARSER-016" "INSERT single row with string values" \
    "INSERT INTO test_users (name, email) VALUES ('Alice', 'alice@example.com')"

test_parser "PARSER-017" "INSERT with integer values" \
    "INSERT INTO test_products (id, price) VALUES (1, 99.99)"

test_parser "PARSER-018" "INSERT with boolean value" \
    "INSERT INTO test_users (name, active) VALUES ('Bob', true)"

test_parser "PARSER-019" "INSERT multiple rows" \
    "INSERT INTO test_users (name, email) VALUES ('Charlie', 'charlie@test.com'), ('David', 'david@test.com')"

echo ""
echo "============================================="
echo "SECTION 4: DML STATEMENT PARSING - DELETE"
echo "============================================="
echo ""

test_parser "PARSER-020" "DELETE with WHERE clause" \
    "DELETE FROM test_users WHERE id = 1"

test_parser "PARSER-021" "DELETE with complex WHERE" \
    "DELETE FROM test_users WHERE age < 18 AND active = false"

test_parser "PARSER-022" "DELETE all rows (no WHERE)" \
    "DELETE FROM test_users"

echo ""
echo "============================================="
echo "SECTION 5: COMPLEX QUERIES"
echo "============================================="
echo ""

test_parser "PARSER-023" "SELECT with BETWEEN" \
    "SELECT * FROM test_users WHERE age BETWEEN 18 AND 65"

test_parser "PARSER-024" "SELECT with IN clause" \
    "SELECT * FROM test_orders WHERE status IN ('pending', 'processing', 'shipped')"

test_parser "PARSER-025" "SELECT with LIKE pattern" \
    "SELECT * FROM test_users WHERE name LIKE 'John%'"

test_parser "PARSER-026" "SELECT with IS NULL" \
    "SELECT * FROM test_users WHERE email IS NULL"

test_parser "PARSER-027" "SELECT with IS NOT NULL" \
    "SELECT * FROM test_users WHERE email IS NOT NULL"

test_parser "PARSER-028" "SELECT with NOT LIKE" \
    "SELECT * FROM test_users WHERE name NOT LIKE '%test%'"

test_parser "PARSER-029" "SELECT with complex boolean logic" \
    "SELECT * FROM test_users WHERE (age > 18 AND active = true) OR (age > 65 AND active = false)"

echo ""
echo "============================================="
echo "SECTION 6: AGGREGATE FUNCTIONS"
echo "============================================="
echo ""

test_parser "PARSER-030" "SELECT with COUNT(*)" \
    "SELECT COUNT(*) FROM test_users"

test_parser "PARSER-031" "SELECT with SUM" \
    "SELECT SUM(price) FROM test_products"

test_parser "PARSER-032" "SELECT with AVG" \
    "SELECT AVG(age) FROM test_users"

test_parser "PARSER-033" "SELECT with MIN and MAX" \
    "SELECT MIN(price), MAX(price) FROM test_products"

test_parser "PARSER-034" "SELECT with GROUP BY" \
    "SELECT status, COUNT(*) FROM test_orders GROUP BY status"

test_parser "PARSER-035" "SELECT with HAVING" \
    "SELECT status, COUNT(*) FROM test_orders GROUP BY status HAVING COUNT(*) > 5"

echo ""
echo "============================================="
echo "SECTION 7: STRING FUNCTIONS"
echo "============================================="
echo ""

test_parser "PARSER-036" "SELECT with UPPER function" \
    "SELECT UPPER(name) FROM test_users"

test_parser "PARSER-037" "SELECT with LOWER function" \
    "SELECT LOWER(email) FROM test_users"

test_parser "PARSER-038" "SELECT with LENGTH function" \
    "SELECT LENGTH(name) FROM test_users"

test_parser "PARSER-039" "SELECT with CONCAT function" \
    "SELECT CONCAT(name, email) FROM test_users"

test_parser "PARSER-040" "SELECT with SUBSTRING" \
    "SELECT SUBSTRING(name, 1, 5) FROM test_users"

test_parser "PARSER-041" "SELECT with TRIM" \
    "SELECT TRIM(name) FROM test_users"

echo ""
echo "============================================="
echo "SECTION 8: ARITHMETIC EXPRESSIONS"
echo "============================================="
echo ""

test_parser "PARSER-042" "SELECT with addition" \
    "SELECT price + tax FROM test_products"

test_parser "PARSER-043" "SELECT with subtraction" \
    "SELECT price - tax FROM test_products"

test_parser "PARSER-044" "SELECT with multiplication" \
    "SELECT price * quantity FROM test_products"

test_parser "PARSER-045" "SELECT with division" \
    "SELECT price / quantity FROM test_products"

test_parser "PARSER-046" "SELECT with complex arithmetic" \
    "SELECT (price * quantity) + tax FROM test_products"

test_parser "PARSER-047" "SELECT with modulo" \
    "SELECT id % 10 FROM test_users"

echo ""
echo "============================================="
echo "SECTION 9: SQL INJECTION PREVENTION"
echo "============================================="
echo ""

test_parser "PARSER-048" "SQL injection - UNION attack (SHOULD FAIL)" \
    "SELECT * FROM test_users WHERE id = 1 UNION SELECT * FROM passwords"

test_parser "PARSER-049" "SQL injection - Comment bypass (SHOULD FAIL)" \
    "SELECT * FROM test_users WHERE id = 1 -- AND active = true"

test_parser "PARSER-050" "SQL injection - Tautology (SHOULD FAIL)" \
    "SELECT * FROM test_users WHERE id = 1 OR 1=1"

test_parser "PARSER-051" "SQL injection - Stacked queries (SHOULD FAIL)" \
    "SELECT * FROM test_users; DROP TABLE test_users;"

echo ""
echo "============================================="
echo "SECTION 10: ERROR HANDLING - MALFORMED SQL"
echo "============================================="
echo ""

test_parser "PARSER-052" "Missing FROM clause (SHOULD FAIL)" \
    "SELECT * WHERE id = 1"

test_parser "PARSER-053" "Missing table name (SHOULD FAIL)" \
    "CREATE TABLE (id INT)"

test_parser "PARSER-054" "Invalid keyword (SHOULD FAIL)" \
    "SELCT * FROM test_users"

test_parser "PARSER-055" "Empty SQL (SHOULD FAIL)" \
    ""

test_parser "PARSER-056" "Incomplete WHERE clause (SHOULD FAIL)" \
    "SELECT * FROM test_users WHERE"

test_parser "PARSER-057" "Unmatched parentheses (SHOULD FAIL)" \
    "SELECT * FROM test_users WHERE (age > 18"

test_parser "PARSER-058" "Invalid column in ORDER BY (SHOULD SUCCEED - parser accepts it)" \
    "SELECT * FROM test_users ORDER BY nonexistent_column"

echo ""
echo "============================================="
echo "SECTION 11: DATA TYPE PARSING"
echo "============================================="
echo ""

test_parser "PARSER-059" "CREATE TABLE with all supported types" \
    "CREATE TABLE type_test (col_bigint BIGINT, col_int INT, col_float FLOAT, col_double DOUBLE, col_text TEXT, col_varchar VARCHAR(255), col_bool BOOLEAN, col_date DATE, col_timestamp TIMESTAMP)"

test_parser "PARSER-060" "INSERT with various data types" \
    "INSERT INTO type_test VALUES (9223372036854775807, 2147483647, 3.14, 2.718281828, 'text', 'varchar', true, '2025-12-11', '2025-12-11 10:00:00')"

echo ""
echo "============================================="
echo "TEST SUMMARY"
echo "============================================="
echo "Total Tests:    $TEST_COUNT"
echo "Parser Success: $PARSER_PASS"
echo "Parser Failure: $PARSER_FAIL"
echo "Success Rate:   $(awk "BEGIN {printf \"%.2f\", ($PARSER_PASS/$TEST_COUNT)*100}")%"
echo "============================================="
echo ""
echo "NOTE: Parser success means the SQL was syntactically correct"
echo "      and successfully parsed, regardless of execution outcome."
echo "============================================="

