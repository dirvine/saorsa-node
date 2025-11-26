//! Autonomi network verifier for checking if data already exists.
//!
//! This module provides functionality to query the autonomi network to determine
//! if content (identified by `XorName`) already exists and has been paid for.

use crate::error::{Error, Result};
use crate::payment::cache::XorName;
use autonomi::{ChunkAddress, Client as AutonomiClient, Multiaddr};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for the autonomi verifier.
#[derive(Debug, Clone)]
pub struct AutonomVerifierConfig {
    /// Bootstrap peers for connecting to the autonomi network.
    pub bootstrap_peers: Vec<String>,
    /// Timeout for network queries.
    pub query_timeout: Duration,
    /// Whether to enable the verifier (false = always require payment).
    pub enabled: bool,
}

impl Default for AutonomVerifierConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: Vec::new(),
            query_timeout: Duration::from_secs(30),
            enabled: true,
        }
    }
}

/// Verifies if data exists on the autonomi network.
///
/// This is used to determine if data has already been paid for on the
/// legacy network and can be stored for free on saorsa.
pub struct AutonomVerifier {
    config: AutonomVerifierConfig,
    /// Autonomi network client.
    client: Option<AutonomiClient>,
}

impl AutonomVerifier {
    /// Create a new autonomi verifier with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the autonomi client fails to initialize.
    pub async fn new(config: AutonomVerifierConfig) -> Result<Self> {
        if !config.enabled {
            info!("Autonomi verifier disabled - all data will require payment");
            return Ok(Self {
                config,
                client: None,
            });
        }

        if config.bootstrap_peers.is_empty() {
            warn!("No autonomi bootstrap peers configured - verifier disabled");
            return Ok(Self {
                config: AutonomVerifierConfig {
                    enabled: false,
                    ..config
                },
                client: None,
            });
        }

        // Parse bootstrap peers as multiaddrs
        let peers: Vec<Multiaddr> = config
            .bootstrap_peers
            .iter()
            .filter_map(|peer| {
                peer.parse().ok().or_else(|| {
                    warn!("Failed to parse bootstrap peer: {}", peer);
                    None
                })
            })
            .collect();

        if peers.is_empty() {
            warn!("No valid autonomi bootstrap peers - verifier disabled");
            return Ok(Self {
                config: AutonomVerifierConfig {
                    enabled: false,
                    ..config
                },
                client: None,
            });
        }

        // Initialize autonomi client
        let client = AutonomiClient::init_with_peers(peers)
            .await
            .map_err(|e| Error::Network(format!("Failed to connect to autonomi: {e}")))?;

        info!(
            "Autonomi verifier initialized with {} bootstrap peers",
            config.bootstrap_peers.len()
        );

        Ok(Self {
            config,
            client: Some(client),
        })
    }

    /// Check if data with the given `XorName` exists on the autonomi network.
    ///
    /// # Arguments
    ///
    /// * `xorname` - The content-addressed name (hash) of the data
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Data exists on autonomi (already paid for)
    /// * `Ok(false)` - Data does not exist on autonomi (requires payment)
    /// * `Err(_)` - Network error (caller should decide how to handle)
    ///
    /// # Errors
    ///
    /// Returns an error if the network query fails.
    pub async fn data_exists(&self, xorname: &XorName) -> Result<bool> {
        if !self.config.enabled {
            debug!("Autonomi verifier disabled, returning false");
            return Ok(false);
        }

        debug!(
            "Checking if data exists on autonomi: {}",
            hex::encode(xorname)
        );

        if let Some(client) = &self.client {
            let addr = ChunkAddress::new(autonomi::XorName(*xorname));
            match tokio::time::timeout(self.config.query_timeout, client.chunk_get(&addr)).await {
                Ok(Ok(_chunk)) => {
                    info!("Data {} exists on autonomi", hex::encode(xorname));
                    Ok(true)
                }
                Ok(Err(autonomi::client::GetError::RecordNotFound)) => {
                    debug!("Data {} not found on autonomi", hex::encode(xorname));
                    Ok(false)
                }
                Ok(Err(e)) => {
                    warn!("Autonomi query error: {e}");
                    Err(Error::Network(format!("Autonomi query failed: {e}")))
                }
                Err(_) => {
                    warn!("Autonomi query timed out");
                    Err(Error::Network("Autonomi query timed out".to_string()))
                }
            }
        } else {
            debug!("No autonomi client, returning false");
            Ok(false)
        }
    }

    /// Check if the verifier is enabled and connected.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the number of bootstrap peers.
    #[must_use]
    pub fn bootstrap_peer_count(&self) -> usize {
        self.config.bootstrap_peers.len()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disabled_verifier() {
        let config = AutonomVerifierConfig {
            enabled: false,
            ..Default::default()
        };
        let verifier = AutonomVerifier::new(config).await.expect("should create");

        assert!(!verifier.is_enabled());

        let xorname = [1u8; 32];
        let result = verifier.data_exists(&xorname).await.expect("should succeed");
        assert!(!result); // Disabled verifier always returns false
    }

    #[tokio::test]
    async fn test_no_bootstrap_peers() {
        let config = AutonomVerifierConfig {
            enabled: true,
            bootstrap_peers: Vec::new(),
            ..Default::default()
        };
        let verifier = AutonomVerifier::new(config).await.expect("should create");

        // Should be disabled due to no bootstrap peers
        assert!(!verifier.is_enabled());
    }

    #[tokio::test]
    async fn test_verifier_with_peers() {
        // Test that peer address validation works
        let config = AutonomVerifierConfig {
            enabled: true,
            bootstrap_peers: vec!["/ip4/127.0.0.1/udp/12000/quic-v1".to_string()],
            ..Default::default()
        };
        // This will attempt to connect, which may fail in test environment
        // We just verify that the config is properly validated
        let result = AutonomVerifier::new(config).await;
        // Either it succeeds and is enabled, or it fails with a network error
        // (which is expected when there's no real peer)
        match result {
            Ok(verifier) => {
                // If it succeeds, it should be enabled
                assert!(verifier.is_enabled());
                assert_eq!(verifier.bootstrap_peer_count(), 1);
            }
            Err(e) => {
                // Network error is expected when no real peer exists
                assert!(
                    e.to_string().contains("autonomi")
                        || e.to_string().contains("connect")
                        || e.to_string().contains("network"),
                    "Expected network-related error, got: {e}"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_peer_address() {
        // Test that invalid multiaddr is handled gracefully
        let config = AutonomVerifierConfig {
            enabled: true,
            bootstrap_peers: vec!["not-a-valid-multiaddr".to_string()],
            ..Default::default()
        };
        let verifier = AutonomVerifier::new(config).await.expect("should create");

        // Should be disabled because the peer address was invalid
        assert!(!verifier.is_enabled());
    }
}
