//! Integration tests for the E2E test infrastructure.
//!
//! These tests verify that the E2E test infrastructure works correctly.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{NetworkState, TestHarness, TestNetwork, TestNetworkConfig};
use std::time::Duration;

/// Test that a minimal network (5 nodes) can form and stabilize.
#[tokio::test]
#[ignore = "Requires real P2P node spawning - run with --ignored"]
async fn test_minimal_network_formation() {
    // Use unique port range to avoid conflicts with parallel tests
    let config = TestNetworkConfig {
        base_port: 19200, // Different from other tests
        ..TestNetworkConfig::minimal()
    };
    let harness = TestHarness::setup_with_config(config)
        .await
        .expect("Failed to setup harness");

    // Verify network is ready
    assert!(harness.is_ready().await);
    assert_eq!(harness.node_count(), 5);

    // Verify we have connections
    let total_connections = harness.total_connections().await;
    assert!(
        total_connections > 0,
        "Should have at least some connections"
    );

    // Cleanup
    harness.teardown().await.expect("Failed to teardown");
}

/// Test that a small network (10 nodes) can form and stabilize.
#[tokio::test]
#[ignore = "Requires real P2P node spawning - run with --ignored"]
async fn test_small_network_formation() {
    // Use unique port range to avoid conflicts with parallel tests
    let config = TestNetworkConfig {
        base_port: 19300, // Different from other tests
        ..TestNetworkConfig::small()
    };
    let harness = TestHarness::setup_with_config(config)
        .await
        .expect("Failed to setup harness");

    // Verify network is ready
    assert!(harness.is_ready().await);
    assert_eq!(harness.node_count(), 10);

    // Verify all nodes are accessible
    for i in 0..10 {
        assert!(harness.node(i).is_some(), "Node {i} should be accessible");
    }

    // Cleanup
    harness.teardown().await.expect("Failed to teardown");
}

/// Test that the full 25-node network can form.
#[tokio::test]
#[ignore = "Requires real P2P node spawning - run with --ignored"]
async fn test_full_network_formation() {
    let harness = TestHarness::setup().await.expect("Failed to setup harness");

    // Verify network is ready
    assert!(harness.is_ready().await);
    assert_eq!(harness.node_count(), 25);

    // Verify bootstrap nodes
    let network = harness.network();
    assert_eq!(network.bootstrap_nodes().len(), 3);

    // Verify regular nodes
    assert_eq!(network.regular_nodes().len(), 22);

    // Verify we can get random nodes
    assert!(harness.random_node().is_some());
    assert!(harness.random_bootstrap_node().is_some());

    // Cleanup
    harness.teardown().await.expect("Failed to teardown");
}

/// Test custom network configuration.
#[tokio::test]
#[ignore = "Requires real P2P node spawning - run with --ignored"]
async fn test_custom_network_config() {
    let config = TestNetworkConfig {
        node_count: 7,
        bootstrap_count: 2,
        base_port: 19100,
        spawn_delay: Duration::from_millis(100),
        stabilization_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    let harness = TestHarness::setup_with_config(config)
        .await
        .expect("Failed to setup harness");

    assert_eq!(harness.node_count(), 7);
    assert_eq!(harness.network().bootstrap_nodes().len(), 2);
    assert_eq!(harness.network().regular_nodes().len(), 5);

    harness.teardown().await.expect("Failed to teardown");
}

/// Test network with EVM testnet.
#[tokio::test]
#[ignore = "Requires real P2P node spawning and Anvil - run with --ignored"]
async fn test_network_with_evm() {
    // Use unique port range to avoid conflicts with parallel tests
    let config = TestNetworkConfig {
        base_port: 19400, // Different from other tests
        ..TestNetworkConfig::default()
    };
    let harness = TestHarness::setup_with_evm_and_config(config)
        .await
        .expect("Failed to setup harness with EVM");

    // Verify EVM is available
    assert!(harness.has_evm());

    let anvil = harness.anvil().expect("Anvil should be present");
    assert!(anvil.is_healthy().await);
    assert!(!anvil.rpc_url().is_empty());

    harness.teardown().await.expect("Failed to teardown");
}

/// Test network config validation.
#[tokio::test]
async fn test_network_config_validation() {
    // Invalid: bootstrap_count >= node_count
    let config = TestNetworkConfig {
        node_count: 5,
        bootstrap_count: 5,
        ..Default::default()
    };

    let result = TestNetwork::new(config).await;
    assert!(result.is_err());

    // Invalid: zero bootstrap nodes
    let config = TestNetworkConfig {
        node_count: 5,
        bootstrap_count: 0,
        ..Default::default()
    };

    let result = TestNetwork::new(config).await;
    assert!(result.is_err());
}

/// Test network state enum.
#[test]
fn test_network_state() {
    assert!(!NetworkState::Uninitialized.is_running());
    assert!(!NetworkState::BootstrappingPhase.is_running());
    assert!(!NetworkState::NodeSpawningPhase.is_running());
    assert!(NetworkState::Stabilizing.is_running());
    assert!(NetworkState::Ready.is_running());
    assert!(!NetworkState::ShuttingDown.is_running());
    assert!(!NetworkState::Stopped.is_running());
    assert!(!NetworkState::Failed("error".to_string()).is_running());
}

/// Test `TestNetworkConfig` presets.
#[test]
fn test_config_presets() {
    let default = TestNetworkConfig::default();
    assert_eq!(default.node_count, 25);
    assert_eq!(default.bootstrap_count, 3);

    let minimal = TestNetworkConfig::minimal();
    assert_eq!(minimal.node_count, 5);
    assert_eq!(minimal.bootstrap_count, 2);

    let small = TestNetworkConfig::small();
    assert_eq!(small.node_count, 10);
    assert_eq!(small.bootstrap_count, 3);
}
