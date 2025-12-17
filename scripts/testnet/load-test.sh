#!/bin/bash
# Load Test Script
# Stores thousands of chunks on the testnet and verifies retrieval
# Usage: ./scripts/testnet/load-test.sh [chunk_count] [chunk_size_kb]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_DIR="$PROJECT_ROOT/testnet-logs"
mkdir -p "$LOG_DIR"

# Configuration
CHUNK_COUNT=${1:-1000}
CHUNK_SIZE_KB=${2:-1}  # Default 1KB chunks
CONCURRENCY=${3:-10}   # Concurrent operations

# Testnet configuration
export SAORSA_TEST_BOOTSTRAP="142.93.52.129:12000,24.199.82.114:12000"
export SAORSA_TEST_EXTERNAL=true
export SAORSA_TEST_CHUNK_COUNT="$CHUNK_COUNT"
export SAORSA_TEST_CHUNK_SIZE_KB="$CHUNK_SIZE_KB"
export SAORSA_TEST_CONCURRENCY="$CONCURRENCY"
export RUST_LOG=info

LOG_FILE="$LOG_DIR/load-test-$(date +%Y%m%d-%H%M%S).log"
ADDRESSES_FILE="$LOG_DIR/chunk-addresses-$(date +%Y%m%d-%H%M%S).txt"

log() {
    local MSG="[$(date '+%Y-%m-%d %H:%M:%S')] $1"
    echo "$MSG" | tee -a "$LOG_FILE"
}

# Check cluster health
check_health() {
    log "--- Checking Cluster Health ---"
    local TOTAL=0
    for IP in 142.93.52.129 24.199.82.114 192.34.62.192 159.223.131.196; do
        COUNT=$(ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 root@$IP "pgrep -c saorsa-node 2>/dev/null" 2>/dev/null || echo "0")
        log "$IP: $COUNT nodes running"
        TOTAL=$((TOTAL + COUNT))
    done
    log "Total: $TOTAL nodes"

    if [ $TOTAL -lt 100 ]; then
        log "WARNING: Less than 100 nodes running. Test may not be reliable."
    fi
}

main() {
    log "=== Saorsa Load Test ==="
    log "Chunk count: $CHUNK_COUNT"
    log "Chunk size: ${CHUNK_SIZE_KB}KB"
    log "Concurrency: $CONCURRENCY"
    log "Addresses file: $ADDRESSES_FILE"
    log ""

    check_health
    log ""

    cd "$PROJECT_ROOT"

    # Run the load test
    log "--- Starting Load Test ---"
    START_TIME=$(date +%s)

    export SAORSA_TEST_ADDRESSES_FILE="$ADDRESSES_FILE"
    cargo test --release --test e2e run_load_test -- --ignored --nocapture 2>&1 | tee -a "$LOG_FILE"

    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    log ""
    log "--- Load Test Complete ---"
    log "Duration: ${DURATION}s"

    if [ -f "$ADDRESSES_FILE" ]; then
        STORED=$(wc -l < "$ADDRESSES_FILE")
        log "Chunks stored: $STORED"
        log "Throughput: $(echo "scale=2; $STORED / $DURATION" | bc) chunks/sec"
    fi

    log ""
    log "=== Results ==="
    log "Log: $LOG_FILE"
    log "Addresses: $ADDRESSES_FILE"
}

main "$@"
