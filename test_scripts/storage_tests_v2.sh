#!/bin/bash

# RustyDB Storage Engine Tests - 100 Tests Total (CORRECTED VERSION)
# Tests execute against real server on localhost:8080 and localhost:5432

BASE_URL="http://localhost:8080"
RESULTS_FILE="/tmp/storage_test_results_v2.txt"
echo "RustyDB Storage Engine Test Results - $(date)" > $RESULTS_FILE
echo "========================================" >> $RESULTS_FILE

# Helper function to test via REST API
test_rest() {
    local test_id="$1"
    local description="$2"
    local method="$3"
    local endpoint="$4"
    local data="$5"

    echo -n "$test_id: $description... "
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X $method "$BASE_URL$endpoint" \
        -H "Content-Type: application/json" \
        -d "$data" 2>&1)

    http_code=$(echo "$response" | grep "HTTP_CODE" | cut -d: -f2)
    body=$(echo "$response" | grep -v "HTTP_CODE")

    if [[ "$http_code" =~ ^(200|201|204)$ ]]; then
        echo "PASS"
        echo "$test_id: PASS - $description" >> $RESULTS_FILE
        echo "  Response: $body" >> $RESULTS_FILE
    else
        echo "FAIL (HTTP $http_code)"
        echo "$test_id: FAIL - $description (HTTP $http_code)" >> $RESULTS_FILE
        echo "  Response: $body" >> $RESULTS_FILE
    fi
}

# Helper for SQL queries
test_sql() {
    local test_id="$1"
    local description="$2"
    local sql="$3"

    echo -n "$test_id: $description... "
    response=$(curl -s -X POST "$BASE_URL/api/v1/query" \
        -H "Content-Type: application/json" \
        -d "{\"sql\":\"$sql\"}")

    if echo "$response" | grep -q '"row_count"'; then
        echo "PASS"
        echo "$test_id: PASS - $description" >> $RESULTS_FILE
        echo "  SQL: $sql" >> $RESULTS_FILE
        row_count=$(echo "$response" | jq -r '.row_count')
        exec_time=$(echo "$response" | jq -r '.execution_time_ms')
        echo "  Rows: $row_count, Time: ${exec_time}ms" >> $RESULTS_FILE
    else
        echo "FAIL"
        echo "$test_id: FAIL - $description" >> $RESULTS_FILE
        echo "  SQL: $sql" >> $RESULTS_FILE
        echo "  Error: $response" >> $RESULTS_FILE
    fi
}

echo ""
echo "===== PART 1: PAGE-BASED STORAGE TESTS (STOR-001 to STOR-020) ====="
echo ""

# STOR-001: Create basic table for page storage
test_rest "STOR-001" "Create table with INTEGER and VARCHAR columns" "POST" "/api/v1/tables/page_test_001" \
    '{"table_name":"page_test_001","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"data","data_type":"VARCHAR(255)","nullable":true}]}'

# STOR-002: Insert single row
test_sql "STOR-002" "Insert single row into page storage" \
    "INSERT INTO page_test_001 (id, data) VALUES (1, 'Test Data')"

# STOR-003: Query inserted data
test_sql "STOR-003" "Query data from page storage" \
    "SELECT * FROM page_test_001"

# STOR-004: Insert multiple rows (simulated - no generate_series)
test_sql "STOR-004" "Insert row 2" \
    "INSERT INTO page_test_001 (id, data) VALUES (2, 'Data 2')"

test_sql "STOR-005" "Insert row 3" \
    "INSERT INTO page_test_001 (id, data) VALUES (3, 'Data 3')"

test_sql "STOR-006" "Insert row 4" \
    "INSERT INTO page_test_001 (id, data) VALUES (4, 'Data 4')"

test_sql "STOR-007" "Insert row 5" \
    "INSERT INTO page_test_001 (id, data) VALUES (5, 'Data 5')"

# STOR-008: Verify row count
test_sql "STOR-008" "Count rows in page storage" \
    "SELECT COUNT(*) FROM page_test_001"

# STOR-009: Create table with all basic data types
test_rest "STOR-009" "Create table with multiple data types" "POST" "/api/v1/tables/page_test_types" \
    '{"table_name":"page_test_types","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"name","data_type":"VARCHAR(100)","nullable":false},{"name":"price","data_type":"FLOAT","nullable":true},{"name":"active","data_type":"BOOLEAN","nullable":true}]}'

# STOR-010: Insert mixed type data
test_sql "STOR-010" "Insert mixed data types" \
    "INSERT INTO page_test_types (id, name, price, active) VALUES (1, 'Product A', 19.99, true)"

# STOR-011: Query mixed types
test_sql "STOR-011" "Query mixed data types" \
    "SELECT * FROM page_test_types WHERE active = true"

# STOR-012: Update operation (page modification)
test_sql "STOR-012" "Update data in page" \
    "UPDATE page_test_001 SET data = 'Updated Data' WHERE id = 1"

# STOR-013: Delete operation (page modification)
test_sql "STOR-013" "Delete data from page" \
    "DELETE FROM page_test_001 WHERE id = 3"

# STOR-014: Create table with TEXT column (large data)
test_rest "STOR-014" "Create table with TEXT column" "POST" "/api/v1/tables/page_test_large" \
    '{"table_name":"page_test_large","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"content","data_type":"TEXT","nullable":true}]}'

# STOR-015: Insert large text data
test_sql "STOR-015" "Insert large TEXT data" \
    "INSERT INTO page_test_large (id, content) VALUES (1, 'Large content that spans multiple pages with lots of text to test storage efficiency and page management in the database system')"

# STOR-016: Create table with BIGINT
test_rest "STOR-016" "Create table with BIGINT" "POST" "/api/v1/tables/page_test_bigint" \
    '{"table_name":"page_test_bigint","columns":[{"name":"id","data_type":"BIGINT","nullable":false},{"name":"value","data_type":"BIGINT","nullable":true}]}'

# STOR-017: Insert BIGINT values
test_sql "STOR-017" "Insert BIGINT values" \
    "INSERT INTO page_test_bigint (id, value) VALUES (9223372036854775, 1234567890123)"

# STOR-018: Create table with DOUBLE
test_rest "STOR-018" "Create table with DOUBLE precision" "POST" "/api/v1/tables/page_test_double" \
    '{"table_name":"page_test_double","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"DOUBLE","nullable":true}]}'

# STOR-019: Insert DOUBLE values
test_sql "STOR-019" "Insert DOUBLE precision values" \
    "INSERT INTO page_test_double (id, value) VALUES (1, 3.141592653589793)"

# STOR-020: Create table with DATE and TIMESTAMP
test_rest "STOR-020" "Create table with DATE and TIMESTAMP" "POST" "/api/v1/tables/page_test_datetime" \
    '{"table_name":"page_test_datetime","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"birth_date","data_type":"DATE","nullable":true},{"name":"created_at","data_type":"TIMESTAMP","nullable":true}]}'

echo ""
echo "===== PART 2: LSM TREE STORAGE TESTS (STOR-021 to STOR-040) ====="
echo ""

# STOR-021: Create table for LSM testing
test_rest "STOR-021" "Create table for LSM storage" "POST" "/api/v1/tables/lsm_test_001" \
    '{"table_name":"lsm_test_001","columns":[{"name":"key","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"VARCHAR(255)","nullable":true}]}'

# STOR-022-026: Insert multiple rows (write-heavy workload)
for i in {1..5}; do
    test_sql "STOR-$(printf '%03d' $((21+i)))" "LSM Insert key $i" \
        "INSERT INTO lsm_test_001 (key, value) VALUES ($i, 'Value $i')"
done

# STOR-027: Update on LSM
test_sql "STOR-027" "LSM update operation" \
    "UPDATE lsm_test_001 SET value = 'Updated 1' WHERE key = 1"

# STOR-028: Read after write
test_sql "STOR-028" "LSM read after write" \
    "SELECT * FROM lsm_test_001 WHERE key = 1"

# STOR-029: Range scan on LSM
test_sql "STOR-029" "LSM range scan" \
    "SELECT * FROM lsm_test_001 WHERE key >= 1 AND key <= 3"

# STOR-030-034: More LSM inserts
for i in {6..10}; do
    test_sql "STOR-$(printf '%03d' $((24+i)))" "LSM Insert key $i" \
        "INSERT INTO lsm_test_001 (key, value) VALUES ($i, 'Value $i')"
done

# STOR-035: Count on LSM
test_sql "STOR-035" "LSM count operation" \
    "SELECT COUNT(*) FROM lsm_test_001"

# STOR-036: Delete on LSM (tombstones)
test_sql "STOR-036" "LSM delete (tombstone)" \
    "DELETE FROM lsm_test_001 WHERE key = 5"

# STOR-037: Query after delete
test_sql "STOR-037" "LSM query after delete" \
    "SELECT * FROM lsm_test_001 WHERE key = 5"

# STOR-038: Create second LSM table
test_rest "STOR-038" "Create second LSM table" "POST" "/api/v1/tables/lsm_test_002" \
    '{"table_name":"lsm_test_002","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"data","data_type":"TEXT","nullable":true}]}'

# STOR-039-040: Insert varying size data
test_sql "STOR-039" "LSM insert small data" \
    "INSERT INTO lsm_test_002 VALUES (1, 'Small')"

test_sql "STOR-040" "LSM insert large data" \
    "INSERT INTO lsm_test_002 VALUES (2, 'Large data with lots of content to test LSM storage efficiency and compaction behavior')"

echo ""
echo "===== PART 3: COLUMNAR STORAGE TESTS (STOR-041 to STOR-060) ====="
echo ""

# STOR-041: Create table for columnar storage
test_rest "STOR-041" "Create columnar analytics table" "POST" "/api/v1/tables/columnar_test_001" \
    '{"table_name":"columnar_test_001","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"category","data_type":"VARCHAR(50)","nullable":true},{"name":"amount","data_type":"FLOAT","nullable":true},{"name":"quantity","data_type":"INTEGER","nullable":true}]}'

# STOR-042-046: Insert analytics data
test_sql "STOR-042" "Insert analytics data row 1" \
    "INSERT INTO columnar_test_001 (id, category, amount, quantity) VALUES (1, 'A', 100.50, 10)"

test_sql "STOR-043" "Insert analytics data row 2" \
    "INSERT INTO columnar_test_001 (id, category, amount, quantity) VALUES (2, 'B', 200.75, 20)"

test_sql "STOR-044" "Insert analytics data row 3" \
    "INSERT INTO columnar_test_001 (id, category, amount, quantity) VALUES (3, 'A', 150.25, 15)"

test_sql "STOR-045" "Insert analytics data row 4" \
    "INSERT INTO columnar_test_001 (id, category, amount, quantity) VALUES (4, 'C', 300.00, 30)"

test_sql "STOR-046" "Insert analytics data row 5" \
    "INSERT INTO columnar_test_001 (id, category, amount, quantity) VALUES (5, 'B', 175.50, 17)"

# STOR-047: Column-wise aggregate (SUM)
test_sql "STOR-047" "SUM aggregate on column" \
    "SELECT SUM(amount) FROM columnar_test_001"

# STOR-048: Column-wise aggregate (AVG)
test_sql "STOR-048" "AVG aggregate on column" \
    "SELECT AVG(quantity) FROM columnar_test_001"

# STOR-049: Grouped aggregation
test_sql "STOR-049" "GROUP BY on columnar storage" \
    "SELECT category, SUM(amount) FROM columnar_test_001 GROUP BY category"

# STOR-050: Column scan performance
test_sql "STOR-050" "Full column scan (COUNT)" \
    "SELECT COUNT(*) FROM columnar_test_001"

# STOR-051: Selective column read
test_sql "STOR-051" "Select specific columns only" \
    "SELECT category, amount FROM columnar_test_001 WHERE id < 4"

# STOR-052: Multi-column aggregation
test_sql "STOR-052" "Multi-column aggregate" \
    "SELECT category, COUNT(*), SUM(amount) FROM columnar_test_001 GROUP BY category"

# STOR-053: Create second columnar table
test_rest "STOR-053" "Create sales columnar table" "POST" "/api/v1/tables/columnar_sales" \
    '{"table_name":"columnar_sales","columns":[{"name":"sale_id","data_type":"INTEGER","nullable":false},{"name":"product","data_type":"VARCHAR(100)","nullable":true},{"name":"price","data_type":"DOUBLE","nullable":true},{"name":"sold","data_type":"INTEGER","nullable":true},{"name":"revenue","data_type":"DOUBLE","nullable":true}]}'

# STOR-054-056: Insert sales data
test_sql "STOR-054" "Insert sales record 1" \
    "INSERT INTO columnar_sales VALUES (1, 'Widget', 19.99, 100, 1999.00)"

test_sql "STOR-055" "Insert sales record 2" \
    "INSERT INTO columnar_sales VALUES (2, 'Gadget', 29.99, 50, 1499.50)"

test_sql "STOR-056" "Insert sales record 3" \
    "INSERT INTO columnar_sales VALUES (3, 'Doohickey', 9.99, 200, 1998.00)"

# STOR-057: Revenue calculation
test_sql "STOR-057" "Calculate total revenue" \
    "SELECT SUM(revenue) FROM columnar_sales"

# STOR-058: Filter and aggregate
test_sql "STOR-058" "Filter and aggregate" \
    "SELECT product, revenue FROM columnar_sales WHERE price > 10.0"

# STOR-059: Create compression test table
test_rest "STOR-059" "Create table for compression test" "POST" "/api/v1/tables/columnar_compress" \
    '{"table_name":"columnar_compress","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"status","data_type":"VARCHAR(20)","nullable":true},{"name":"value","data_type":"INTEGER","nullable":true}]}'

# STOR-060: Insert repeating values
test_sql "STOR-060" "Insert repeating values" \
    "INSERT INTO columnar_compress VALUES (1, 'ACTIVE', 10), (2, 'ACTIVE', 20), (3, 'ACTIVE', 30), (4, 'INACTIVE', 40)"

echo ""
echo "===== PART 4: BUFFER POOL TESTS (STOR-061 to STOR-080) ====="
echo ""

# STOR-061: Get initial metrics
test_rest "STOR-061" "Get initial buffer pool metrics" "GET" "/api/v1/metrics" ""

# STOR-062: Create table for buffer pool test
test_rest "STOR-062" "Create table for buffer test" "POST" "/api/v1/tables/buffer_test_001" \
    '{"table_name":"buffer_test_001","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"data","data_type":"VARCHAR(1000)","nullable":true}]}'

# STOR-063-067: Insert data to fill buffer
for i in {1..5}; do
    test_sql "STOR-$(printf '%03d' $((62+i)))" "Buffer test insert $i" \
        "INSERT INTO buffer_test_001 VALUES ($i, 'Data XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX')"
done

# STOR-068: Sequential scan (buffer pool utilization)
test_sql "STOR-068" "Sequential scan (buffer pool)" \
    "SELECT COUNT(*) FROM buffer_test_001"

# STOR-069-071: Random access pattern
test_sql "STOR-069" "Random access pattern 1" \
    "SELECT * FROM buffer_test_001 WHERE id = 3"

test_sql "STOR-070" "Random access pattern 2" \
    "SELECT * FROM buffer_test_001 WHERE id = 1"

test_sql "STOR-071" "Random access pattern 3" \
    "SELECT * FROM buffer_test_001 WHERE id = 5"

# STOR-072: Get metrics after operations
test_rest "STOR-072" "Get buffer metrics after ops" "GET" "/api/v1/metrics" ""

# STOR-073-074: Create multiple tables (buffer pressure)
test_rest "STOR-073" "Create buffer test table 2" "POST" "/api/v1/tables/buffer_test_002" \
    '{"table_name":"buffer_test_002","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"TEXT","nullable":true}]}'

test_rest "STOR-074" "Create buffer test table 3" "POST" "/api/v1/tables/buffer_test_003" \
    '{"table_name":"buffer_test_003","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"TEXT","nullable":true}]}'

# STOR-075-077: Insert into multiple tables
test_sql "STOR-075" "Insert into buffer_test_002" \
    "INSERT INTO buffer_test_002 VALUES (1, 'Value 1'), (2, 'Value 2')"

test_sql "STOR-076" "Insert into buffer_test_003" \
    "INSERT INTO buffer_test_003 VALUES (1, 'Value 1'), (2, 'Value 2')"

test_sql "STOR-077" "Query buffer_test_001" \
    "SELECT COUNT(*) FROM buffer_test_001"

# STOR-078-079: Cross-table queries
test_sql "STOR-078" "Query buffer_test_002" \
    "SELECT COUNT(*) FROM buffer_test_002"

test_sql "STOR-079" "Query buffer_test_003" \
    "SELECT COUNT(*) FROM buffer_test_003"

# STOR-080: Update operations (dirty page management)
test_sql "STOR-080" "Updates (dirty pages)" \
    "UPDATE buffer_test_001 SET data = 'Updated' WHERE id = 1"

echo ""
echo "===== PART 5: DATA TYPES STORAGE TESTS (STOR-081 to STOR-100) ====="
echo ""

# STOR-081: INTEGER storage
test_rest "STOR-081" "Create INTEGER test table" "POST" "/api/v1/tables/type_test_integer" \
    '{"table_name":"type_test_integer","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"INTEGER","nullable":true}]}'

test_sql "STOR-082" "Insert INTEGER max value" \
    "INSERT INTO type_test_integer VALUES (1, 2147483647)"

test_sql "STOR-083" "Insert INTEGER min value" \
    "INSERT INTO type_test_integer VALUES (2, -2147483648)"

test_sql "STOR-084" "Insert INTEGER zero" \
    "INSERT INTO type_test_integer VALUES (3, 0)"

# STOR-085: FLOAT storage
test_rest "STOR-085" "Create FLOAT test table" "POST" "/api/v1/tables/type_test_float" \
    '{"table_name":"type_test_float","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"FLOAT","nullable":true}]}'

test_sql "STOR-086" "Insert FLOAT pi" \
    "INSERT INTO type_test_float VALUES (1, 3.14159)"

test_sql "STOR-087" "Insert FLOAT negative" \
    "INSERT INTO type_test_float VALUES (2, -2.71828)"

# STOR-088: VARCHAR storage
test_rest "STOR-088" "Create VARCHAR test table" "POST" "/api/v1/tables/type_test_varchar" \
    '{"table_name":"type_test_varchar","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"short_text","data_type":"VARCHAR(10)","nullable":true},{"name":"medium_text","data_type":"VARCHAR(100)","nullable":true}]}'

test_sql "STOR-089" "Insert VARCHAR values" \
    "INSERT INTO type_test_varchar VALUES (1, 'Short', 'Medium length text here')"

# STOR-090: TEXT storage
test_rest "STOR-090" "Create TEXT test table" "POST" "/api/v1/tables/type_test_text" \
    '{"table_name":"type_test_text","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"content","data_type":"TEXT","nullable":true}]}'

test_sql "STOR-091" "Insert TEXT value" \
    "INSERT INTO type_test_text VALUES (1, 'This is a very long text field that can store large amounts of data without a predefined limit like VARCHAR')"

# STOR-092: BOOLEAN storage
test_rest "STOR-092" "Create BOOLEAN test table" "POST" "/api/v1/tables/type_test_boolean" \
    '{"table_name":"type_test_boolean","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"flag","data_type":"BOOLEAN","nullable":true}]}'

test_sql "STOR-093" "Insert BOOLEAN true" \
    "INSERT INTO type_test_boolean VALUES (1, true)"

test_sql "STOR-094" "Insert BOOLEAN false" \
    "INSERT INTO type_test_boolean VALUES (2, false)"

# STOR-095: BIGINT storage
test_rest "STOR-095" "Create BIGINT test table" "POST" "/api/v1/tables/type_test_bigint" \
    '{"table_name":"type_test_bigint","columns":[{"name":"id","data_type":"BIGINT","nullable":false},{"name":"large_value","data_type":"BIGINT","nullable":true}]}'

test_sql "STOR-096" "Insert BIGINT large value" \
    "INSERT INTO type_test_bigint VALUES (123456789012345, 987654321098765)"

# STOR-097: DOUBLE storage
test_rest "STOR-097" "Create DOUBLE test table" "POST" "/api/v1/tables/type_test_double" \
    '{"table_name":"type_test_double","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"precise_value","data_type":"DOUBLE","nullable":true}]}'

test_sql "STOR-098" "Insert DOUBLE precise value" \
    "INSERT INTO type_test_double VALUES (1, 3.141592653589793238)"

# STOR-099: NULL values
test_rest "STOR-099" "Create NULL test table" "POST" "/api/v1/tables/type_test_null" \
    '{"table_name":"type_test_null","columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"nullable_int","data_type":"INTEGER","nullable":true},{"name":"nullable_text","data_type":"TEXT","nullable":true}]}'

test_sql "STOR-100" "Insert NULL values" \
    "INSERT INTO type_test_null VALUES (1, NULL, 'Text'), (2, 42, NULL)"

echo ""
echo "========================================"
echo "All 100 storage tests completed!"
echo "Results saved to: $RESULTS_FILE"
echo "========================================"

# Print summary
total_tests=100
passed=$(grep -c "PASS" $RESULTS_FILE || echo "0")
failed=$(grep -c "FAIL" $RESULTS_FILE || echo "0")

echo ""
echo "SUMMARY:"
echo "  Total Tests: $total_tests"
echo "  Passed: $passed"
echo "  Failed: $failed"
echo "  Success Rate: $(awk "BEGIN {printf \"%.1f\", ($passed/$total_tests)*100}")%"
