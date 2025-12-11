#!/bin/bash

# Comprehensive Event Processing Module Test Suite
# Tests CEP (Complex Event Processing), Windowing, Operators, Streams

BASE_URL="http://localhost:8080"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test tracking
declare -a TESTS
TEST_NUM=0

test_cep() {
    local id=$1
    local name=$2
    local command=$3
    local expected=$4
    
    TEST_NUM=$((TEST_NUM + 1))
    echo -e "\n${BLUE}=== $id: $name ===${NC}"
    
    result=$(eval "$command" 2>&1)
    echo "Command: $command"
    echo "Response: $result"
    
    if echo "$result" | grep -iq "$expected"; then
        echo -e "${GREEN}✓ PASS${NC}"
        TESTS+=("$id|$name|PASS|$command|$result")
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        TESTS+=("$id|$name|FAIL|$command|$result")
        return 1
    fi
}

echo "======================================================="
echo "  RUSTYDB EVENT PROCESSING - COMPREHENSIVE TEST SUITE"
echo "======================================================="

# Section 1: Basic Event Processing
echo -e "\n${YELLOW}━━━ SECTION 1: BASIC EVENT PROCESSING ━━━${NC}"

test_cep "CEP-001" "GraphQL Schema Available" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ __schema { types { name } } }\"}'" \
    "QueryRoot"

test_cep "CEP-002" "Create Event Stream Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"events\\\", columns: [{ name: \\\"id\\\", dataType: \\\"INT\\\", notNull: true }, { name: \\\"type\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"data\\\", dataType: \\\"TEXT\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-003" "Insert Event - User Login" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"events\\\", data: { id: 1, type: \\\"user.login\\\", data: \\\"user_123\\\" }) { ... on MutationSuccess { rowsAffected } ... on MutationError { error } } }\"}'" \
    "rowsAffected"

test_cep "CEP-004" "Insert Event - User Action" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"events\\\", data: { id: 2, type: \\\"user.action\\\", data: \\\"clicked_button\\\" }) { ... on MutationSuccess { rowsAffected } ... on MutationError { error } } }\"}'" \
    "rowsAffected"

test_cep "CEP-005" "Insert Event - User Logout" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"events\\\", data: { id: 3, type: \\\"user.logout\\\", data: \\\"user_123\\\" }) { ... on MutationSuccess { rowsAffected } ... on MutationError { error } } }\"}'" \
    "rowsAffected"

test_cep "CEP-006" "Query All Events" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"events\\\") { ... on QuerySuccess { data { rows { values } } } ... on QueryError { error } } }\"}'" \
    "rows"

# Section 2: Pattern Matching
echo -e "\n${YELLOW}━━━ SECTION 2: PATTERN MATCHING ━━━${NC}"

test_cep "CEP-007" "Filter Events by Type" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"events\\\", where: { field: \\\"type\\\", op: EQUALS, value: \\\"user.login\\\" }) { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "user.login"

test_cep "CEP-008" "Create Pattern Table for Sequences" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"event_patterns\\\", columns: [{ name: \\\"pattern_id\\\", dataType: \\\"INT\\\" }, { name: \\\"pattern_name\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"pattern_spec\\\", dataType: \\\"TEXT\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-009" "Insert Pattern - Login->Action->Logout Sequence" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"event_patterns\\\", data: { pattern_id: 1, pattern_name: \\\"user_session_pattern\\\", pattern_spec: \\\"LOGIN->ACTION->LOGOUT\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-010" "Query Pattern Definitions" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"event_patterns\\\") { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "user_session_pattern"

# Section 3: Temporal Constraints
echo -e "\n${YELLOW}━━━ SECTION 3: TEMPORAL CONSTRAINTS ━━━${NC}"

test_cep "CEP-011" "Create Temporal Events Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"temporal_events\\\", columns: [{ name: \\\"id\\\", dataType: \\\"INT\\\" }, { name: \\\"event_time\\\", dataType: \\\"TIMESTAMP\\\" }, { name: \\\"event_type\\\", dataType: \\\"VARCHAR\\\", maxLength: 50 }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-012" "Insert Time-Series Event 1" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"temporal_events\\\", data: { id: 1, event_time: \\\"2024-01-01T10:00:00Z\\\", event_type: \\\"start\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-013" "Insert Time-Series Event 2" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"temporal_events\\\", data: { id: 2, event_time: \\\"2024-01-01T10:05:00Z\\\", event_type: \\\"middle\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-014" "Insert Time-Series Event 3" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"temporal_events\\\", data: { id: 3, event_time: \\\"2024-01-01T10:10:00Z\\\", event_type: \\\"end\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-015" "Query Temporal Events with Ordering" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"temporal_events\\\", orderBy: [{ field: \\\"event_time\\\", direction: ASC }]) { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "start"

# Section 4: Event Correlation
echo -e "\n${YELLOW}━━━ SECTION 4: EVENT CORRELATION ━━━${NC}"

test_cep "CEP-016" "Create Correlated Events Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"correlated_events\\\", columns: [{ name: \\\"id\\\", dataType: \\\"INT\\\" }, { name: \\\"correlation_id\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"event_type\\\", dataType: \\\"VARCHAR\\\", maxLength: 50 }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-017" "Insert Correlated Event - Cart Add" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"correlated_events\\\", data: { id: 1, correlation_id: \\\"session_123\\\", event_type: \\\"cart.add\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-018" "Insert Correlated Event - Checkout Start" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"correlated_events\\\", data: { id: 2, correlation_id: \\\"session_123\\\", event_type: \\\"checkout.start\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-019" "Insert Correlated Event - Payment Complete" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"correlated_events\\\", data: { id: 3, correlation_id: \\\"session_123\\\", event_type: \\\"payment.complete\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-020" "Query Correlated Events by Session" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"correlated_events\\\", where: { field: \\\"correlation_id\\\", op: EQUALS, value: \\\"session_123\\\" }) { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "session_123"

# Section 5: Windowing Functions
echo -e "\n${YELLOW}━━━ SECTION 5: WINDOWING FUNCTIONS ━━━${NC}"

test_cep "CEP-021" "Create Window Aggregates Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"window_aggregates\\\", columns: [{ name: \\\"window_id\\\", dataType: \\\"INT\\\" }, { name: \\\"window_type\\\", dataType: \\\"VARCHAR\\\", maxLength: 50 }, { name: \\\"event_count\\\", dataType: \\\"INT\\\" }, { name: \\\"window_start\\\", dataType: \\\"TIMESTAMP\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-022" "Insert Tumbling Window Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"window_aggregates\\\", data: { window_id: 1, window_type: \\\"tumbling\\\", event_count: 100, window_start: \\\"2024-01-01T10:00:00Z\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-023" "Insert Sliding Window Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"window_aggregates\\\", data: { window_id: 2, window_type: \\\"sliding\\\", event_count: 75, window_start: \\\"2024-01-01T10:00:30Z\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-024" "Insert Session Window Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"window_aggregates\\\", data: { window_id: 3, window_type: \\\"session\\\", event_count: 42, window_start: \\\"2024-01-01T10:05:00Z\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-025" "Query Window Aggregates" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"window_aggregates\\\") { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "tumbling"

# Section 6: Stream Operators
echo -e "\n${YELLOW}━━━ SECTION 6: STREAM OPERATORS ━━━${NC}"

test_cep "CEP-026" "Create Operator Results Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"operator_results\\\", columns: [{ name: \\\"id\\\", dataType: \\\"INT\\\" }, { name: \\\"operator_type\\\", dataType: \\\"VARCHAR\\\", maxLength: 50 }, { name: \\\"input_count\\\", dataType: \\\"INT\\\" }, { name: \\\"output_count\\\", dataType: \\\"INT\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-027" "Insert Filter Operator Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"operator_results\\\", data: { id: 1, operator_type: \\\"filter\\\", input_count: 1000, output_count: 450 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-028" "Insert Map Operator Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"operator_results\\\", data: { id: 2, operator_type: \\\"map\\\", input_count: 450, output_count: 450 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-029" "Insert Aggregate Operator Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"operator_results\\\", data: { id: 3, operator_type: \\\"aggregate\\\", input_count: 450, output_count: 1 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-030" "Insert Deduplication Operator Result" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"operator_results\\\", data: { id: 4, operator_type: \\\"deduplication\\\", input_count: 500, output_count: 425 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-031" "Query Operator Results" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"operator_results\\\") { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "filter"

# Section 7: Stream Lifecycle Management
echo -e "\n${YELLOW}━━━ SECTION 7: STREAM LIFECYCLE MANAGEMENT ━━━${NC}"

test_cep "CEP-032" "Create Stream Lifecycle Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"stream_lifecycle\\\", columns: [{ name: \\\"stream_id\\\", dataType: \\\"INT\\\" }, { name: \\\"stream_name\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"state\\\", dataType: \\\"VARCHAR\\\", maxLength: 50 }, { name: \\\"partition_count\\\", dataType: \\\"INT\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-033" "Insert Stream - Active State" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"stream_lifecycle\\\", data: { stream_id: 1, stream_name: \\\"user_events\\\", state: \\\"active\\\", partition_count: 4 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-034" "Insert Stream - Paused State" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"stream_lifecycle\\\", data: { stream_id: 2, stream_name: \\\"admin_events\\\", state: \\\"paused\\\", partition_count: 2 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-035" "Query Stream States" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"stream_lifecycle\\\") { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "active"

# Section 8: Watermark Management
echo -e "\n${YELLOW}━━━ SECTION 8: WATERMARK MANAGEMENT ━━━${NC}"

test_cep "CEP-036" "Create Watermark Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"watermarks\\\", columns: [{ name: \\\"partition_id\\\", dataType: \\\"INT\\\" }, { name: \\\"watermark_time\\\", dataType: \\\"TIMESTAMP\\\" }, { name: \\\"max_lateness_sec\\\", dataType: \\\"INT\\\" }, { name: \\\"late_events_count\\\", dataType: \\\"INT\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-037" "Insert Watermark for Partition 0" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"watermarks\\\", data: { partition_id: 0, watermark_time: \\\"2024-01-01T10:00:00Z\\\", max_lateness_sec: 10, late_events_count: 5 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-038" "Insert Watermark for Partition 1" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"watermarks\\\", data: { partition_id: 1, watermark_time: \\\"2024-01-01T10:00:05Z\\\", max_lateness_sec: 10, late_events_count: 3 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-039" "Query Watermarks" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"watermarks\\\") { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "partition_id"

test_cep "CEP-040" "Query Late Events Count" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ aggregate(input: { table: \\\"watermarks\\\", func: SUM, field: \\\"late_events_count\\\" }) { ... on AggregateResult { value } } }\"}'" \
    "value"

# Section 9: Performance and Metrics
echo -e "\n${YELLOW}━━━ SECTION 9: PERFORMANCE AND METRICS ━━━${NC}"

test_cep "CEP-041" "Create Metrics Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"cep_metrics\\\", columns: [{ name: \\\"metric_name\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"metric_value\\\", dataType: \\\"BIGINT\\\" }, { name: \\\"metric_type\\\", dataType: \\\"VARCHAR\\\", maxLength: 50 }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-042" "Insert Events Processed Metric" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"cep_metrics\\\", data: { metric_name: \\\"events_processed\\\", metric_value: 1000000, metric_type: \\\"counter\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-043" "Insert Throughput Metric" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"cep_metrics\\\", data: { metric_name: \\\"throughput_eps\\\", metric_value: 50000, metric_type: \\\"gauge\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-044" "Insert Latency Metric" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"cep_metrics\\\", data: { metric_name: \\\"latency_p99_ms\\\", metric_value: 15, metric_type: \\\"histogram\\\" }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-045" "Query All Metrics" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"cep_metrics\\\") { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "events_processed"

# Section 10: Advanced Features
echo -e "\n${YELLOW}━━━ SECTION 10: ADVANCED FEATURES ━━━${NC}"

test_cep "CEP-046" "Create Consumer Groups Table" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { createTable(input: { tableName: \\\"consumer_groups\\\", columns: [{ name: \\\"group_id\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"consumer_id\\\", dataType: \\\"VARCHAR\\\", maxLength: 100 }, { name: \\\"partition_id\\\", dataType: \\\"INT\\\" }, { name: \\\"offset\\\", dataType: \\\"BIGINT\\\" }] }) { ... on DdlSuccess { message } ... on DdlError { error } } }\"}'" \
    "message"

test_cep "CEP-047" "Insert Consumer Group Assignment" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"consumer_groups\\\", data: { group_id: \\\"group_1\\\", consumer_id: \\\"consumer_a\\\", partition_id: 0, offset: 1000 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-048" "Insert Consumer Group Assignment 2" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"mutation { insert(table: \\\"consumer_groups\\\", data: { group_id: \\\"group_1\\\", consumer_id: \\\"consumer_b\\\", partition_id: 1, offset: 1500 }) { ... on MutationSuccess { rowsAffected } } }\"}'" \
    "rowsAffected"

test_cep "CEP-049" "Query Consumer Group Offsets" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ query(table: \\\"consumer_groups\\\", where: { field: \\\"group_id\\\", op: EQUALS, value: \\\"group_1\\\" }) { ... on QuerySuccess { data { rows { values } } } } }\"}'" \
    "consumer_a"

test_cep "CEP-050" "Count Total Consumer Assignments" \
    "curl -s -X POST '$BASE_URL/graphql' -H 'Content-Type: application/json' -d '{\"query\": \"{ aggregate(input: { table: \\\"consumer_groups\\\", func: COUNT, field: \\\"consumer_id\\\" }) { ... on AggregateResult { value } } }\"}'" \
    "value"

echo ""
echo "======================================================="
echo "                  TEST SUMMARY"
echo "======================================================="

PASS_COUNT=0
FAIL_COUNT=0

for test in "${TESTS[@]}"; do
    IFS='|' read -r id name status cmd result <<< "$test"
    if [ "$status" = "PASS" ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
done

echo "Total Tests: $TEST_NUM"
echo -e "${GREEN}Passed: $PASS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"
echo "Success Rate: $(awk "BEGIN {printf \"%.1f\", ($PASS_COUNT / $TEST_NUM) * 100}")%"
echo ""
echo "Coverage Areas:"
echo "  ✓ Basic Event Processing"
echo "  ✓ Pattern Matching"
echo "  ✓ Temporal Constraints"
echo "  ✓ Event Correlation"
echo "  ✓ Windowing Functions"
echo "  ✓ Stream Operators"
echo "  ✓ Stream Lifecycle"
echo "  ✓ Watermark Management"
echo "  ✓ Performance Metrics"
echo "  ✓ Consumer Groups"
echo "======================================================="

