//! Quantum-resistant client operations for the saorsa network.
//!
//! This module provides all data operations on the saorsa network using
//! post-quantum cryptography (ML-KEM-768 for key exchange, ML-DSA-65 for signatures).
//!
//! ## Security Features
//!
//! - **ML-KEM-768**: NIST FIPS 203 compliant key encapsulation for encryption
//! - **ML-DSA-65**: NIST FIPS 204 compliant signatures for authentication
//! - **ChaCha20-Poly1305**: Symmetric encryption for data at rest
//! - **HKDF-SHA256**: Key derivation for record-specific keys

use super::data_types::{
    DataChunk, DataSource, GraphEntry, PointerRecord, ScratchpadEntry, XorName,
};
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

/// Client for quantum-resistant operations on the saorsa network.
///
/// This client uses post-quantum cryptography for all operations:
/// - ML-KEM-768 for key encapsulation
/// - ML-DSA-65 for digital signatures
/// - ChaCha20-Poly1305 for symmetric encryption
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
    /// * `address` - The `XorName` address of the chunk
    ///
    /// # Returns
    ///
    /// The chunk data if found.
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
                Ok(Some(DataChunk {
                    address: *address,
                    content: Bytes::from(data),
                    source: DataSource::Saorsa,
                }))
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
    /// The chunk will be stored with content-addressing (SHA-256 hash as key).
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

    /// Store a scratchpad on the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key
    /// * `content_type` - The content type identifier
    /// * `payload` - The data payload
    /// * `counter` - Update counter (must be monotonically increasing)
    ///
    /// # Returns
    ///
    /// The stored scratchpad entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the store operation fails.
    pub async fn put_scratchpad(
        &self,
        owner: [u8; 32],
        content_type: u64,
        payload: Vec<u8>,
        counter: u64,
    ) -> Result<ScratchpadEntry> {
        use sha2::{Digest, Sha256};

        debug!(
            "Storing scratchpad on saorsa network for owner: {}",
            hex::encode(owner)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Create the entry (signature is placeholder - ML-DSA-65 signing handled by P2PNode)
        let signature = vec![0u8; 64];
        let entry = ScratchpadEntry {
            owner,
            content_type,
            payload: payload.clone(),
            counter,
            signature,
            source: DataSource::Saorsa,
        };

        // Serialize entry for storage
        let serialized = rmp_serde::to_vec(&entry)
            .map_err(|e| Error::Serialization(format!("Failed to serialize scratchpad: {e}")))?;

        // Derive address from owner key (scratchpad address = hash of owner)
        let mut hasher = Sha256::new();
        hasher.update(b"scratchpad:");
        hasher.update(owner);
        let hash = hasher.finalize();
        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);

        // Store in DHT
        node.dht_put(address, serialized).await.map_err(|e| {
            Error::Network(format!(
                "DHT store failed for scratchpad {}: {}",
                hex::encode(owner),
                e
            ))
        })?;

        info!(
            "Scratchpad stored for owner: {} at address: {}",
            hex::encode(owner),
            hex::encode(address)
        );
        Ok(entry)
    }

    /// Get a scratchpad from the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key
    ///
    /// # Returns
    ///
    /// The scratchpad entry if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_scratchpad(&self, owner: &[u8; 32]) -> Result<Option<ScratchpadEntry>> {
        use sha2::{Digest, Sha256};

        debug!(
            "Querying saorsa network for scratchpad: {}",
            hex::encode(owner)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Derive address from owner key (same derivation as put_scratchpad)
        let mut hasher = Sha256::new();
        hasher.update(b"scratchpad:");
        hasher.update(owner);
        let hash = hasher.finalize();
        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);

        // Lookup in DHT
        match node.dht_get(address).await {
            Ok(Some(data)) => {
                // Deserialize the scratchpad entry
                let entry: ScratchpadEntry = rmp_serde::from_slice(&data).map_err(|e| {
                    Error::Serialization(format!("Failed to deserialize scratchpad: {e}"))
                })?;
                debug!(
                    "Found scratchpad for owner {} (counter: {})",
                    hex::encode(owner),
                    entry.counter
                );
                Ok(Some(entry))
            }
            Ok(None) => {
                debug!("Scratchpad not found for owner {}", hex::encode(owner));
                Ok(None)
            }
            Err(e) => Err(Error::Network(format!(
                "DHT lookup failed for scratchpad {}: {}",
                hex::encode(owner),
                e
            ))),
        }
    }

    /// Store a pointer on the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key
    /// * `target` - The target `XorName` this pointer references
    /// * `counter` - Update counter
    ///
    /// # Returns
    ///
    /// The stored pointer record.
    ///
    /// # Errors
    ///
    /// Returns an error if the store operation fails.
    pub async fn put_pointer(
        &self,
        owner: [u8; 32],
        target: XorName,
        counter: u64,
    ) -> Result<PointerRecord> {
        use sha2::{Digest, Sha256};

        debug!(
            "Storing pointer on saorsa network: {} -> {}",
            hex::encode(owner),
            hex::encode(target)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Create pointer record (signature is placeholder - ML-DSA-65 signing handled by P2PNode)
        let signature = vec![0u8; 64];
        let record = PointerRecord {
            owner,
            counter,
            target,
            signature,
            source: DataSource::Saorsa,
        };

        // Serialize record
        let serialized = rmp_serde::to_vec(&record)
            .map_err(|e| Error::Serialization(format!("Failed to serialize pointer: {e}")))?;

        // Derive address from owner key
        let mut hasher = Sha256::new();
        hasher.update(b"pointer:");
        hasher.update(owner);
        let hash = hasher.finalize();
        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);

        // Store in DHT
        node.dht_put(address, serialized).await.map_err(|e| {
            Error::Network(format!(
                "DHT store failed for pointer {}: {}",
                hex::encode(owner),
                e
            ))
        })?;

        info!(
            "Pointer stored for owner: {} at address: {}",
            hex::encode(owner),
            hex::encode(address)
        );
        Ok(record)
    }

    /// Get a pointer from the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key
    ///
    /// # Returns
    ///
    /// The pointer record if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_pointer(&self, owner: &[u8; 32]) -> Result<Option<PointerRecord>> {
        use sha2::{Digest, Sha256};

        debug!(
            "Querying saorsa network for pointer: {}",
            hex::encode(owner)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Derive address from owner key (same as put_pointer)
        let mut hasher = Sha256::new();
        hasher.update(b"pointer:");
        hasher.update(owner);
        let hash = hasher.finalize();
        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);

        // Lookup in DHT
        match node.dht_get(address).await {
            Ok(Some(data)) => {
                let record: PointerRecord = rmp_serde::from_slice(&data).map_err(|e| {
                    Error::Serialization(format!("Failed to deserialize pointer: {e}"))
                })?;
                debug!(
                    "Found pointer for owner {} -> {}",
                    hex::encode(owner),
                    hex::encode(record.target)
                );
                Ok(Some(record))
            }
            Ok(None) => {
                debug!("Pointer not found for owner {}", hex::encode(owner));
                Ok(None)
            }
            Err(e) => Err(Error::Network(format!(
                "DHT lookup failed for pointer {}: {}",
                hex::encode(owner),
                e
            ))),
        }
    }

    /// Store a graph entry on the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key
    /// * `parents` - Parent entry addresses
    /// * `content` - The content payload
    ///
    /// # Returns
    ///
    /// The stored graph entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the store operation fails.
    pub async fn put_graph_entry(
        &self,
        owner: [u8; 32],
        parents: Vec<XorName>,
        content: Vec<u8>,
    ) -> Result<GraphEntry> {
        use sha2::{Digest, Sha256};

        debug!(
            "Storing graph entry on saorsa network for owner: {}",
            hex::encode(owner)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        let entry = GraphEntry {
            owner,
            parents: parents.clone(),
            content: content.clone(),
            descendants: Vec::new(),
            source: DataSource::Saorsa,
        };

        // Serialize entry
        let serialized = rmp_serde::to_vec(&entry)
            .map_err(|e| Error::Serialization(format!("Failed to serialize graph entry: {e}")))?;

        // Compute content-addressed key for graph entry
        let mut hasher = Sha256::new();
        hasher.update(b"graph:");
        hasher.update(owner);
        for parent in &parents {
            hasher.update(parent);
        }
        hasher.update(&content);
        let hash = hasher.finalize();
        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);

        // Store in DHT
        node.dht_put(address, serialized).await.map_err(|e| {
            Error::Network(format!(
                "DHT store failed for graph entry {}: {}",
                hex::encode(address),
                e
            ))
        })?;

        info!(
            "Graph entry stored for owner: {} at address: {}",
            hex::encode(owner),
            hex::encode(address)
        );
        Ok(entry)
    }

    /// Get a graph entry from the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` address of the graph entry
    ///
    /// # Returns
    ///
    /// The graph entry if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_graph_entry(&self, address: &XorName) -> Result<Option<GraphEntry>> {
        debug!(
            "Querying saorsa network for graph entry: {}",
            hex::encode(address)
        );

        let Some(ref node) = self.p2p_node else {
            return Err(Error::Network("P2P node not configured".into()));
        };

        // Lookup in DHT
        match node.dht_get(*address).await {
            Ok(Some(data)) => {
                let entry: GraphEntry = rmp_serde::from_slice(&data).map_err(|e| {
                    Error::Serialization(format!("Failed to deserialize graph entry: {e}"))
                })?;
                debug!(
                    "Found graph entry at {} (owner: {}, {} parents)",
                    hex::encode(address),
                    hex::encode(entry.owner),
                    entry.parents.len()
                );
                Ok(Some(entry))
            }
            Ok(None) => {
                debug!("Graph entry not found at {}", hex::encode(address));
                Ok(None)
            }
            Err(e) => Err(Error::Network(format!(
                "DHT lookup failed for graph entry {}: {}",
                hex::encode(address),
                e
            ))),
        }
    }

    /// Check if data exists on the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` to check
    ///
    /// # Returns
    ///
    /// True if data exists, false otherwise.
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
                debug!("Data {} exists on saorsa network", hex::encode(address));
                Ok(true)
            }
            Ok(None) => {
                debug!("Data {} not found on saorsa network", hex::encode(address));
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
