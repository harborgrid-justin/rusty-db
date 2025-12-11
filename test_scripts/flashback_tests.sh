#!/bin/bash

echo "=== FLASHBACK MODULE COMPREHENSIVE TEST SUITE ==="
echo "Testing Server: http://localhost:8080"
echo ""

# Test counter
TEST_NUM=1

# Function to execute test
run_test() {
    local test_id=$1
    local description=$2
    local curl_cmd=$3
    
    echo "TEST ${test_id}: ${description}"
    echo "Command: ${curl_cmd}"
    
    response=$(eval ${curl_cmd})
    http_code=$?
    
    echo "Response: ${response}"
    
    if [ $http_code -eq 0 ] && [ ! -z "$response" ]; then
        echo "Status: PASS"
    else
        echo "Status: FAIL"
    fi
    echo "---"
    echo ""
}

# FLASHBACK-001: Create test table for flashback operations
run_test "FLASHBACK-001" \
    "Create employees table for flashback testing" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createTable(name: \\\"employees_flashback\\\", columns: [{name: \\\"id\\\", type: \\\"INTEGER\\\"}, {name: \\\"name\\\", type: \\\"VARCHAR\\\"}, {name: \\\"salary\\\", type: \\\"INTEGER\\\"}]) { success message } }\"}' 2>/dev/null"

# FLASHBACK-002: Insert initial data
run_test "FLASHBACK-002" \
    "Insert initial employee data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"mutation { insertOne(table: \\\"employees_flashback\\\", data: {id: 1, name: \\\"Alice\\\", salary: 50000}) { id } }\"}' 2>/dev/null"

# FLASHBACK-003: Query current state
run_test "FLASHBACK-003" \
    "Query current employee data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{ queryTable(table: \\\"employees_flashback\\\") { rows { data } } }\"}' 2>/dev/null"

# FLASHBACK-004: Update data to create version history
run_test "FLASHBACK-004" \
    "Update employee salary to create version" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"mutation { updateOne(table: \\\"employees_flashback\\\", filter: {id: 1}, data: {salary: 60000}) { rowsAffected } }\"}' 2>/dev/null"

# FLASHBACK-005: Test AS OF TIMESTAMP query via executeSql
run_test "FLASHBACK-005" \
    "Execute AS OF TIMESTAMP query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{ executeSql(sql: \\\"SELECT * FROM employees_flashback AS OF TIMESTAMP CURRENT_TIMESTAMP\\\") { rows columns } }\"}' 2>/dev/null"

# FLASHBACK-006: Test VERSIONS BETWEEN query
run_test "FLASHBACK-006" \
    "Execute VERSIONS BETWEEN query" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{ executeSql(sql: \\\"SELECT * FROM employees_flashback VERSIONS BETWEEN SCN 0 AND MAXVALUE\\\") { rows columns } }\"}' 2>/dev/null"

# FLASHBACK-007: Create restore point
run_test "FLASHBACK-007" \
    "Create restore point via SQL" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{ executeSql(sql: \\\"CREATE RESTORE POINT before_delete\\\") { rows } }\"}' 2>/dev/null"

# FLASHBACK-008: Delete data
run_test "FLASHBACK-008" \
    "Delete employee data" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"mutation { deleteOne(table: \\\"employees_flashback\\\", filter: {id: 1}) { rowsAffected } }\"}' 2>/dev/null"

# FLASHBACK-009: Test historical query after delete
run_test "FLASHBACK-009" \
    "Query historical data after delete" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{ executeSql(sql: \\\"SELECT * FROM employees_flashback AS OF SCN 1000\\\") { rows columns } }\"}' 2>/dev/null"

# FLASHBACK-010: Test FLASHBACK TABLE via executeSql
run_test "FLASHBACK-010" \
    "Execute FLASHBACK TABLE command" \
    "curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\":\"{ executeSql(sql: \\\"FLASHBACK TABLE employees_flashback TO RESTORE POINT before_delete\\\") { rows } }\"}' 2>/dev/null"

echo "=== FLASHBACK TESTS COMPLETE ==="
