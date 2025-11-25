//! Autonomi network verifier for checking if data already exists.
//!
//! This module provides functionality to query the autonomi network to determine
//! if content (identified by XorName) already exists and has been paid for.

use crate::error::{Error, Result};
use crate::payment::cache::XorName;
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
    // TODO: Add autonomi::Client when integrated
    // client: Option<autonomi::Client>,
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
                // client: None,
            });
        }

        if config.bootstrap_peers.is_empty() {
            warn!("No autonomi bootstrap peers configured - verifier disabled");
            return Ok(Self {
                config: AutonomVerifierConfig {
                    enabled: false,
                    ..config
                },
                // client: None,
            });
        }

        // TODO: Initialize autonomi client
        // let client = autonomi::Client::connect(&config.bootstrap_peers).await
        //     .map_err(|e| Error::Network(format!("Failed to connect to autonomi: {e}")))?;

        info!(
            "Autonomi verifier initialized with {} bootstrap peers",
            config.bootstrap_peers.len()
        );

        Ok(Self {
            config,
            // client: Some(client),
        })
    }

    /// Check if data with the given XorName exists on the autonomi network.
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

        debug!("Checking if data exists on autonomi: {}", hex::encode(xorname));

        // TODO: Implement actual autonomi query when client is integrated
        // match &self.client {
        //     Some(client) => {
        //         let addr = ChunkAddress::new(*xorname);
        //         match tokio::time::timeout(
        //             self.config.query_timeout,
        //             client.chunk_get(&addr)
        //         ).await {
        //             Ok(Ok(_chunk)) => {
        //                 info!("Data {} exists on autonomi", hex::encode(xorname));
        //                 Ok(true)
        //             }
        //             Ok(Err(GetError::RecordNotFound)) => {
        //                 debug!("Data {} not found on autonomi", hex::encode(xorname));
        //                 Ok(false)
        //             }
        //             Ok(Err(e)) => {
        //                 warn!("Autonomi query error: {e}");
        //                 Err(Error::Network(format!("Autonomi query failed: {e}")))
        //             }
        //             Err(_) => {
        //                 warn!("Autonomi query timed out");
        //                 Err(Error::Network("Autonomi query timed out".to_string()))
        //             }
        //         }
        //     }
        //     None => {
        //         debug!("No autonomi client, returning false");
        //         Ok(false)
        //     }
        // }

        // Placeholder: Return false (data not found) until autonomi client is integrated
        debug!(
            "Autonomi client not yet integrated, returning false for {}",
            hex::encode(xorname)
        );
        Ok(false)
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
        let config = AutonomVerifierConfig {
            enabled: true,
            bootstrap_peers: vec!["127.0.0.1:12000".to_string()],
            ..Default::default()
        };
        let verifier = AutonomVerifier::new(config).await.expect("should create");

        assert!(verifier.is_enabled());
        assert_eq!(verifier.bootstrap_peer_count(), 1);
    }
}
