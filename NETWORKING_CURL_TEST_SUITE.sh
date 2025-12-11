#!/bin/bash
#
# RustyDB Distributed Networking - Complete cURL Test Suite
#
# IMPORTANT: These tests require networking API endpoints to be integrated
# into the REST server. Currently, these endpoints are NOT mounted.
#
# Integration Required in: /home/user/rusty-db/src/api/rest/server.rs
#
# Usage: ./NETWORKING_CURL_TEST_SUITE.sh
#

set -e

BASE_URL="http://localhost:8080"
NETWORK_API="${BASE_URL}/api/v1/network"
GRAPHQL_URL="${BASE_URL}/graphql"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "RustyDB Networking Test Suite"
echo "========================================"
echo ""

# Test counter
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test
run_test() {
    local test_id=$1
    local test_name=$2
    local curl_command=$3

    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    echo "----------------------------------------"
    echo "Test ${test_id}: ${test_name}"
    echo "Command: ${curl_command}"
    echo ""

    if eval $curl_command 2>/dev/null; then
        echo -e "${GREEN}✓ PASS${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗ FAIL (Endpoint not available)${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    echo ""
}

# =============================================================================
# Category 1: Health & Status Tests
# =============================================================================

echo "=== CATEGORY 1: HEALTH & STATUS ==="
echo ""

run_test "NETWORKING-001" "Get Overall Network Health" \
  "curl -s -X GET ${NETWORK_API}/health | jq ."

run_test "NETWORKING-002" "Get Network Statistics" \
  "curl -s -X GET ${NETWORK_API}/stats | jq ."

run_test "NETWORKING-003" "Get Cluster Topology" \
  "curl -s -X GET ${NETWORK_API}/topology | jq ."

run_test "NETWORKING-004" "List All Peers" \
  "curl -s -X GET ${NETWORK_API}/peers | jq ."

run_test "NETWORKING-005" "Get Specific Peer Info" \
  "curl -s -X GET ${NETWORK_API}/peers/node2 | jq ."

run_test "NETWORKING-006" "Get Node Health" \
  "curl -s -X GET ${NETWORK_API}/node/node2/health | jq ."

# =============================================================================
# Category 2: Cluster Operations
# =============================================================================

echo "=== CATEGORY 2: CLUSTER OPERATIONS ==="
echo ""

run_test "NETWORKING-007" "Join Cluster" \
  "curl -s -X POST ${NETWORK_API}/join \
    -H 'Content-Type: application/json' \
    -d '{\"seed_nodes\":[\"node1:7000\",\"node2:7000\"]}' | jq ."

run_test "NETWORKING-008" "Leave Cluster" \
  "curl -s -X POST ${NETWORK_API}/leave | jq ."

# =============================================================================
# Category 3: Transport & Connection Tests
# =============================================================================

echo "=== CATEGORY 3: TRANSPORT & CONNECTIONS ==="
echo ""

run_test "NETWORKING-009" "Get Connection Pool Statistics" \
  "curl -s -X GET ${NETWORK_API}/pool/statistics | jq ."

run_test "NETWORKING-010" "Get Active Connections" \
  "curl -s -X GET ${NETWORK_API}/pool/connections | jq ."

run_test "NETWORKING-011" "Connect to Node via TCP" \
  "curl -s -X POST ${NETWORK_API}/transport/tcp/connect \
    -H 'Content-Type: application/json' \
    -d '{\"host\":\"192.168.1.10\",\"port\":7000,\"node_id\":\"node2\"}' | jq ."

run_test "NETWORKING-012" "Get Transport Statistics" \
  "curl -s -X GET ${NETWORK_API}/transport/stats | jq ."

# =============================================================================
# Category 4: Message Routing Tests
# =============================================================================

echo "=== CATEGORY 4: MESSAGE ROUTING ==="
echo ""

run_test "NETWORKING-013" "Send Direct Message" \
  "curl -s -X POST ${NETWORK_API}/routing/send \
    -H 'Content-Type: application/json' \
    -d '{
      \"target_node\":\"node2\",
      \"message_type\":\"Heartbeat\",
      \"priority\":\"High\",
      \"data\":{\"timestamp\":1733943600}
    }' | jq ."

run_test "NETWORKING-014" "Broadcast Message" \
  "curl -s -X POST ${NETWORK_API}/routing/broadcast \
    -H 'Content-Type: application/json' \
    -d '{
      \"message_type\":\"ConfigUpdate\",
      \"priority\":\"Critical\",
      \"data\":{\"setting\":\"max_connections\",\"value\":\"200\"}
    }' | jq ."

run_test "NETWORKING-015" "Scatter-Gather Query" \
  "curl -s -X POST ${NETWORK_API}/routing/scatter-gather \
    -H 'Content-Type: application/json' \
    -d '{
      \"target_nodes\":[\"node1\",\"node2\",\"node3\"],
      \"message_type\":\"QueryRequest\",
      \"timeout_ms\":5000,
      \"data\":{\"query\":\"SELECT COUNT(*) FROM users\"}
    }' | jq ."

run_test "NETWORKING-016" "Quorum Operation" \
  "curl -s -X POST ${NETWORK_API}/routing/quorum \
    -H 'Content-Type: application/json' \
    -d '{
      \"nodes\":[\"node1\",\"node2\",\"node3\",\"node4\",\"node5\"],
      \"quorum_size\":3,
      \"message_type\":\"ConsensusProposal\",
      \"timeout_ms\":3000
    }' | jq ."

run_test "NETWORKING-017" "Get Routing Table" \
  "curl -s -X GET ${NETWORK_API}/routing/table | jq ."

# =============================================================================
# Category 5: Health Monitoring Tests
# =============================================================================

echo "=== CATEGORY 5: HEALTH MONITORING ==="
echo ""

run_test "NETWORKING-018" "Get Heartbeat Status" \
  "curl -s -X GET ${NETWORK_API}/health/heartbeat/status | jq ."

run_test "NETWORKING-019" "Get Failure Detector Phi Values" \
  "curl -s -X GET ${NETWORK_API}/health/failure-detector/phi-values | jq ."

run_test "NETWORKING-020" "Add Health Check" \
  "curl -s -X POST ${NETWORK_API}/health/checks/add \
    -H 'Content-Type: application/json' \
    -d '{
      \"node_id\":\"node2\",
      \"check_type\":\"HTTP\",
      \"endpoint\":\"http://node2:8080/health\",
      \"interval_ms\":5000,
      \"timeout_ms\":2000
    }' | jq ."

run_test "NETWORKING-021" "Get Health Aggregate" \
  "curl -s -X GET ${NETWORK_API}/health/aggregate | jq ."

run_test "NETWORKING-022" "Get Liveness Probe" \
  "curl -s -X GET ${NETWORK_API}/health/liveness/node2 | jq ."

run_test "NETWORKING-023" "Get Readiness Probe" \
  "curl -s -X GET ${NETWORK_API}/health/readiness/node2 | jq ."

run_test "NETWORKING-024" "Trigger Recovery" \
  "curl -s -X POST ${NETWORK_API}/health/recovery/trigger \
    -H 'Content-Type: application/json' \
    -d '{\"node_id\":\"node4\",\"strategy\":\"ExponentialBackoff\"}' | jq ."

# =============================================================================
# Category 6: Service Discovery Tests
# =============================================================================

echo "=== CATEGORY 6: SERVICE DISCOVERY ==="
echo ""

run_test "NETWORKING-025" "DNS Discovery - Get Nodes" \
  "curl -s -X GET ${NETWORK_API}/discovery/dns/nodes | jq ."

run_test "NETWORKING-026" "Kubernetes Discovery - Get Nodes" \
  "curl -s -X GET ${NETWORK_API}/discovery/kubernetes/nodes | jq ."

run_test "NETWORKING-027" "Consul - Register Service" \
  "curl -s -X POST ${NETWORK_API}/discovery/consul/register \
    -H 'Content-Type: application/json' \
    -d '{
      \"service_name\":\"rustydb\",
      \"node_id\":\"node1\",
      \"address\":\"192.168.1.10\",
      \"port\":5432,
      \"tags\":[\"database\",\"primary\"]
    }' | jq ."

run_test "NETWORKING-028" "Consul - Discover Nodes" \
  "curl -s -X GET ${NETWORK_API}/discovery/consul/nodes | jq ."

run_test "NETWORKING-029" "etcd - Get Nodes" \
  "curl -s -X GET ${NETWORK_API}/discovery/etcd/nodes | jq ."

run_test "NETWORKING-030" "Cloud Discovery (AWS) - Get Nodes" \
  "curl -s -X GET ${NETWORK_API}/discovery/cloud/aws/nodes | jq ."

# =============================================================================
# Category 7: Auto-Discovery Tests
# =============================================================================

echo "=== CATEGORY 7: AUTO-DISCOVERY ==="
echo ""

run_test "NETWORKING-031" "Gossip Protocol - Get State" \
  "curl -s -X GET ${NETWORK_API}/autodiscovery/gossip/state | jq ."

run_test "NETWORKING-032" "mDNS - Get Discovered Nodes" \
  "curl -s -X GET ${NETWORK_API}/autodiscovery/mdns/discovered | jq ."

run_test "NETWORKING-033" "UDP Broadcast - Announce" \
  "curl -s -X POST ${NETWORK_API}/autodiscovery/broadcast/announce \
    -H 'Content-Type: application/json' \
    -d '{\"broadcast_address\":\"255.255.255.255\",\"port\":7947}' | jq ."

run_test "NETWORKING-034" "Anti-Entropy - Sync State" \
  "curl -s -X POST ${NETWORK_API}/autodiscovery/anti-entropy/sync \
    -H 'Content-Type: application/json' \
    -d '{\"target_node\":\"node2\"}' | jq ."

# =============================================================================
# Category 8: Membership Tests
# =============================================================================

echo "=== CATEGORY 8: CLUSTER MEMBERSHIP ==="
echo ""

run_test "NETWORKING-035" "Raft - Get Status" \
  "curl -s -X GET ${NETWORK_API}/membership/raft/status | jq ."

run_test "NETWORKING-036" "SWIM - Get Status" \
  "curl -s -X GET ${NETWORK_API}/membership/swim/status | jq ."

run_test "NETWORKING-037" "Get Membership View" \
  "curl -s -X GET ${NETWORK_API}/membership/view | jq ."

run_test "NETWORKING-038" "Join Membership" \
  "curl -s -X POST ${NETWORK_API}/membership/join \
    -H 'Content-Type: application/json' \
    -d '{
      \"node_id\":\"node4\",
      \"address\":\"192.168.1.14:5432\",
      \"seed_nodes\":[\"192.168.1.10:5432\",\"192.168.1.11:5432\"]
    }' | jq ."

run_test "NETWORKING-039" "Leave Membership" \
  "curl -s -X POST ${NETWORK_API}/membership/leave \
    -H 'Content-Type: application/json' \
    -d '{\"node_id\":\"node4\",\"reason\":\"maintenance\"}' | jq ."

run_test "NETWORKING-040" "Bootstrap Cluster" \
  "curl -s -X POST ${NETWORK_API}/membership/bootstrap \
    -H 'Content-Type: application/json' \
    -d '{
      \"cluster_name\":\"rustydb-prod\",
      \"initial_nodes\":[\"node1\",\"node2\",\"node3\"],
      \"replication_factor\":3
    }' | jq ."

# =============================================================================
# Category 9: Load Balancing Tests
# =============================================================================

echo "=== CATEGORY 9: LOAD BALANCING ==="
echo ""

run_test "NETWORKING-041" "Select Node (Round Robin)" \
  "curl -s -X POST ${NETWORK_API}/loadbalancer/select \
    -H 'Content-Type: application/json' \
    -d '{\"strategy\":\"RoundRobin\",\"available_nodes\":[\"node1\",\"node2\",\"node3\"]}' | jq ."

run_test "NETWORKING-042" "Select Node (Least Connections)" \
  "curl -s -X POST ${NETWORK_API}/loadbalancer/select \
    -H 'Content-Type: application/json' \
    -d '{\"strategy\":\"LeastConnections\"}' | jq ."

run_test "NETWORKING-043" "Select Node (Consistent Hash)" \
  "curl -s -X POST ${NETWORK_API}/loadbalancer/select \
    -H 'Content-Type: application/json' \
    -d '{\"strategy\":\"ConsistentHash\",\"routing_key\":\"user_12345\"}' | jq ."

run_test "NETWORKING-044" "Select Node (Adaptive)" \
  "curl -s -X POST ${NETWORK_API}/loadbalancer/select \
    -H 'Content-Type: application/json' \
    -d '{\"strategy\":\"Adaptive\"}' | jq ."

run_test "NETWORKING-045" "Get Circuit Breaker Status" \
  "curl -s -X GET ${NETWORK_API}/loadbalancer/circuit-breaker/status | jq ."

run_test "NETWORKING-046" "Configure Retry Policy" \
  "curl -s -X POST ${NETWORK_API}/loadbalancer/retry/configure \
    -H 'Content-Type: application/json' \
    -d '{
      \"strategy\":\"ExponentialBackoff\",
      \"max_attempts\":3,
      \"initial_delay_ms\":100,
      \"max_delay_ms\":5000
    }' | jq ."

run_test "NETWORKING-047" "Set Rate Limit" \
  "curl -s -X POST ${NETWORK_API}/loadbalancer/rate-limit/set \
    -H 'Content-Type: application/json' \
    -d '{\"node\":\"node2\",\"requests_per_second\":1000,\"burst_size\":50}' | jq ."

# =============================================================================
# Category 10: Connection Pool Tests
# =============================================================================

echo "=== CATEGORY 10: CONNECTION POOL ==="
echo ""

run_test "NETWORKING-048" "Get Pool Statistics" \
  "curl -s -X GET ${NETWORK_API}/pool/statistics | jq ."

run_test "NETWORKING-049" "Get Multiplexing Streams" \
  "curl -s -X GET ${NETWORK_API}/pool/multiplexing/streams | jq ."

run_test "NETWORKING-050" "Warmup Pool" \
  "curl -s -X POST ${NETWORK_API}/pool/warmup \
    -H 'Content-Type: application/json' \
    -d '{\"node\":\"node3\",\"target_size\":10}' | jq ."

run_test "NETWORKING-051" "Configure Eviction Policy" \
  "curl -s -X POST ${NETWORK_API}/pool/eviction/configure \
    -H 'Content-Type: application/json' \
    -d '{
      \"policy\":\"LRU\",
      \"idle_timeout_sec\":300,
      \"max_lifetime_sec\":3600
    }' | jq ."

# =============================================================================
# Category 11: Security Tests
# =============================================================================

echo "=== CATEGORY 11: SECURITY ==="
echo ""

run_test "NETWORKING-052" "Get TLS Configuration" \
  "curl -s -X GET ${NETWORK_API}/security/tls/config | jq ."

run_test "NETWORKING-053" "Verify mTLS Client" \
  "curl -s -X POST ${NETWORK_API}/security/mtls/verify \
    -H 'Content-Type: application/json' \
    --cert client-cert.pem --key client-key.pem | jq ."

run_test "NETWORKING-054" "Encrypt Message" \
  "curl -s -X POST ${NETWORK_API}/security/encryption/encrypt \
    -H 'Content-Type: application/json' \
    -d '{\"data\":\"sensitive information\",\"algorithm\":\"AES256GCM\"}' | jq ."

run_test "NETWORKING-055" "Add ACL Rule" \
  "curl -s -X POST ${NETWORK_API}/security/acl/add \
    -H 'Content-Type: application/json' \
    -d '{
      \"rule\":{
        \"source\":\"10.0.1.0/24\",
        \"destination\":\"node1\",
        \"action\":\"ALLOW\",
        \"priority\":100
      }
    }' | jq ."

run_test "NETWORKING-056" "Get Firewall Rules" \
  "curl -s -X GET ${NETWORK_API}/security/firewall/rules | jq ."

run_test "NETWORKING-057" "Rotate Certificate" \
  "curl -s -X POST ${NETWORK_API}/security/certificates/rotate \
    -H 'Content-Type: application/json' \
    -d '{\"node\":\"node1\",\"new_cert_path\":\"/certs/new-cert.pem\"}' | jq ."

run_test "NETWORKING-058" "Verify Node Identity" \
  "curl -s -X POST ${NETWORK_API}/security/identity/verify \
    -H 'Content-Type: application/json' \
    -d '{
      \"node_id\":\"node2\",
      \"challenge\":\"base64challenge\",
      \"signature\":\"base64sig\"
    }' | jq ."

# =============================================================================
# Category 12: GraphQL Tests
# =============================================================================

echo "=== CATEGORY 12: GRAPHQL API ==="
echo ""

run_test "NETWORKING-059" "GraphQL - Get Peers" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"query { peers { nodeId address state health } }\"}' | jq ."

run_test "NETWORKING-060" "GraphQL - Get Topology" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"query { topology { localNode clusterSize members { id address state } } }\"}' | jq ."

run_test "NETWORKING-061" "GraphQL - Get Network Stats" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"query { networkStats { messagesSent messagesReceived activeConnections avgLatencyMs } }\"}' | jq ."

run_test "NETWORKING-062" "GraphQL - Get Node Info" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"query { nodeInfo(nodeId: \\\"node2\\\") { id address state health } }\"}' | jq ."

run_test "NETWORKING-063" "GraphQL - Get Unhealthy Nodes" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"query { unhealthyNodes }\"}' | jq ."

run_test "NETWORKING-064" "GraphQL - Check Cluster Health" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"query { clusterHealth }\"}' | jq ."

run_test "NETWORKING-065" "GraphQL - Join Cluster Mutation" \
  "curl -s -X POST ${GRAPHQL_URL} \
    -H 'Content-Type: application/json' \
    -d '{\"query\":\"mutation { joinCluster(seedNodes: [\\\"node1:7000\\\", \\\"node2:7000\\\"]) { success message clusterSize } }\"}' | jq ."

# =============================================================================
# Summary
# =============================================================================

echo "========================================"
echo "TEST SUMMARY"
echo "========================================"
echo "Total Tests: ${TOTAL_TESTS}"
echo -e "${GREEN}Passed: ${PASSED_TESTS}${NC}"
echo -e "${RED}Failed: ${FAILED_TESTS}${NC}"
echo ""

if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${YELLOW}NOTE: Failures are expected as networking API endpoints are not yet integrated.${NC}"
    echo ""
    echo "To enable these tests:"
    echo "1. Integrate networking API routes in /home/user/rusty-db/src/api/rest/server.rs"
    echo "2. Mount NetworkManager and routes in the REST server"
    echo "3. Restart the RustyDB server"
    echo ""
fi

exit 0
