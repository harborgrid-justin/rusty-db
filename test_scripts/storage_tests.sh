#!/bin/bash

# RustyDB Storage Engine Tests - 100 Tests Total
# Tests execute against real server on localhost:8080 and localhost:5432

BASE_URL="http://localhost:8080"
RESULTS_FILE="/tmp/storage_test_results.txt"
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
        echo "  Response: $response" >> $RESULTS_FILE
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
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"data","data_type":"VARCHAR(255)","nullable":true}]}'

# STOR-002: Insert single row
test_sql "STOR-002" "Insert single row into page storage" \
    "INSERT INTO page_test_001 (id, data) VALUES (1, 'Test Data')"

# STOR-003: Query inserted data
test_sql "STOR-003" "Query data from page storage" \
    "SELECT * FROM page_test_001"

# STOR-004: Insert multiple rows (fill first page)
test_sql "STOR-004" "Insert 100 rows to fill pages" \
    "INSERT INTO page_test_001 (id, data) SELECT generate_series(2, 101), 'Data ' || generate_series(2, 101)"

# STOR-005: Verify row count
test_sql "STOR-005" "Count rows in page storage" \
    "SELECT COUNT(*) FROM page_test_001"

# STOR-006: Create table with all basic data types
test_rest "STOR-006" "Create table with multiple data types" "POST" "/api/v1/tables/page_test_types" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"name","data_type":"VARCHAR(100)","nullable":false},{"name":"price","data_type":"FLOAT","nullable":true},{"name":"active","data_type":"BOOLEAN","nullable":true}]}'

# STOR-007: Insert mixed type data
test_sql "STOR-007" "Insert mixed data types" \
    "INSERT INTO page_test_types (id, name, price, active) VALUES (1, 'Product A', 19.99, true)"

# STOR-008: Query mixed types
test_sql "STOR-008" "Query mixed data types" \
    "SELECT * FROM page_test_types WHERE active = true"

# STOR-009: Update operation (page modification)
test_sql "STOR-009" "Update data in page" \
    "UPDATE page_test_001 SET data = 'Updated Data' WHERE id = 1"

# STOR-010: Delete operation (page modification)
test_sql "STOR-010" "Delete data from page" \
    "DELETE FROM page_test_001 WHERE id = 50"

# STOR-011: Create table with TEXT column (large data)
test_rest "STOR-011" "Create table with TEXT column" "POST" "/api/v1/tables/page_test_large" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"content","data_type":"TEXT","nullable":true}]}'

# STOR-012: Insert large text data
test_sql "STOR-012" "Insert large TEXT data" \
    "INSERT INTO page_test_large (id, content) VALUES (1, 'Large content that spans multiple pages')"

# STOR-013: Create table with BIGINT
test_rest "STOR-013" "Create table with BIGINT" "POST" "/api/v1/tables/page_test_bigint" \
    '{"columns":[{"name":"id","data_type":"BIGINT","nullable":false},{"name":"value","data_type":"BIGINT","nullable":true}]}'

# STOR-014: Insert BIGINT values
test_sql "STOR-014" "Insert BIGINT values" \
    "INSERT INTO page_test_bigint (id, value) VALUES (9223372036854775807, 1234567890123)"

# STOR-015: Create table with DOUBLE
test_rest "STOR-015" "Create table with DOUBLE precision" "POST" "/api/v1/tables/page_test_double" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"DOUBLE","nullable":true}]}'

# STOR-016: Insert DOUBLE values
test_sql "STOR-016" "Insert DOUBLE precision values" \
    "INSERT INTO page_test_double (id, value) VALUES (1, 3.141592653589793)"

# STOR-017: Create table with DATE
test_rest "STOR-017" "Create table with DATE column" "POST" "/api/v1/tables/page_test_date" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"birth_date","data_type":"DATE","nullable":true}]}'

# STOR-018: Insert DATE values
test_sql "STOR-018" "Insert DATE values" \
    "INSERT INTO page_test_date (id, birth_date) VALUES (1, '2024-01-15')"

# STOR-019: Create table with TIMESTAMP
test_rest "STOR-019" "Create table with TIMESTAMP" "POST" "/api/v1/tables/page_test_timestamp" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"created_at","data_type":"TIMESTAMP","nullable":true}]}'

# STOR-020: Insert TIMESTAMP values
test_sql "STOR-020" "Insert TIMESTAMP values" \
    "INSERT INTO page_test_timestamp (id, created_at) VALUES (1, '2024-01-15 14:30:00')"

echo ""
echo "===== PART 2: LSM TREE STORAGE TESTS (STOR-021 to STOR-040) ====="
echo ""

# STOR-021: Create table for LSM testing
test_rest "STOR-021" "Create table for LSM storage" "POST" "/api/v1/tables/lsm_test_001" \
    '{"columns":[{"name":"key","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"VARCHAR(255)","nullable":true}]}'

# STOR-022: Bulk insert for LSM (write-heavy workload)
test_sql "STOR-022" "Bulk insert 500 rows (LSM write test)" \
    "INSERT INTO lsm_test_001 (key, value) SELECT generate_series(1, 500), 'Value ' || generate_series(1, 500)"

# STOR-023: Random updates (LSM modification)
test_sql "STOR-023" "Random updates on LSM storage" \
    "UPDATE lsm_test_001 SET value = 'Updated ' || key WHERE key % 10 = 0"

# STOR-024: Read after many writes
test_sql "STOR-024" "Read performance after writes" \
    "SELECT * FROM lsm_test_001 WHERE key = 250"

# STOR-025: Range scan on LSM
test_sql "STOR-025" "Range scan on LSM storage" \
    "SELECT * FROM lsm_test_001 WHERE key BETWEEN 100 AND 200"

# STOR-026: Multiple small inserts (memtable test)
test_sql "STOR-026" "Insert row 501 (memtable)" \
    "INSERT INTO lsm_test_001 VALUES (501, 'Test 501')"

test_sql "STOR-027" "Insert row 502 (memtable)" \
    "INSERT INTO lsm_test_001 VALUES (502, 'Test 502')"

test_sql "STOR-028" "Insert row 503 (memtable)" \
    "INSERT INTO lsm_test_001 VALUES (503, 'Test 503')"

# STOR-029: Read recently inserted data
test_sql "STOR-029" "Read from memtable" \
    "SELECT * FROM lsm_test_001 WHERE key IN (501, 502, 503)"

# STOR-030: Large batch insert
test_sql "STOR-030" "Large batch insert (1000 rows)" \
    "INSERT INTO lsm_test_001 SELECT generate_series(600, 1599), 'Batch ' || generate_series(600, 1599)"

# STOR-031: Count after bulk operations
test_sql "STOR-031" "Count rows after bulk operations" \
    "SELECT COUNT(*) FROM lsm_test_001"

# STOR-032: Delete operations on LSM
test_sql "STOR-032" "Delete operations (tombstones)" \
    "DELETE FROM lsm_test_001 WHERE key > 1500"

# STOR-033: Query after deletes
test_sql "STOR-033" "Query after deletes" \
    "SELECT * FROM lsm_test_001 WHERE key > 1490"

# STOR-034: Create second LSM table
test_rest "STOR-034" "Create second LSM table" "POST" "/api/v1/tables/lsm_test_002" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"data","data_type":"TEXT","nullable":true}]}'

# STOR-035: Insert with varying sizes
test_sql "STOR-035" "Insert varying size data" \
    "INSERT INTO lsm_test_002 VALUES (1, 'Small'), (2, 'Medium sized data here'), (3, 'Large data with lots of content to test LSM storage efficiency')"

# STOR-036: Update with size change
test_sql "STOR-036" "Update changing data size" \
    "UPDATE lsm_test_002 SET data = 'Very large updated content' WHERE id = 1"

# STOR-037: Concurrent inserts simulation
test_sql "STOR-037" "Batch insert set 1" \
    "INSERT INTO lsm_test_002 SELECT generate_series(10, 109), 'Data ' || generate_series(10, 109)"

test_sql "STOR-038" "Batch insert set 2" \
    "INSERT INTO lsm_test_002 SELECT generate_series(110, 209), 'Data ' || generate_series(110, 209)"

# STOR-039: Read mixed data
test_sql "STOR-039" "Read mixed data sizes" \
    "SELECT * FROM lsm_test_002 WHERE id < 10"

# STOR-040: Aggregate on LSM storage
test_sql "STOR-040" "Aggregate query on LSM" \
    "SELECT COUNT(*), MIN(id), MAX(id) FROM lsm_test_002"

echo ""
echo "===== PART 3: COLUMNAR STORAGE TESTS (STOR-041 to STOR-060) ====="
echo ""

# STOR-041: Create table for columnar storage
test_rest "STOR-041" "Create columnar analytics table" "POST" "/api/v1/tables/columnar_test_001" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"category","data_type":"VARCHAR(50)","nullable":true},{"name":"amount","data_type":"FLOAT","nullable":true},{"name":"quantity","data_type":"INTEGER","nullable":true}]}'

# STOR-042: Insert analytics data
test_sql "STOR-042" "Insert analytics data" \
    "INSERT INTO columnar_test_001 (id, category, amount, quantity) VALUES (1, 'A', 100.50, 10), (2, 'B', 200.75, 20), (3, 'A', 150.25, 15)"

# STOR-043: Column-wise aggregate (SUM)
test_sql "STOR-043" "SUM aggregate on column" \
    "SELECT SUM(amount) FROM columnar_test_001"

# STOR-044: Column-wise aggregate (AVG)
test_sql "STOR-044" "AVG aggregate on column" \
    "SELECT AVG(quantity) FROM columnar_test_001"

# STOR-045: Grouped aggregation
test_sql "STOR-045" "GROUP BY on columnar storage" \
    "SELECT category, SUM(amount) FROM columnar_test_001 GROUP BY category"

# STOR-046: Insert larger dataset
test_sql "STOR-046" "Bulk insert for analytics (500 rows)" \
    "INSERT INTO columnar_test_001 SELECT generate_series(4, 503), 'Cat_' || (generate_series(4, 503) % 10), (generate_series(4, 503) * 1.5), generate_series(4, 503)"

# STOR-047: Column scan performance
test_sql "STOR-047" "Full column scan (COUNT)" \
    "SELECT COUNT(*) FROM columnar_test_001"

# STOR-048: Selective column read
test_sql "STOR-048" "Select specific columns only" \
    "SELECT category, amount FROM columnar_test_001 WHERE id < 10"

# STOR-049: Multi-column aggregation
test_sql "STOR-050" "Multi-column aggregate" \
    "SELECT category, COUNT(*), SUM(amount), AVG(quantity) FROM columnar_test_001 GROUP BY category"

# STOR-051: Create second columnar table
test_rest "STOR-051" "Create sales columnar table" "POST" "/api/v1/tables/columnar_sales" \
    '{"columns":[{"name":"sale_id","data_type":"INTEGER","nullable":false},{"name":"product","data_type":"VARCHAR(100)","nullable":true},{"name":"price","data_type":"DOUBLE","nullable":true},{"name":"sold","data_type":"INTEGER","nullable":true},{"name":"revenue","data_type":"DOUBLE","nullable":true}]}'

# STOR-052: Insert sales data
test_sql "STOR-052" "Insert sales records" \
    "INSERT INTO columnar_sales VALUES (1, 'Widget', 19.99, 100, 1999.00), (2, 'Gadget', 29.99, 50, 1499.50), (3, 'Doohickey', 9.99, 200, 1998.00)"

# STOR-053: Revenue calculation
test_sql "STOR-053" "Calculate total revenue" \
    "SELECT SUM(revenue) FROM columnar_sales"

# STOR-054: Top products by revenue
test_sql "STOR-054" "Top products by revenue" \
    "SELECT product, revenue FROM columnar_sales ORDER BY revenue DESC"

# STOR-055: Bulk sales data
test_sql "STOR-055" "Bulk insert sales data" \
    "INSERT INTO columnar_sales SELECT generate_series(10, 509), 'Product_' || generate_series(10, 509), (generate_series(10, 509) * 0.99), generate_series(10, 509), (generate_series(10, 509) * 0.99 * generate_series(10, 509))"

# STOR-056: OLAP-style query (aggregation with filter)
test_sql "STOR-056" "OLAP query with filter" \
    "SELECT product, SUM(sold), SUM(revenue) FROM columnar_sales WHERE price > 10 GROUP BY product"

# STOR-057: Column compression test (repeating values)
test_rest "STOR-057" "Create table for compression test" "POST" "/api/v1/tables/columnar_compress" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"status","data_type":"VARCHAR(20)","nullable":true},{"name":"value","data_type":"INTEGER","nullable":true}]}'

# STOR-058: Insert repeating values (good for compression)
test_sql "STOR-058" "Insert repeating values" \
    "INSERT INTO columnar_compress SELECT generate_series(1, 1000), 'ACTIVE', generate_series(1, 1000) % 100"

# STOR-059: Query compressed column
test_sql "STOR-059" "Query on compressed column" \
    "SELECT status, COUNT(*) FROM columnar_compress GROUP BY status"

# STOR-060: Range query on columnar
test_sql "STOR-060" "Range query on columnar storage" \
    "SELECT * FROM columnar_compress WHERE value BETWEEN 50 AND 60"

echo ""
echo "===== PART 4: BUFFER POOL TESTS (STOR-061 to STOR-080) ====="
echo ""

# STOR-061: Get initial metrics
test_rest "STOR-061" "Get initial buffer pool metrics" "GET" "/api/v1/metrics" ""

# STOR-062: Create table for buffer pool test
test_rest "STOR-062" "Create table for buffer test" "POST" "/api/v1/tables/buffer_test_001" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"data","data_type":"VARCHAR(1000)","nullable":true}]}'

# STOR-063: Insert data to fill buffer
test_sql "STOR-063" "Insert 200 rows (buffer fill)" \
    "INSERT INTO buffer_test_001 SELECT generate_series(1, 200), 'Data ' || REPEAT('X', 900)"

# STOR-064: Sequential scan (buffer pool utilization)
test_sql "STOR-064" "Sequential scan (buffer pool)" \
    "SELECT COUNT(*) FROM buffer_test_001"

# STOR-065: Random access pattern
test_sql "STOR-065" "Random access pattern" \
    "SELECT * FROM buffer_test_001 WHERE id = 100"

test_sql "STOR-066" "Random access 2" \
    "SELECT * FROM buffer_test_001 WHERE id = 50"

test_sql "STOR-067" "Random access 3" \
    "SELECT * FROM buffer_test_001 WHERE id = 150"

# STOR-068: Get metrics after operations
test_rest "STOR-068" "Get buffer metrics after ops" "GET" "/api/v1/metrics" ""

# STOR-069: Create multiple tables (buffer pressure)
test_rest "STOR-069" "Create buffer test table 2" "POST" "/api/v1/tables/buffer_test_002" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"TEXT","nullable":true}]}'

test_rest "STOR-070" "Create buffer test table 3" "POST" "/api/v1/tables/buffer_test_003" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"TEXT","nullable":true}]}'

# STOR-071: Insert into multiple tables
test_sql "STOR-071" "Insert into buffer_test_002" \
    "INSERT INTO buffer_test_002 SELECT generate_series(1, 100), 'Value ' || generate_series(1, 100)"

test_sql "STOR-072" "Insert into buffer_test_003" \
    "INSERT INTO buffer_test_003 SELECT generate_series(1, 100), 'Value ' || generate_series(1, 100)"

# STOR-073: Cross-table queries (buffer switching)
test_sql "STOR-073" "Query buffer_test_001" \
    "SELECT COUNT(*) FROM buffer_test_001"

test_sql "STOR-074" "Query buffer_test_002" \
    "SELECT COUNT(*) FROM buffer_test_002"

test_sql "STOR-075" "Query buffer_test_003" \
    "SELECT COUNT(*) FROM buffer_test_003"

# STOR-076: Large data insertion (buffer eviction test)
test_sql "STOR-076" "Large data insertion" \
    "INSERT INTO buffer_test_001 SELECT generate_series(201, 400), REPEAT('Y', 990)"

# STOR-077: Access old data (may require reload from disk)
test_sql "STOR-077" "Access potentially evicted data" \
    "SELECT * FROM buffer_test_001 WHERE id = 1"

# STOR-078: Sequential scan after eviction
test_sql "STOR-078" "Full scan after eviction" \
    "SELECT COUNT(*) FROM buffer_test_001"

# STOR-079: Get final buffer metrics
test_rest "STOR-079" "Get final buffer pool metrics" "GET" "/api/v1/metrics" ""

# STOR-080: Update operations (dirty page management)
test_sql "STOR-080" "Updates (dirty pages)" \
    "UPDATE buffer_test_001 SET data = 'Updated' WHERE id % 10 = 0"

echo ""
echo "===== PART 5: DATA TYPES STORAGE TESTS (STOR-081 to STOR-100) ====="
echo ""

# STOR-081: INTEGER storage
test_rest "STOR-081" "Create INTEGER test table" "POST" "/api/v1/tables/type_test_integer" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"INTEGER","nullable":true}]}'

test_sql "STOR-082" "Insert INTEGER values" \
    "INSERT INTO type_test_integer VALUES (1, 2147483647), (2, -2147483648), (3, 0), (4, 42)"

# STOR-083: FLOAT storage
test_rest "STOR-083" "Create FLOAT test table" "POST" "/api/v1/tables/type_test_float" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"value","data_type":"FLOAT","nullable":true}]}'

test_sql "STOR-084" "Insert FLOAT values" \
    "INSERT INTO type_test_float VALUES (1, 3.14159), (2, -2.71828), (3, 0.0), (4, 1.23456e10)"

# STOR-085: VARCHAR storage (various lengths)
test_rest "STOR-085" "Create VARCHAR test table" "POST" "/api/v1/tables/type_test_varchar" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"short_text","data_type":"VARCHAR(10)","nullable":true},{"name":"medium_text","data_type":"VARCHAR(100)","nullable":true},{"name":"long_text","data_type":"VARCHAR(500)","nullable":true}]}'

test_sql "STOR-086" "Insert VARCHAR values" \
    "INSERT INTO type_test_varchar VALUES (1, 'Short', 'Medium length text here', 'Long text with lots of characters to test VARCHAR storage')"

# STOR-087: TEXT storage
test_rest "STOR-087" "Create TEXT test table" "POST" "/api/v1/tables/type_test_text" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"content","data_type":"TEXT","nullable":true}]}'

test_sql "STOR-088" "Insert TEXT values" \
    "INSERT INTO type_test_text VALUES (1, 'This is a very long text field that can store large amounts of data without a predefined limit like VARCHAR')"

# STOR-089: BOOLEAN storage
test_rest "STOR-089" "Create BOOLEAN test table" "POST" "/api/v1/tables/type_test_boolean" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"flag","data_type":"BOOLEAN","nullable":true}]}'

test_sql "STOR-090" "Insert BOOLEAN values" \
    "INSERT INTO type_test_boolean VALUES (1, true), (2, false), (3, true)"

# STOR-091: DATE storage
test_rest "STOR-091" "Create DATE comprehensive table" "POST" "/api/v1/tables/type_test_date_full" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"event_date","data_type":"DATE","nullable":true}]}'

test_sql "STOR-092" "Insert DATE values" \
    "INSERT INTO type_test_date_full VALUES (1, '2024-01-01'), (2, '1999-12-31'), (3, '2025-06-15')"

# STOR-093: TIMESTAMP storage
test_rest "STOR-093" "Create TIMESTAMP comprehensive table" "POST" "/api/v1/tables/type_test_timestamp_full" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"created_at","data_type":"TIMESTAMP","nullable":true},{"name":"updated_at","data_type":"TIMESTAMP","nullable":true}]}'

test_sql "STOR-094" "Insert TIMESTAMP values" \
    "INSERT INTO type_test_timestamp_full VALUES (1, '2024-01-15 10:30:00', '2024-01-15 14:45:30')"

# STOR-095: BIGINT storage
test_rest "STOR-095" "Create BIGINT comprehensive table" "POST" "/api/v1/tables/type_test_bigint_full" \
    '{"columns":[{"name":"id","data_type":"BIGINT","nullable":false},{"name":"large_value","data_type":"BIGINT","nullable":true}]}'

test_sql "STOR-096" "Insert BIGINT values" \
    "INSERT INTO type_test_bigint_full VALUES (9223372036854775807, 1234567890123456789)"

# STOR-097: DOUBLE storage
test_rest "STOR-097" "Create DOUBLE comprehensive table" "POST" "/api/v1/tables/type_test_double_full" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"precise_value","data_type":"DOUBLE","nullable":true}]}'

test_sql "STOR-098" "Insert DOUBLE values" \
    "INSERT INTO type_test_double_full VALUES (1, 3.141592653589793238), (2, 2.718281828459045235)"

# STOR-099: Mixed NULL values
test_rest "STOR-099" "Create NULL test table" "POST" "/api/v1/tables/type_test_null" \
    '{"columns":[{"name":"id","data_type":"INTEGER","nullable":false},{"name":"nullable_int","data_type":"INTEGER","nullable":true},{"name":"nullable_text","data_type":"TEXT","nullable":true},{"name":"nullable_bool","data_type":"BOOLEAN","nullable":true}]}'

test_sql "STOR-100" "Insert NULL values" \
    "INSERT INTO type_test_null VALUES (1, NULL, 'Text', true), (2, 42, NULL, false), (3, 100, 'Data', NULL)"

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
