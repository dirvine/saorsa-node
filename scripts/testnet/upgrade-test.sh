#!/bin/bash
# Upgrade Test Script
# Tests that the auto-upgrade system works correctly
# Usage: ./scripts/testnet/upgrade-test.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Worker IPs
WORKERS=("142.93.52.129" "24.199.82.114" "192.34.62.192" "159.223.131.196")
START_INDICES=(0 50 100 150)

echo "=== Saorsa Upgrade Test ==="
echo "Testing auto-upgrade functionality across 200-node testnet"
echo ""

# Get current version on all workers
get_all_versions() {
    echo "--- Current Versions ---"
    for i in "${!WORKERS[@]}"; do
        IP="${WORKERS[$i]}"
        VERSION=$(ssh -o StrictHostKeyChecking=no root@$IP "/usr/local/bin/saorsa-node --version 2>/dev/null | head -1" || echo "unknown")
        RUNNING=$(ssh -o StrictHostKeyChecking=no root@$IP "pgrep -c saorsa-node 2>/dev/null" || echo "0")
        echo "Worker $((i+1)) ($IP): $VERSION - $RUNNING nodes running"
    done
    echo ""
}

# Check if auto-upgrade is enabled
check_upgrade_config() {
    echo "--- Checking Upgrade Configuration ---"
    for i in "${!WORKERS[@]}"; do
        IP="${WORKERS[$i]}"
        echo -n "Worker $((i+1)) ($IP): "
        # Check if nodes have auto-upgrade enabled (look for --auto-upgrade in service files)
        HAS_UPGRADE=$(ssh -o StrictHostKeyChecking=no root@$IP "grep -l 'auto-upgrade' /etc/systemd/system/saorsa-node-*.service 2>/dev/null | wc -l" || echo "0")
        if [ "$HAS_UPGRADE" -gt "0" ]; then
            echo "Auto-upgrade enabled ($HAS_UPGRADE services)"
        else
            echo "Auto-upgrade NOT enabled"
        fi
    done
    echo ""
}

# Get latest release version from GitHub
get_latest_release() {
    LATEST=$(gh release view --json tagName -q '.tagName' 2>/dev/null || echo "unknown")
    echo "Latest GitHub Release: $LATEST"
}

# Monitor upgrade progress
monitor_upgrade() {
    local TARGET_VERSION="$1"
    local MAX_WAIT=3600  # 1 hour max wait (staged rollout window)
    local WAIT_INTERVAL=60
    local ELAPSED=0

    echo "--- Monitoring Upgrade to $TARGET_VERSION ---"
    echo "Checking every $WAIT_INTERVAL seconds (max wait: $MAX_WAIT seconds)"
    echo ""

    while [ $ELAPSED -lt $MAX_WAIT ]; do
        TOTAL_UPGRADED=0
        TOTAL_NODES=0

        for i in "${!WORKERS[@]}"; do
            IP="${WORKERS[$i]}"

            # Count nodes on target version (check logs for version)
            RUNNING=$(ssh -o StrictHostKeyChecking=no root@$IP "pgrep -c saorsa-node 2>/dev/null" || echo "0")
            TOTAL_NODES=$((TOTAL_NODES + RUNNING))

            # Check if binary is updated
            CURRENT_VER=$(ssh -o StrictHostKeyChecking=no root@$IP "/usr/local/bin/saorsa-node --version 2>/dev/null | grep -oP 'saorsa-node \K[0-9.]+'" || echo "0")
            if [ "$CURRENT_VER" = "$TARGET_VERSION" ]; then
                TOTAL_UPGRADED=$((TOTAL_UPGRADED + RUNNING))
            fi
        done

        PERCENT=$((TOTAL_UPGRADED * 100 / TOTAL_NODES))
        echo "[$ELAPSED s] $TOTAL_UPGRADED / $TOTAL_NODES nodes on $TARGET_VERSION ($PERCENT%)"

        if [ $TOTAL_UPGRADED -eq $TOTAL_NODES ]; then
            echo ""
            echo "SUCCESS: All $TOTAL_NODES nodes upgraded to $TARGET_VERSION"
            return 0
        fi

        sleep $WAIT_INTERVAL
        ELAPSED=$((ELAPSED + WAIT_INTERVAL))
    done

    echo ""
    echo "WARNING: Timeout waiting for upgrade. $TOTAL_UPGRADED / $TOTAL_NODES upgraded."
    return 1
}

# Run tests
echo "=== Phase 1: Pre-Upgrade Status ==="
get_all_versions
check_upgrade_config
get_latest_release

echo ""
echo "=== Phase 2: Check for Pending Upgrade ==="
CURRENT_VER=$(ssh -o StrictHostKeyChecking=no root@${WORKERS[0]} "/usr/local/bin/saorsa-node --version 2>/dev/null | grep -oP 'saorsa-node \K[0-9.]+'" || echo "0")
LATEST_VER=$(gh release view --json tagName -q '.tagName' 2>/dev/null | sed 's/v//' || echo "unknown")

echo "Current version: $CURRENT_VER"
echo "Latest release: $LATEST_VER"

if [ "$CURRENT_VER" = "$LATEST_VER" ]; then
    echo ""
    echo "Nodes are already on latest version. No upgrade to test."
    echo "To test upgrade:"
    echo "  1. Bump version in Cargo.toml"
    echo "  2. Commit and push"
    echo "  3. Create release tag: git tag vX.Y.Z && git push --tags"
    echo "  4. Wait for release workflow to complete"
    echo "  5. Re-run this script"
else
    echo ""
    echo "=== Phase 3: Monitor Upgrade Progress ==="
    monitor_upgrade "$LATEST_VER"
fi

echo ""
echo "=== Phase 4: Final Status ==="
get_all_versions

echo ""
echo "=== Upgrade Test Complete ==="
