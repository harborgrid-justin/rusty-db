#!/bin/bash

# Resource Manager Testing Script
# Tests all resource management features via REST API and GraphQL

SERVER="http://localhost:8080"
GRAPHQL_URL="$SERVER/graphql"

echo "====================================================================="
echo "RESOURCE MANAGER COMPREHENSIVE TEST REPORT"
echo "Test Date: $(date)"
echo "Server: $SERVER"
echo "====================================================================="
echo ""

# Test counter
TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0

# Helper function to run test
run_test() {
    TEST_COUNT=$((TEST_COUNT + 1))
    local test_id=$1
    local test_name=$2
    local curl_cmd=$3

    echo "---------------------------------------------------------------------"
    echo "TEST: $test_id - $test_name"
    echo "---------------------------------------------------------------------"
    echo "COMMAND: $curl_cmd"
    echo ""

    # Execute command
    local response=$(eval "$curl_cmd" 2>&1)
    local exit_code=$?

    echo "RESPONSE:"
    echo "$response" | head -50
    echo ""

    # Check if response contains error
    if [ $exit_code -eq 0 ] && echo "$response" | grep -q -v '"error"' && echo "$response" | grep -q -v '"errors"'; then
        echo "STATUS: ✓ PASS"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "STATUS: ✗ FAIL"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    echo ""
}

echo "====================================================================="
echo "SECTION 1: CONSUMER GROUPS TESTING"
echo "====================================================================="
echo ""

# RESOURCE-001: List all consumer groups
run_test "RESOURCE-001" "List all consumer groups" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ listConsumerGroups { id name priority category currentSessions } }\"}'"

# RESOURCE-002: Get specific consumer group
run_test "RESOURCE-002" "Get consumer group by ID" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getConsumerGroup(groupId: 1) { id name priority category maxSessions cpuAllocationPct } }\"}'"

# RESOURCE-003: Get consumer group by name
run_test "RESOURCE-003" "Get consumer group by name" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getConsumerGroupByName(name: \\\"INTERACTIVE_GROUP\\\") { id name priority category } }\"}'"

# RESOURCE-004: Create new consumer group
run_test "RESOURCE-004" "Create custom consumer group" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createConsumerGroup(name: \\\"TEST_ANALYTICS_GROUP\\\", priority: 3, category: \\\"ANALYTICS\\\") { groupId } }\"}'"

# RESOURCE-005: Update consumer group configuration
run_test "RESOURCE-005" "Update consumer group CPU allocation" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { updateConsumerGroup(groupId: 2, cpuAllocationPct: 70) { success message } }\"}'"

# RESOURCE-006: Get consumer group statistics
run_test "RESOURCE-006" "Get consumer group statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getConsumerGroupStats(groupId: 2) { groupId groupName currentSessions cpuAllocationPct } }\"}'"

echo "====================================================================="
echo "SECTION 2: RESOURCE PLANS TESTING"
echo "====================================================================="
echo ""

# RESOURCE-007: List all resource plans
run_test "RESOURCE-007" "List all resource plans" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ listResourcePlans { id name description status isEnabled cpuMethod } }\"}'"

# RESOURCE-008: Get specific resource plan
run_test "RESOURCE-008" "Get resource plan by ID" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getResourcePlan(planId: 1) { id name description status cpuMethod parallelExecutionManaged } }\"}'"

# RESOURCE-009: Get resource plan by name
run_test "RESOURCE-009" "Get resource plan by name" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getResourcePlanByName(name: \\\"DEFAULT_PLAN\\\") { id name status isEnabled } }\"}'"

# RESOURCE-010: Create new resource plan
run_test "RESOURCE-010" "Create custom resource plan" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createResourcePlan(name: \\\"PEAK_HOURS_PLAN\\\", cpuMethod: \\\"SHARES\\\") { planId } }\"}'"

# RESOURCE-011: Get active resource plan
run_test "RESOURCE-011" "Get currently active resource plan" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getActiveResourcePlan { id name status } }\"}'"

# RESOURCE-012: Activate resource plan
run_test "RESOURCE-012" "Activate resource plan" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { activateResourcePlan(planId: 1) { success message } }\"}'"

# RESOURCE-013: Get plan directives
run_test "RESOURCE-013" "Get resource plan directives" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getResourcePlanDirectives(planId: 1) { id groupId cpuPct parallelDegreeLimit } }\"}'"

# RESOURCE-014: Create plan directive
run_test "RESOURCE-014" "Create resource plan directive" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createPlanDirective(planId: 1, groupId: 2, cpuPct: 60) { directiveId } }\"}'"

# RESOURCE-015: Validate resource plan
run_test "RESOURCE-015" "Validate resource plan configuration" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ validateResourcePlan(planId: 1) { isValid warnings } }\"}'"

echo "====================================================================="
echo "SECTION 3: CPU SCHEDULING TESTING"
echo "====================================================================="
echo ""

# RESOURCE-016: Get CPU scheduler statistics
run_test "RESOURCE-016" "Get CPU scheduler statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getCpuSchedulerStats { totalScheduled contextSwitches totalCpuTime runawayQueriesDetected } }\"}'"

# RESOURCE-017: Register group with CPU scheduler
run_test "RESOURCE-017" "Register consumer group with CPU scheduler" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { registerCpuGroup(groupId: 2, shares: 2000) { success message } }\"}'"

# RESOURCE-018: Add CPU task
run_test "RESOURCE-018" "Add task to CPU scheduler" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { addCpuTask(queryId: 1001, groupId: 2, priority: 3) { taskId } }\"}'"

# RESOURCE-019: Get CPU group statistics
run_test "RESOURCE-019" "Get CPU group statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getCpuGroupStats(groupId: 2) { groupId shares cpuTimeUsed activeTasks } }\"}'"

# RESOURCE-020: Detect runaway queries
run_test "RESOURCE-020" "Detect runaway queries" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ detectRunawayQueries { taskIds count } }\"}'"

# RESOURCE-021: Rebalance CPU groups
run_test "RESOURCE-021" "Rebalance CPU groups" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { rebalanceCpuGroups { success message } }\"}'"

echo "====================================================================="
echo "SECTION 4: I/O SCHEDULING TESTING"
echo "====================================================================="
echo ""

# RESOURCE-022: Get I/O scheduler statistics
run_test "RESOURCE-022" "Get I/O scheduler statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getIoSchedulerStats { totalRequests completedRequests totalBytesRead totalBytesWritten avgLatencyUs deadlineMisses throttledRequests } }\"}'"

# RESOURCE-023: Register group with I/O scheduler
run_test "RESOURCE-023" "Register consumer group with I/O scheduler" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { registerIoGroup(groupId: 2, bandwidthLimit: 104857600, iopsLimit: 1000, weight: 100) { success message } }\"}'"

# RESOURCE-024: Submit I/O request - Read
run_test "RESOURCE-024" "Submit read I/O request" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { submitIoRequest(groupId: 2, requestType: \\\"READ\\\", priority: \\\"NORMAL\\\", sizeBytes: 4096, offset: 0) { requestId } }\"}'"

# RESOURCE-025: Submit I/O request - Write
run_test "RESOURCE-025" "Submit write I/O request" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { submitIoRequest(groupId: 2, requestType: \\\"WRITE\\\", priority: \\\"HIGH\\\", sizeBytes: 8192, offset: 4096) { requestId } }\"}'"

# RESOURCE-026: Schedule next I/O request
run_test "RESOURCE-026" "Schedule next I/O request" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ scheduleNextIoRequest { requestId scheduled } }\"}'"

# RESOURCE-027: Get I/O group statistics
run_test "RESOURCE-027" "Get I/O group statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getIoGroupStats(groupId: 2) { groupId totalBytes totalOps pendingRequests } }\"}'"

# RESOURCE-028: Update I/O bandwidth metrics
run_test "RESOURCE-028" "Update I/O bandwidth metrics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { updateIoBandwidthMetrics { success message } }\"}'"

echo "====================================================================="
echo "SECTION 5: MEMORY MANAGEMENT TESTING"
echo "====================================================================="
echo ""

# RESOURCE-029: Get memory manager statistics
run_test "RESOURCE-029" "Get memory manager statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getMemoryStats { totalAllocations totalDeallocations failedAllocations peakUsage pressureEvents autoAdjustments } }\"}'"

# RESOURCE-030: List memory pools
run_test "RESOURCE-030" "List all memory pools" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ listMemoryPools { id name poolType maxSize allocatedSize usagePercent } }\"}'"

# RESOURCE-031: Get specific memory pool
run_test "RESOURCE-031" "Get memory pool by ID" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getMemoryPool(poolId: 1) { id name poolType maxSize allocatedSize minSize autoTune } }\"}'"

# RESOURCE-032: Get memory pressure level
run_test "RESOURCE-032" "Get current memory pressure level" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getMemoryPressure { level usagePercent } }\"}'"

# RESOURCE-033: Register group memory limits
run_test "RESOURCE-033" "Register group memory limits" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { registerGroupMemoryLimits(groupId: 2, maxGroupMemory: 2147483648, maxSessionPga: 104857600) { success message } }\"}'"

# RESOURCE-034: Create session memory quota
run_test "RESOURCE-034" "Create session memory quota" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createSessionMemoryQuota(sessionId: 1001, groupId: 2, maxPgaMemory: 104857600) { success message } }\"}'"

# RESOURCE-035: Allocate from memory pool
run_test "RESOURCE-035" "Allocate memory from pool" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { allocateFromMemoryPool(poolId: 2, size: 1048576) { success message } }\"}'"

# RESOURCE-036: Get session memory quota info
run_test "RESOURCE-036" "Get session memory quota information" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getSessionMemoryQuota(sessionId: 1001) { sessionId groupId maxPgaMemory currentPgaUsage peakUsage } }\"}'"

# RESOURCE-037: Auto-tune memory pools
run_test "RESOURCE-037" "Get memory auto-tune recommendations" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ autoTuneMemoryPools { poolId currentSize recommendedSize reason estimatedBenefit } }\"}'"

# RESOURCE-038: Get database memory usage
run_test "RESOURCE-038" "Get total database memory usage" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getDatabaseMemoryUsage { totalMemory usedMemory availableMemory usagePercent } }\"}'"

echo "====================================================================="
echo "SECTION 6: PARALLEL EXECUTION CONTROL TESTING"
echo "====================================================================="
echo ""

# RESOURCE-039: Get parallel execution statistics
run_test "RESOURCE-039" "Get parallel execution statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getParallelExecutionStats { totalParallelQueries totalSerialQueries downgradedQueries queuedQueries avgDopGranted peakTotalDop } }\"}'"

# RESOURCE-040: Create server pool
run_test "RESOURCE-040" "Create parallel server pool" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createServerPool(name: \\\"TEST_POOL\\\", minServers: 4, maxServers: 16) { poolId } }\"}'"

# RESOURCE-041: Request parallel execution - Automatic mode
run_test "RESOURCE-041" "Request parallel execution (automatic mode)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { requestParallelExecution(sessionId: 1001, groupId: 2, requestedDop: 8, mode: \\\"AUTOMATIC\\\", estimatedCost: 100000) { queryId grantedDop } }\"}'"

# RESOURCE-042: Request parallel execution - Manual mode
run_test "RESOURCE-042" "Request parallel execution (manual mode)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { requestParallelExecution(sessionId: 1002, groupId: 2, requestedDop: 4, mode: \\\"MANUAL\\\") { queryId grantedDop } }\"}'"

# RESOURCE-043: Set group DOP limit
run_test "RESOURCE-043" "Set consumer group DOP limit" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { setGroupDopLimit(groupId: 2, limit: 16) { success message } }\"}'"

# RESOURCE-044: Get parallel execution info
run_test "RESOURCE-044" "Get parallel execution information" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getParallelExecution(queryId: 1) { queryId sessionId grantedDop requestedDop state rowsProcessed } }\"}'"

# RESOURCE-045: Update system load for auto DOP
run_test "RESOURCE-045" "Update system load for auto DOP calculation" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { updateSystemLoad(load: 0.65) { success message } }\"}'"

# RESOURCE-046: Complete parallel execution
run_test "RESOURCE-046" "Complete parallel execution" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { completeParallelExecution(queryId: 1) { success message } }\"}'"

echo "====================================================================="
echo "SECTION 7: SESSION CONTROL TESTING"
echo "====================================================================="
echo ""

# RESOURCE-047: Get session controller statistics
run_test "RESOURCE-047" "Get session controller statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getSessionStats { totalSessionsCreated totalSessionsTerminated idleTimeoutTerminations executionTimeoutTerminations currentActiveSessions peakConcurrentSessions } }\"}'"

# RESOURCE-048: Create session
run_test "RESOURCE-048" "Create new session" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createSession(userId: 1001, username: \\\"testuser\\\", groupId: 2) { sessionId } }\"}'"

# RESOURCE-049: Get session information
run_test "RESOURCE-049" "Get session information" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getSession(sessionId: 1) { sessionId username groupId state priority totalQueries } }\"}'"

# RESOURCE-050: List all sessions
run_test "RESOURCE-050" "List all sessions" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ listSessions { sessionId username groupId state priority } }\"}'"

# RESOURCE-051: List active sessions
run_test "RESOURCE-051" "List active sessions" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ listActiveSessions { sessionId username state currentQueryDuration } }\"}'"

# RESOURCE-052: Configure group session pool
run_test "RESOURCE-052" "Configure active session pool for group" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { configureGroupSessionPool(groupId: 2, maxActiveSessions: 50, queueTimeout: 60) { success message } }\"}'"

# RESOURCE-053: Start query
run_test "RESOURCE-053" "Start query (request active session slot)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { startQuery(sessionId: 1) { canStart queued } }\"}'"

# RESOURCE-054: Complete query
run_test "RESOURCE-054" "Complete query (release active session slot)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { completeQuery(sessionId: 1) { success message } }\"}'"

# RESOURCE-055: Set session limits
run_test "RESOURCE-055" "Set session timeout limits" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { setSessionLimits(sessionId: 1, idleTimeout: 3600, maxExecutionTime: 1800) { success message } }\"}'"

# RESOURCE-056: Boost session priority
run_test "RESOURCE-056" "Boost session priority" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { boostSessionPriority(sessionId: 1) { success message newPriority } }\"}'"

# RESOURCE-057: Check idle timeouts
run_test "RESOURCE-057" "Check and terminate idle sessions" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ checkIdleTimeouts { terminatedSessions count } }\"}'"

# RESOURCE-058: Check execution timeouts
run_test "RESOURCE-058" "Check and terminate long-running queries" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ checkExecutionTimeouts { terminatedSessions count } }\"}'"

# RESOURCE-059: Kill session
run_test "RESOURCE-059" "Manually kill session" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { killSession(sessionId: 999) { success message } }\"}'"

echo "====================================================================="
echo "SECTION 8: RESOURCE MANAGER INTEGRATION TESTING"
echo "====================================================================="
echo ""

# RESOURCE-060: Get comprehensive resource stats
run_test "RESOURCE-060" "Get comprehensive resource statistics" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getResourceStats { cpuStats { totalScheduled contextSwitches } ioStats { totalRequests completedRequests } memoryUsagePercent memoryPressure } }\"}'"

# RESOURCE-061: Start monitoring
run_test "RESOURCE-061" "Start resource monitoring" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { startResourceMonitoring { success message } }\"}'"

# RESOURCE-062: Rebalance resources
run_test "RESOURCE-062" "Trigger resource rebalancing" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { rebalanceResources { cpuRebalanced memoryAdjustmentsMade ioMetricsUpdated memoryRecommendations actions } }\"}'"

# RESOURCE-063: Check resource plan schedule
run_test "RESOURCE-063" "Check and switch resource plan based on schedule" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { checkAndSwitchResourcePlan { switched newPlanId newPlanName } }\"}'"

# RESOURCE-064: Check timeouts
run_test "RESOURCE-064" "Check and enforce session timeouts" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ checkTimeouts { idleTimeoutCount executionTimeoutCount terminatedSessions } }\"}'"

# RESOURCE-065: Stop monitoring
run_test "RESOURCE-065" "Stop resource monitoring" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { stopResourceMonitoring { success message } }\"}'"

echo "====================================================================="
echo "SECTION 9: ADVANCED FEATURES TESTING"
echo "====================================================================="
echo ""

# RESOURCE-066: Map user to consumer group
run_test "RESOURCE-066" "Map user to consumer group" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { mapUserToGroup(userId: 1001, username: \\\"testuser\\\", groupId: 2, isPermanent: true) { success message } }\"}'"

# RESOURCE-067: Switch session to different group
run_test "RESOURCE-067" "Switch session to different consumer group" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { switchSessionGroup(sessionId: 1, newGroupId: 3, reason: \\\"WORKLOAD_BASED\\\") { success message } }\"}'"

# RESOURCE-068: Add assignment rule
run_test "RESOURCE-068" "Add consumer group assignment rule" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { addAssignmentRule(name: \\\"BATCH_USERS\\\", priority: 10, condition: \\\"USERNAME\\\", conditionValue: \\\"batch_\\\", targetGroupId: 3) { ruleId } }\"}'"

# RESOURCE-069: Add plan schedule
run_test "RESOURCE-069" "Add time-based plan schedule" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { addPlanSchedule(name: \\\"BUSINESS_HOURS\\\", startHour: 9, endHour: 17, planId: 2, priority: 1) { scheduleId } }\"}'"

# RESOURCE-070: Add maintenance window
run_test "RESOURCE-070" "Add maintenance window" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { addMaintenanceWindow(name: \\\"SUNDAY_MAINTENANCE\\\", dayOfWeek: 0, startHour: 2, durationMinutes: 240, maintenancePlanId: 4) { windowId } }\"}'"

echo "====================================================================="
echo "SECTION 10: ERROR HANDLING & EDGE CASES TESTING"
echo "====================================================================="
echo ""

# RESOURCE-071: Invalid group ID
run_test "RESOURCE-071" "Get non-existent consumer group (error test)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getConsumerGroup(groupId: 99999) { id name } }\"}'"

# RESOURCE-072: Invalid plan ID
run_test "RESOURCE-072" "Get non-existent resource plan (error test)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getResourcePlan(planId: 99999) { id name } }\"}'"

# RESOURCE-073: Invalid session ID
run_test "RESOURCE-073" "Get non-existent session (error test)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"{ getSession(sessionId: 99999) { sessionId username } }\"}'"

# RESOURCE-074: Duplicate group name
run_test "RESOURCE-074" "Create consumer group with duplicate name (error test)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { createConsumerGroup(name: \\\"INTERACTIVE_GROUP\\\", priority: 3, category: \\\"INTERACTIVE\\\") { groupId } }\"}'"

# RESOURCE-075: Exceed memory limit
run_test "RESOURCE-075" "Allocate memory exceeding pool limit (error test)" \
"curl -s -X POST '$GRAPHQL_URL' -H 'Content-Type: application/json' -d '{\"query\":\"mutation { allocateFromMemoryPool(poolId: 2, size: 999999999999) { success message } }\"}'"

echo ""
echo "====================================================================="
echo "TEST SUMMARY"
echo "====================================================================="
echo "Total Tests: $TEST_COUNT"
echo "Passed: $PASS_COUNT"
echo "Failed: $FAIL_COUNT"
echo "Success Rate: $(echo "scale=2; $PASS_COUNT * 100 / $TEST_COUNT" | bc)%"
echo "====================================================================="
echo ""
echo "Note: Many tests may fail if GraphQL schema does not expose these"
echo "specific endpoints. This is expected as the API surface area may"
echo "differ from the internal module capabilities."
echo "====================================================================="
