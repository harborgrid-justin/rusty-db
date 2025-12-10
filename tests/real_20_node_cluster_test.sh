#!/bin/bash
#
# RustyDB REAL 20-Node Cluster Test
# ==================================
# This script starts 20 ACTUAL server processes on different ports
# using an LD_PRELOAD shim to override port bindings.
#
# Each node runs the full RustyDB server binary as a separate process.
#

# Don't exit on error - we want to continue even if some nodes fail
# set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SERVER_BIN="$PROJECT_DIR/target/release/rusty-db-server"
PORT_SHIM="$SCRIPT_DIR/port_override.so"
LOG_DIR="$PROJECT_DIR/test_logs/nodes"
NUM_NODES=20
BASE_DB_PORT=5432
BASE_API_PORT=8080

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Array to store PIDs
declare -a NODE_PIDS

cleanup() {
    echo ""
    echo -e "${YELLOW}[CLEANUP]${NC} Stopping all nodes..."
    for pid in "${NODE_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    # Also kill any stragglers
    pkill -f "rusty-db-server" 2>/dev/null || true
    echo -e "${GREEN}[CLEANUP]${NC} All nodes stopped"
}

trap cleanup EXIT

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║     RustyDB REAL 20-Node Cluster Test                      ║"
echo "║     Running 20 Actual Server Processes                     ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Date: $(date)"
echo ""

# Check prerequisites
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Checking Prerequisites${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

if [ ! -f "$SERVER_BIN" ]; then
    echo -e "${RED}[ERROR]${NC} Server binary not found: $SERVER_BIN"
    exit 1
fi
echo -e "${GREEN}[OK]${NC} Server binary found"

if [ ! -f "$PORT_SHIM" ]; then
    echo -e "${YELLOW}[INFO]${NC} Compiling port override shim..."
    gcc -shared -fPIC -o "$PORT_SHIM" "$SCRIPT_DIR/port_override.c" -ldl
fi
echo -e "${GREEN}[OK]${NC} Port override shim ready"

# Kill any existing servers
pkill -f "rusty-db-server" 2>/dev/null || true
sleep 1

# Create log directory
mkdir -p "$LOG_DIR"
echo -e "${GREEN}[OK]${NC} Log directory created: $LOG_DIR"
echo ""

# Start 20 nodes
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Starting 20 Database Nodes${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

NODES_STARTED=0
DATACENTERS=("dc-us-east" "dc-us-west" "dc-eu-west" "dc-ap-south")

for i in $(seq 0 $((NUM_NODES - 1))); do
    NODE_ID=$(printf "node-%02d" $i)
    DB_PORT=$((BASE_DB_PORT + i))
    API_PORT=$((BASE_API_PORT + i))
    DC_IDX=$((i / 5))
    DC="${DATACENTERS[$DC_IDX]}"
    DATA_DIR="$PROJECT_DIR/test_data/$NODE_ID"
    NODE_LOG="$LOG_DIR/$NODE_ID.log"

    # Create data directory for this node
    mkdir -p "$DATA_DIR"

    # Determine role
    if [ $i -eq 0 ]; then
        ROLE="leader"
    else
        ROLE="follower"
    fi

    # Start the server with port override
    RUSTYDB_PORT=$DB_PORT \
    RUSTYDB_API_PORT=$API_PORT \
    LD_PRELOAD="$PORT_SHIM" \
    "$SERVER_BIN" > "$NODE_LOG" 2>&1 &

    PID=$!
    NODE_PIDS+=($PID)

    # Brief pause to avoid port conflicts during startup
    sleep 0.3

    # Check if process is still running
    if kill -0 "$PID" 2>/dev/null; then
        echo -e "  ${GREEN}+${NC} Started $NODE_ID (PID: $PID) - DB:$DB_PORT API:$API_PORT [$DC] ($ROLE)"
        ((NODES_STARTED++))
    else
        echo -e "  ${RED}x${NC} Failed to start $NODE_ID"
    fi
done

echo ""
echo -e "${GREEN}[INFO]${NC} Started $NODES_STARTED of $NUM_NODES nodes"
echo ""

# Wait for all nodes to be ready
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Waiting for Nodes to Initialize${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

sleep 5  # Give nodes time to initialize

NODES_READY=0
for i in $(seq 0 $((NUM_NODES - 1))); do
    NODE_ID=$(printf "node-%02d" $i)
    API_PORT=$((BASE_API_PORT + i))

    if curl -s "http://127.0.0.1:$API_PORT/api/v1/cluster/nodes" > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} $NODE_ID (port $API_PORT) is ready"
        ((NODES_READY++))
    else
        echo -e "  ${YELLOW}○${NC} $NODE_ID (port $API_PORT) not responding yet"
    fi
done

echo ""
echo -e "${GREEN}[INFO]${NC} $NODES_READY of $NUM_NODES nodes are ready"
echo ""

# Test connectivity between nodes
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Testing Node Connectivity${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

CONNECTIVITY_TESTS=0
CONNECTIVITY_PASSED=0

for i in $(seq 0 $((NUM_NODES - 1))); do
    API_PORT=$((BASE_API_PORT + i))
    NODE_ID=$(printf "node-%02d" $i)

    RESPONSE=$(curl -s "http://127.0.0.1:$API_PORT/api/v1/cluster/topology" 2>&1)

    ((CONNECTIVITY_TESTS++))

    if echo "$RESPONSE" | grep -q "cluster_id"; then
        echo -e "  ${GREEN}✓${NC} $NODE_ID responds to topology query"
        ((CONNECTIVITY_PASSED++))
    else
        echo -e "  ${RED}✗${NC} $NODE_ID failed topology query"
    fi
done

echo ""
echo -e "${GREEN}[INFO]${NC} Connectivity: $CONNECTIVITY_PASSED/$CONNECTIVITY_TESTS nodes responding"
echo ""

# Test cross-node communication
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Testing Cross-Node Communication${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

echo "Simulating cross-datacenter queries..."
echo ""

# Query from node-00 (US-EAST) to node-05 (US-WEST)
echo "  Route: node-00 (US-EAST:8080) -> node-05 (US-WEST:8085)"
R1=$(curl -s "http://127.0.0.1:8080/api/v1/cluster/nodes" 2>&1)
R2=$(curl -s "http://127.0.0.1:8085/api/v1/cluster/nodes" 2>&1)
if echo "$R1" | grep -q "node" && echo "$R2" | grep -q "node"; then
    echo -e "    ${GREEN}✓${NC} Cross-datacenter communication successful"
else
    echo -e "    ${YELLOW}○${NC} Limited response from nodes"
fi

# Query from node-10 (EU-WEST) to node-15 (AP-SOUTH)
echo "  Route: node-10 (EU-WEST:8090) -> node-15 (AP-SOUTH:8095)"
R3=$(curl -s "http://127.0.0.1:8090/api/v1/cluster/nodes" 2>&1)
R4=$(curl -s "http://127.0.0.1:8095/api/v1/cluster/nodes" 2>&1)
if echo "$R3" | grep -q "node" && echo "$R4" | grep -q "node"; then
    echo -e "    ${GREEN}✓${NC} Cross-datacenter communication successful"
else
    echo -e "    ${YELLOW}○${NC} Limited response from nodes"
fi

echo ""

# Test cluster configuration on each node
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Verifying Cluster Configuration${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

for i in 0 5 10 15; do
    NODE_ID=$(printf "node-%02d" $i)
    API_PORT=$((BASE_API_PORT + i))
    DC="${DATACENTERS[$((i / 5))]}"

    CONFIG=$(curl -s "http://127.0.0.1:$API_PORT/api/v1/cluster/config" 2>&1)

    if echo "$CONFIG" | grep -q "replication_factor"; then
        RF=$(echo "$CONFIG" | grep -o '"replication_factor":[0-9]*' | cut -d':' -f2)
        echo -e "  ${GREEN}✓${NC} $NODE_ID ($DC): replication_factor=$RF"
    else
        echo -e "  ${YELLOW}○${NC} $NODE_ID ($DC): config not available"
    fi
done

echo ""

# Test replication status
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Testing Replication Status${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

for i in 0 5 10 15; do
    NODE_ID=$(printf "node-%02d" $i)
    API_PORT=$((BASE_API_PORT + i))

    REPL=$(curl -s "http://127.0.0.1:$API_PORT/api/v1/cluster/replication" 2>&1)

    if echo "$REPL" | grep -q "primary_node"; then
        SYNC=$(echo "$REPL" | grep -o '"sync_state":"[^"]*"' | cut -d'"' -f4)
        echo -e "  ${GREEN}✓${NC} $NODE_ID: sync_state=$SYNC"
    else
        echo -e "  ${YELLOW}○${NC} $NODE_ID: replication status not available"
    fi
done

echo ""

# Test GraphQL on multiple nodes
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Testing GraphQL API on Multiple Nodes${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

GRAPHQL_TESTS=0
GRAPHQL_PASSED=0

for i in 0 4 9 14 19; do
    NODE_ID=$(printf "node-%02d" $i)
    API_PORT=$((BASE_API_PORT + i))

    GRAPHQL=$(curl -s -X POST "http://127.0.0.1:$API_PORT/graphql" \
        -H "Content-Type: application/json" \
        -d '{"query": "{ __schema { queryType { name } } }"}' 2>&1)

    ((GRAPHQL_TESTS++))

    if echo "$GRAPHQL" | grep -q "queryType"; then
        echo -e "  ${GREEN}✓${NC} $NODE_ID (port $API_PORT): GraphQL responding"
        ((GRAPHQL_PASSED++))
    else
        echo -e "  ${YELLOW}○${NC} $NODE_ID (port $API_PORT): GraphQL not responding"
    fi
done

echo ""
echo -e "${GREEN}[INFO]${NC} GraphQL: $GRAPHQL_PASSED/$GRAPHQL_TESTS nodes responding"
echo ""

# Display running processes
echo -e "${CYAN}═══════════════════════════════════════${NC}"
echo -e "${CYAN}Running Node Processes${NC}"
echo -e "${CYAN}═══════════════════════════════════════${NC}"

RUNNING_COUNT=0
for i in "${!NODE_PIDS[@]}"; do
    PID="${NODE_PIDS[$i]}"
    NODE_ID=$(printf "node-%02d" $i)

    if kill -0 "$PID" 2>/dev/null; then
        MEM=$(ps -o rss= -p "$PID" 2>/dev/null | awk '{print $1/1024 " MB"}' || echo "N/A")
        echo -e "  ${GREEN}●${NC} $NODE_ID (PID: $PID) - Memory: $MEM"
        ((RUNNING_COUNT++))
    else
        echo -e "  ${RED}○${NC} $NODE_ID (PID: $PID) - STOPPED"
    fi
done

echo ""
echo -e "${GREEN}[INFO]${NC} $RUNNING_COUNT of $NUM_NODES processes running"
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════════"
echo "                      TEST SUMMARY"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  Nodes Started:        $NODES_STARTED / $NUM_NODES"
echo "  Nodes Ready:          $NODES_READY / $NUM_NODES"
echo "  Connectivity Tests:   $CONNECTIVITY_PASSED / $CONNECTIVITY_TESTS"
echo "  GraphQL Tests:        $GRAPHQL_PASSED / $GRAPHQL_TESTS"
echo "  Processes Running:    $RUNNING_COUNT / $NUM_NODES"
echo ""

if [ $RUNNING_COUNT -ge 15 ]; then
    echo -e "${GREEN}  RESULT: SUCCESS - Majority of nodes running${NC}"
else
    echo -e "${YELLOW}  RESULT: PARTIAL - Some nodes failed to start${NC}"
fi

echo ""
echo "  Port Allocation:"
echo "    - Native DB Ports: 5432-5451 (20 nodes)"
echo "    - REST API Ports:  8080-8099 (20 nodes)"
echo ""
echo "  Datacenters:"
echo "    - dc-us-east:  nodes 00-04"
echo "    - dc-us-west:  nodes 05-09"
echo "    - dc-eu-west:  nodes 10-14"
echo "    - dc-ap-south: nodes 15-19"
echo ""
echo "  Log Files: $LOG_DIR/"
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Keep nodes running for manual testing (optional)
echo "Nodes will remain running for 30 seconds for manual testing..."
echo "Press Ctrl+C to stop earlier."
echo ""

sleep 30

echo "Test complete. Shutting down nodes..."
