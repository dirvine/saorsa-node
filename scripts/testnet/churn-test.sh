#!/bin/bash
# Churn Test Script
# Randomly kills and restarts nodes while verifying data availability
# Usage: ./scripts/testnet/churn-test.sh [duration_minutes] [churn_rate_percent]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_DIR="$PROJECT_ROOT/testnet-logs"
mkdir -p "$LOG_DIR"

# Configuration
DURATION_MINUTES=${1:-30}
CHURN_RATE=${2:-10}  # Percentage of nodes to churn per cycle
CHURN_INTERVAL=60    # Seconds between churn events
VERIFY_INTERVAL=30   # Seconds between verification checks

# Worker configuration
WORKERS=("142.93.52.129" "24.199.82.114" "192.34.62.192" "159.223.131.196")
START_INDICES=(0 50 100 150)
NODES_PER_WORKER=50
TOTAL_NODES=200

LOG_FILE="$LOG_DIR/churn-test-$(date +%Y%m%d-%H%M%S).log"

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

# Get node details for a specific worker
get_worker_nodes() {
    local WORKER_IDX=$1
    local IP="${WORKERS[$WORKER_IDX]}"
    local START_IDX="${START_INDICES[$WORKER_IDX]}"

    # Get list of running node indices on this worker
    ssh -o StrictHostKeyChecking=no root@$IP "
        for svc in /etc/systemd/system/saorsa-node-*.service; do
            NAME=\$(basename \$svc .service)
            IDX=\$(echo \$NAME | grep -oP 'saorsa-node-\K[0-9]+')
            if systemctl is-active --quiet \$NAME 2>/dev/null; then
                echo \$IDX
            fi
        done
    " 2>/dev/null || true
}

# Kill random nodes
kill_random_nodes() {
    local NUM_TO_KILL=$1
    log "Killing $NUM_TO_KILL random nodes..."

    local KILLED=0
    while [ $KILLED -lt $NUM_TO_KILL ]; do
        # Pick a random worker
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
            # Pick a random running node
            local NODE_ARRAY=($RUNNING_NODES)
            local NODE_IDX=${NODE_ARRAY[$((RANDOM % ${#NODE_ARRAY[@]}))]}

            # Kill it
            ssh -o StrictHostKeyChecking=no root@$IP "systemctl stop saorsa-node-$NODE_IDX" 2>/dev/null || true
            log "  Killed node $NODE_IDX on $IP"
            KILLED=$((KILLED + 1))
        fi
    done
}

# Restart random stopped nodes
restart_random_nodes() {
    local NUM_TO_RESTART=$1
    log "Restarting $NUM_TO_RESTART random nodes..."

    local RESTARTED=0
    while [ $RESTARTED -lt $NUM_TO_RESTART ]; do
        # Pick a random worker
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
            # Pick a random stopped node
            local NODE_ARRAY=($STOPPED_NODES)
            local NODE_IDX=${NODE_ARRAY[$((RANDOM % ${#NODE_ARRAY[@]}))]}

            # Restart it
            ssh -o StrictHostKeyChecking=no root@$IP "systemctl start saorsa-node-$NODE_IDX" 2>/dev/null || true
            log "  Restarted node $NODE_IDX on $IP"
            RESTARTED=$((RESTARTED + 1))
        fi
    done
}

# Churn cycle: kill some nodes, wait, restart them
churn_cycle() {
    local NUM_NODES=$(( TOTAL_NODES * CHURN_RATE / 100 ))
    if [ $NUM_NODES -lt 1 ]; then
        NUM_NODES=1
    fi

    kill_random_nodes $NUM_NODES
    sleep 5
    restart_random_nodes $NUM_NODES
}

# Verify data availability (placeholder - actual verification done by Rust tests)
verify_data() {
    local RUNNING=$(get_node_count)
    local PERCENT=$((RUNNING * 100 / TOTAL_NODES))
    log "Node health: $RUNNING / $TOTAL_NODES running ($PERCENT%)"

    # Check per-worker status
    for i in "${!WORKERS[@]}"; do
        local IP="${WORKERS[$i]}"
        local COUNT=$(ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 root@$IP "pgrep -c saorsa-node 2>/dev/null" 2>/dev/null || echo "0")
        log "  Worker $((i+1)) ($IP): $COUNT nodes"
    done
}

# Main test loop
main() {
    log "=== Saorsa Churn Test ==="
    log "Duration: $DURATION_MINUTES minutes"
    log "Churn rate: $CHURN_RATE% ($((TOTAL_NODES * CHURN_RATE / 100)) nodes per cycle)"
    log "Churn interval: $CHURN_INTERVAL seconds"
    log ""

    # Initial status
    log "--- Initial Status ---"
    verify_data
    log ""

    # Calculate end time
    local END_TIME=$(($(date +%s) + DURATION_MINUTES * 60))
    local CYCLE=0

    log "--- Starting Churn Test ---"
    while [ $(date +%s) -lt $END_TIME ]; do
        CYCLE=$((CYCLE + 1))
        log ""
        log "=== Churn Cycle $CYCLE ==="

        # Perform churn
        churn_cycle

        # Wait and verify
        log "Waiting $VERIFY_INTERVAL seconds..."
        sleep $VERIFY_INTERVAL

        # Verify health
        verify_data

        # Calculate remaining time
        local REMAINING=$(( (END_TIME - $(date +%s)) / 60 ))
        log "Time remaining: $REMAINING minutes"

        # Wait before next cycle
        sleep $((CHURN_INTERVAL - VERIFY_INTERVAL))
    done

    log ""
    log "=== Final Status ==="
    verify_data

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
    verify_data

    log ""
    log "=== Churn Test Complete ==="
    log "Log file: $LOG_FILE"
}

main "$@"
