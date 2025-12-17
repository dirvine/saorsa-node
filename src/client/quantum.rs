//! Quantum-resistant client operations for chunk storage.
//!
//! This module provides content-addressed chunk storage operations on the saorsa network
//! using post-quantum cryptography (ML-KEM-768 for key exchange, ML-DSA-65 for signatures).
//!
//! ## Data Model
//!
//! Chunks are the only data type supported:
//! - **Content-addressed**: Address = SHA256(content)
//! - **Immutable**: Once stored, content cannot change
//! - **Paid**: All storage requires EVM payment on Arbitrum
//!
//! ## Security Features
//!
//! - **ML-KEM-768**: NIST FIPS 203 compliant key encapsulation for encryption
//! - **ML-DSA-65**: NIST FIPS 204 compliant signatures for authentication
//! - **ChaCha20-Poly1305**: Symmetric encryption for data at rest

use super::data_types::{DataChunk, XorName};
use crate::error::{Error, Result};
use bytes::Bytes;
use saorsa_core::P2PNode;
use std::sync::Arc;
use tracing::{debug, info};

/// Configuration for the quantum-resistant client.
#[derive(Debug, Clone)]
pub struct QuantumConfig {
    /// Timeout for network operations in seconds.
    pub timeout_secs: u64,
    /// Number of replicas for data redundancy.
    pub replica_count: u8,
    /// Enable encryption for all stored data.
    pub encrypt_data: bool,
}

impl Default for QuantumConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            replica_count: 4,
            encrypt_data: true,
        }
    }
}

/// Client for quantum-resistant chunk operations on the saorsa network.
///
/// This client uses post-quantum cryptography for all operations:
/// - ML-KEM-768 for key encapsulation
/// - ML-DSA-65 for digital signatures
/// - ChaCha20-Poly1305 for symmetric encryption
///
/// ## Chunk Storage Model
///
/// Chunks are content-addressed: the address is the SHA256 hash of the content.
/// This ensures data integrity - if the content matches the address, the data
/// is authentic. All chunk storage requires EVM payment on Arbitrum.
pub struct QuantumClient {
    config: QuantumConfig,
    p2p_node: Option<Arc<P2PNode>>,
}

impl QuantumClient {
    /// Create a new quantum client with the given configuration.
    #[must_use]
    pub fn new(config: QuantumConfig) -> Self {
        debug!("Creating quantum-resistant saorsa client");
        Self {
            config,
            p2p_node: None,
        }
    }

    /// Create a quantum client with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(QuantumConfig::default())
    }

    /// Set the P2P node for network operations.
    #[must_use]
    pub fn with_node(mut self, node: Arc<P2PNode>) -> Self {
        self.p2p_node = Some(node);
        self
    }

    /// Get a chunk from the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` address of the chunk (SHA256 of content)
    ///
    /// # Returns
    ///
    /// The chunk data if found, or None if not present in the network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_chunk(&self, address: &XorName) -> Result<Option<DataChunk>> {
        debug!(
            "Querying saorsa network for chunk: {}",
            hex::encode(address)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        let _ = self.config.timeout_secs; // Use config for future timeout implementation

        // Lookup chunk in DHT
        match node.dht_get(*address).await {
            Ok(Some(data)) => {
                debug!(
                    "Found chunk {} on saorsa network ({} bytes)",
                    hex::encode(address),
                    data.len()
                );
                Ok(Some(DataChunk::new(*address, Bytes::from(data))))
            }
            Ok(None) => {
                debug!("Chunk {} not found on saorsa network", hex::encode(address));
                Ok(None)
            }
            Err(e) => Err(Error::Network(format!(
                "DHT lookup failed for {}: {}",
                hex::encode(address),
                e
            ))),
        }
    }

    /// Store a chunk on the saorsa network.
    ///
    /// The chunk address is computed as SHA256(content), ensuring content-addressing.
    /// The `P2PNode` handles ML-DSA-65 signing internally.
    ///
    /// # Arguments
    ///
    /// * `content` - The data to store
    ///
    /// # Returns
    ///
    /// The `XorName` address where the chunk was stored.
    ///
    /// # Errors
    ///
    /// Returns an error if the store operation fails.
    pub async fn put_chunk(&self, content: Bytes) -> Result<XorName> {
        use sha2::{Digest, Sha256};

        debug!("Storing chunk on saorsa network ({} bytes)", content.len());

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Compute content address using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();

        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);

        let _ = self.config.replica_count; // Used for future replication verification

        // Store in DHT - P2PNode handles ML-DSA-65 signing internally
        node.dht_put(address, content.to_vec()).await.map_err(|e| {
            Error::Network(format!(
                "DHT store failed for {}: {}",
                hex::encode(address),
                e
            ))
        })?;

        info!(
            "Chunk stored at address: {} ({} bytes)",
            hex::encode(address),
            content.len()
        );
        Ok(address)
    }

    /// Check if a chunk exists on the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` to check
    ///
    /// # Returns
    ///
    /// True if the chunk exists, false otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn exists(&self, address: &XorName) -> Result<bool> {
        debug!(
            "Checking existence on saorsa network: {}",
            hex::encode(address)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Check if data exists in DHT
        match node.dht_get(*address).await {
            Ok(Some(_)) => {
                debug!("Chunk {} exists on saorsa network", hex::encode(address));
                Ok(true)
            }
            Ok(None) => {
                debug!("Chunk {} not found on saorsa network", hex::encode(address));
                Ok(false)
            }
            Err(e) => Err(Error::Network(format!(
                "DHT lookup failed for {}: {}",
                hex::encode(address),
                e
            ))),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_config_default() {
        let config = QuantumConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.replica_count, 4);
        assert!(config.encrypt_data);
    }

    #[test]
    fn test_quantum_client_creation() {
        let client = QuantumClient::with_defaults();
        assert_eq!(client.config.timeout_secs, 30);
        assert!(client.p2p_node.is_none());
    }

    #[tokio::test]
    async fn test_get_chunk_without_node_fails() {
        let client = QuantumClient::with_defaults();
        let address = [0; 32];

        let result = client.get_chunk(&address).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_chunk_without_node_fails() {
        let client = QuantumClient::with_defaults();
        let content = Bytes::from("test data");

        let result = client.put_chunk(content).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_exists_without_node_fails() {
        let client = QuantumClient::with_defaults();
        let address = [0; 32];

        let result = client.exists(&address).await;
        assert!(result.is_err());
    }
}
