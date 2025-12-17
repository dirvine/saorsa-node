#!/bin/bash
# Combined Churn + Verification Test
# Runs node churn while continuously verifying data availability
# Usage: ./scripts/testnet/churn-verify.sh [addresses_file] [duration_minutes] [churn_rate]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_DIR="$PROJECT_ROOT/testnet-logs"
mkdir -p "$LOG_DIR"

# Configuration
ADDRESSES_FILE=${1:-"$LOG_DIR/chunk-addresses-latest.txt"}
DURATION_MINUTES=${2:-30}
CHURN_RATE=${3:-10}  # Percentage of nodes to churn per cycle
VERIFY_INTERVAL=30   # Seconds between verification rounds
CHURN_INTERVAL=60    # Seconds between churn events

# Workers configuration
WORKERS=("142.93.52.129" "24.199.82.114" "192.34.62.192" "159.223.131.196")
START_INDICES=(0 50 100 150)
NODES_PER_WORKER=50
TOTAL_NODES=200

LOG_FILE="$LOG_DIR/churn-verify-$(date +%Y%m%d-%H%M%S).log"

# Testnet configuration
export SAORSA_TEST_BOOTSTRAP="142.93.52.129:12000,24.199.82.114:12000"
export SAORSA_TEST_EXTERNAL=true
export RUST_LOG=info

log() {
    local MSG="[$(date '+%Y-%m-%d %H:%M:%S')] $1"
    echo "$MSG" | tee -a "$LOG_FILE"
}

# Get count of running nodes
get_node_count() {
    local TOTAL=0
    for IP in "${WORKERS[@]}"; do
        COUNT=$(ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 root@$IP "pgrep -c saorsa-node 2>/dev/null" 2>/dev/null || echo "0")
        TOTAL=$((TOTAL + COUNT))
    done
    echo $TOTAL
}

# Kill random nodes
kill_random_nodes() {
    local NUM_TO_KILL=$1
    log "  Killing $NUM_TO_KILL random nodes..."

    local KILLED=0
    while [ $KILLED -lt $NUM_TO_KILL ]; do
        local WORKER_IDX=$((RANDOM % ${#WORKERS[@]}))
        local IP="${WORKERS[$WORKER_IDX]}"
        local START_IDX="${START_INDICES[$WORKER_IDX]}"

        # Get running nodes on this worker
        local RUNNING_NODES=$(ssh -o StrictHostKeyChecking=no root@$IP "
            for i in \$(seq $START_IDX $((START_IDX + NODES_PER_WORKER - 1))); do
                if systemctl is-active --quiet saorsa-node-\$i 2>/dev/null; then
                    echo \$i
                fi
            done
        " 2>/dev/null || true)

        if [ -n "$RUNNING_NODES" ]; then
            local NODE_ARRAY=($RUNNING_NODES)
            local NODE_IDX=${NODE_ARRAY[$((RANDOM % ${#NODE_ARRAY[@]}))]}

            ssh -o StrictHostKeyChecking=no root@$IP "systemctl stop saorsa-node-$NODE_IDX" 2>/dev/null || true
            log "    Killed node $NODE_IDX on $IP"
            KILLED=$((KILLED + 1))
        fi
    done
}

# Restart random stopped nodes
restart_random_nodes() {
    local NUM_TO_RESTART=$1
    log "  Restarting $NUM_TO_RESTART random nodes..."

    local RESTARTED=0
    while [ $RESTARTED -lt $NUM_TO_RESTART ]; do
        local WORKER_IDX=$((RANDOM % ${#WORKERS[@]}))
        local IP="${WORKERS[$WORKER_IDX]}"
        local START_IDX="${START_INDICES[$WORKER_IDX]}"

        # Get stopped nodes on this worker
        local STOPPED_NODES=$(ssh -o StrictHostKeyChecking=no root@$IP "
            for i in \$(seq $START_IDX $((START_IDX + NODES_PER_WORKER - 1))); do
                if ! systemctl is-active --quiet saorsa-node-\$i 2>/dev/null; then
                    echo \$i
                fi
            done
        " 2>/dev/null || true)

        if [ -n "$STOPPED_NODES" ]; then
            local NODE_ARRAY=($STOPPED_NODES)
            local NODE_IDX=${NODE_ARRAY[$((RANDOM % ${#NODE_ARRAY[@]}))]}

            ssh -o StrictHostKeyChecking=no root@$IP "systemctl start saorsa-node-$NODE_IDX" 2>/dev/null || true
            log "    Restarted node $NODE_IDX on $IP"
            RESTARTED=$((RESTARTED + 1))
        fi
    done
}

# Churn cycle
churn_cycle() {
    local NUM_NODES=$(( TOTAL_NODES * CHURN_RATE / 100 ))
    if [ $NUM_NODES -lt 1 ]; then
        NUM_NODES=1
    fi

    kill_random_nodes $NUM_NODES
    sleep 5
    restart_random_nodes $NUM_NODES
}

# Verify data availability using Rust test
verify_data() {
    local ADDRESSES_FILE=$1
    log "--- Verifying Data Availability ---"

    local RUNNING=$(get_node_count)
    local PERCENT=$((RUNNING * 100 / TOTAL_NODES))
    log "  Node health: $RUNNING / $TOTAL_NODES running ($PERCENT%)"

    # Run verification test
    cd "$PROJECT_ROOT"
    export SAORSA_TEST_ADDRESSES_FILE="$ADDRESSES_FILE"

    # Run quick verification (sample of chunks)
    local RESULT=$(cargo test --release --test e2e run_verify_chunks -- --ignored --nocapture 2>&1)

    if echo "$RESULT" | grep -q "PASSED"; then
        local VERIFIED=$(echo "$RESULT" | grep -oP 'verified: \K[0-9]+' || echo "?")
        local TOTAL=$(echo "$RESULT" | grep -oP 'total: \K[0-9]+' || echo "?")
        log "  Data verification: $VERIFIED / $TOTAL chunks available (100%)"
        return 0
    else
        log "  WARNING: Some chunks may be unavailable!"
        echo "$RESULT" | tail -10 >> "$LOG_FILE"
        return 1
    fi
}

# Main test
main() {
    log "=== Saorsa Churn + Verification Test ==="
    log "Addresses file: $ADDRESSES_FILE"
    log "Duration: $DURATION_MINUTES minutes"
    log "Churn rate: $CHURN_RATE%"
    log ""

    # Check addresses file exists
    if [ ! -f "$ADDRESSES_FILE" ]; then
        log "ERROR: Addresses file not found: $ADDRESSES_FILE"
        log "Please run load-test.sh first to create chunk addresses."
        exit 1
    fi

    CHUNK_COUNT=$(wc -l < "$ADDRESSES_FILE")
    log "Chunks to verify: $CHUNK_COUNT"
    log ""

    # Initial status
    log "--- Initial Status ---"
    verify_data "$ADDRESSES_FILE"
    log ""

    # Calculate end time
    local END_TIME=$(($(date +%s) + DURATION_MINUTES * 60))
    local CYCLE=0
    local VERIFY_SUCCESS=0
    local VERIFY_FAIL=0

    log "--- Starting Churn + Verification Test ---"
    while [ $(date +%s) -lt $END_TIME ]; do
        CYCLE=$((CYCLE + 1))
        log ""
        log "=== Cycle $CYCLE ==="

        # Perform churn
        log "--- Churning Nodes ---"
        churn_cycle

        # Wait and verify
        log "Waiting $VERIFY_INTERVAL seconds for network to stabilize..."
        sleep $VERIFY_INTERVAL

        # Verify data
        if verify_data "$ADDRESSES_FILE"; then
            VERIFY_SUCCESS=$((VERIFY_SUCCESS + 1))
        else
            VERIFY_FAIL=$((VERIFY_FAIL + 1))
        fi

        # Calculate remaining time
        local REMAINING=$(( (END_TIME - $(date +%s)) / 60 ))
        log "Time remaining: $REMAINING minutes"

        # Wait before next cycle (if time remains)
        if [ $(date +%s) -lt $END_TIME ]; then
            sleep $((CHURN_INTERVAL - VERIFY_INTERVAL))
        fi
    done

    log ""
    log "=== Final Status ==="
    verify_data "$ADDRESSES_FILE"

    # Restore all nodes
    log ""
    log "--- Restoring All Nodes ---"
    for i in "${!WORKERS[@]}"; do
        local IP="${WORKERS[$i]}"
        local START_IDX="${START_INDICES[$i]}"
        log "Restarting all nodes on $IP..."
        ssh -o StrictHostKeyChecking=no root@$IP "/usr/local/bin/start-nodes.sh $START_IDX" &
    done
    wait

    sleep 10
    log ""
    log "--- Post-Recovery Status ---"
    verify_data "$ADDRESSES_FILE"

    log ""
    log "=== Test Summary ==="
    log "Total cycles: $CYCLE"
    log "Verification passes: $VERIFY_SUCCESS"
    log "Verification failures: $VERIFY_FAIL"

    if [ $VERIFY_FAIL -eq 0 ]; then
        log "RESULT: SUCCESS - 100% data availability maintained during churn"
    else
        log "RESULT: FAILURE - Data was unavailable during some verification cycles"
    fi

    log ""
    log "Log file: $LOG_FILE"
}

main "$@"
