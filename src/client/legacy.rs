//! Legacy autonomi network client operations.
//!
//! This module provides read-only access to the autonomi network for retrieving
//! legacy data. All retrieved data can be migrated to the saorsa network with
//! quantum-resistant cryptography.
//!
//! ## Design Philosophy
//!
//! - **Read-only for legacy data**: We only retrieve from autonomi, never write
//! - **Trust existing verification**: autonomi network already verified BLS signatures
//! - **Migration path**: Retrieved data is re-encrypted with PQC before storing on saorsa
//!
//! ## API Notes
//!
//! The autonomi network uses BLS public keys (48 bytes) for scratchpad, pointer, and
//! graph entry addresses. Our unified types use 32-byte arrays for compatibility with
//! the quantum-safe saorsa network. Therefore:
//!
//! - `get_chunk` and `exists` work directly (`XorName` is 32 bytes)
//! - `get_scratchpad`, `get_pointer`, `get_graph_entry` require valid 48-byte BLS keys
//!   passed as the first 48 bytes of a larger buffer, or return None if key parsing fails

use super::data_types::{
    DataChunk, DataSource, GraphEntry, PointerRecord, ScratchpadEntry, XorName,
};
use crate::error::{Error, Result};
use autonomi::{ChunkAddress, Client as AutonomiClient, Multiaddr};
use bytes::Bytes;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for the legacy autonomi client.
#[derive(Debug, Clone)]
pub struct LegacyConfig {
    /// Timeout for network operations in seconds.
    pub timeout_secs: u64,
    /// Number of retries for failed operations.
    pub max_retries: u32,
    /// Enable parallel queries for faster lookups.
    pub parallel_queries: bool,
    /// Bootstrap peers for connecting to the autonomi network.
    pub bootstrap_peers: Vec<String>,
    /// Whether the client is enabled.
    pub enabled: bool,
}

impl Default for LegacyConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
            parallel_queries: true,
            bootstrap_peers: Vec::new(),
            enabled: true,
        }
    }
}

/// Client for read-only access to the legacy autonomi network.
///
/// This client is used to retrieve data that was stored on the autonomi network
/// before the migration to saorsa. It does not perform BLS signature verification
/// as that is handled by the autonomi network itself.
pub struct LegacyClient {
    config: LegacyConfig,
    /// The autonomi network client.
    client: Option<AutonomiClient>,
}

impl LegacyClient {
    /// Create a new legacy client with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the autonomi client fails to initialize.
    pub async fn new(config: LegacyConfig) -> Result<Self> {
        if !config.enabled {
            info!("Legacy autonomi client disabled");
            return Ok(Self {
                config,
                client: None,
            });
        }

        if config.bootstrap_peers.is_empty() {
            warn!("No autonomi bootstrap peers configured - legacy client disabled");
            return Ok(Self {
                config: LegacyConfig {
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
                    warn!("Failed to parse autonomi bootstrap peer: {}", peer);
                    None
                })
            })
            .collect();

        if peers.is_empty() {
            warn!("No valid autonomi bootstrap peers - legacy client disabled");
            return Ok(Self {
                config: LegacyConfig {
                    enabled: false,
                    ..config
                },
                client: None,
            });
        }

        // Initialize autonomi client
        debug!("Initializing autonomi client with {} peers", peers.len());
        let client = AutonomiClient::init_with_peers(peers)
            .await
            .map_err(|e| Error::Network(format!("Failed to connect to autonomi: {e}")))?;

        info!(
            "Legacy autonomi client initialized with {} bootstrap peers",
            config.bootstrap_peers.len()
        );

        Ok(Self {
            config,
            client: Some(client),
        })
    }

    /// Create a legacy client with default configuration (disabled, no peers).
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub async fn with_defaults() -> Result<Self> {
        Self::new(LegacyConfig::default()).await
    }

    /// Check if the legacy client is enabled and connected.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.client.is_some()
    }

    /// Get the query timeout duration.
    fn timeout(&self) -> Duration {
        Duration::from_secs(self.config.timeout_secs)
    }

    /// Get a chunk from the autonomi network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` address of the chunk
    ///
    /// # Returns
    ///
    /// The chunk data if found, or None if not present on the network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_chunk(&self, address: &XorName) -> Result<Option<DataChunk>> {
        debug!(
            "Querying autonomi network for chunk: {}",
            hex::encode(address)
        );

        let Some(ref client) = self.client else {
            debug!("Legacy client not connected, returning None");
            return Ok(None);
        };

        // Create chunk address from XorName
        let chunk_addr = ChunkAddress::new(autonomi::XorName(*address));

        // Query the autonomi network with timeout
        match tokio::time::timeout(self.timeout(), client.chunk_get(&chunk_addr)).await {
            Ok(Ok(chunk)) => {
                debug!(
                    "Retrieved chunk {} from autonomi ({} bytes)",
                    hex::encode(address),
                    chunk.value.len()
                );
                Ok(Some(wrap_autonomi_chunk(*address, chunk.value)))
            }
            Ok(Err(autonomi::client::GetError::RecordNotFound)) => {
                debug!(
                    "Chunk {} not found on autonomi network",
                    hex::encode(address)
                );
                Ok(None)
            }
            Ok(Err(e)) => {
                warn!("Autonomi chunk query error: {e}");
                Err(Error::Network(format!("Autonomi query failed: {e}")))
            }
            Err(_) => {
                warn!("Autonomi chunk query timed out");
                Err(Error::Network("Autonomi query timed out".to_string()))
            }
        }
    }

    /// Get a scratchpad from the autonomi network.
    ///
    /// **Note:** Autonomi uses BLS public keys (48 bytes) for scratchpad addresses.
    /// This method accepts 32 bytes for API compatibility with our unified types,
    /// but will return `None` because a valid 48-byte BLS key cannot be derived
    /// from 32 bytes.
    ///
    /// For actual scratchpad retrieval from autonomi, use the autonomi client directly
    /// with a proper BLS public key.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's identifier (32 bytes) - Note: BLS keys are 48 bytes
    ///
    /// # Returns
    ///
    /// Always returns `None` due to key format incompatibility.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    #[allow(clippy::unused_async)]
    pub async fn get_scratchpad(&self, owner: &[u8; 32]) -> Result<Option<ScratchpadEntry>> {
        debug!(
            "Querying autonomi network for scratchpad: {}",
            hex::encode(owner)
        );

        if self.client.is_none() {
            debug!("Legacy client not connected, returning None");
            return Ok(None);
        }

        // Autonomi uses BLS public keys (48 bytes) for scratchpad addresses.
        // Our API passes 32-byte arrays for compatibility with saorsa network.
        // Since we cannot construct a valid 48-byte BLS key from 32 bytes,
        // this method returns None. For actual scratchpad retrieval, use
        // the autonomi client directly with proper BLS keys.
        debug!(
            "Scratchpad retrieval requires 48-byte BLS key, got 32 bytes. \
             Use autonomi client directly for scratchpad access."
        );
        Ok(None)
    }

    /// Get a pointer from the autonomi network.
    ///
    /// **Note:** Autonomi uses BLS public keys (48 bytes) for pointer addresses.
    /// This method accepts 32 bytes for API compatibility with our unified types,
    /// but will return `None` because a valid 48-byte BLS key cannot be derived
    /// from 32 bytes.
    ///
    /// For actual pointer retrieval from autonomi, use the autonomi client directly
    /// with a proper BLS public key.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's identifier (32 bytes) - Note: BLS keys are 48 bytes
    ///
    /// # Returns
    ///
    /// Always returns `None` due to key format incompatibility.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    #[allow(clippy::unused_async)]
    pub async fn get_pointer(&self, owner: &[u8; 32]) -> Result<Option<PointerRecord>> {
        debug!(
            "Querying autonomi network for pointer: {}",
            hex::encode(owner)
        );

        if self.client.is_none() {
            debug!("Legacy client not connected, returning None");
            return Ok(None);
        }

        // Autonomi uses BLS public keys (48 bytes) for pointer addresses.
        // Our API passes 32-byte arrays for compatibility with saorsa network.
        // Since we cannot construct a valid 48-byte BLS key from 32 bytes,
        // this method returns None. For actual pointer retrieval, use
        // the autonomi client directly with proper BLS keys.
        debug!(
            "Pointer retrieval requires 48-byte BLS key, got 32 bytes. \
             Use autonomi client directly for pointer access."
        );
        Ok(None)
    }

    /// Get a graph entry from the autonomi network.
    ///
    /// **Note:** Autonomi uses BLS public keys (48 bytes) for graph entry addresses.
    /// This method accepts 32 bytes for API compatibility with our unified types,
    /// but will return `None` because a valid 48-byte BLS key cannot be derived
    /// from 32 bytes.
    ///
    /// For actual graph entry retrieval from autonomi, use the autonomi client directly
    /// with a proper BLS public key.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's identifier (32 bytes) - Note: BLS keys are 48 bytes
    ///
    /// # Returns
    ///
    /// Always returns `None` due to key format incompatibility.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    #[allow(clippy::unused_async)]
    pub async fn get_graph_entry(&self, owner: &[u8; 32]) -> Result<Option<GraphEntry>> {
        debug!(
            "Querying autonomi network for graph entry: {}",
            hex::encode(owner)
        );

        if self.client.is_none() {
            debug!("Legacy client not connected, returning None");
            return Ok(None);
        }

        // Autonomi uses BLS public keys (48 bytes) for graph entry addresses.
        // Our API passes 32-byte arrays for compatibility with saorsa network.
        // Since we cannot construct a valid 48-byte BLS key from 32 bytes,
        // this method returns None. For actual graph entry retrieval, use
        // the autonomi client directly with proper BLS keys.
        debug!(
            "Graph entry retrieval requires 48-byte BLS key, got 32 bytes. \
             Use autonomi client directly for graph entry access."
        );
        Ok(None)
    }

    /// Check if a chunk exists on the autonomi network.
    ///
    /// This is a lightweight check that doesn't retrieve the full data.
    /// For chunks, we use the address as the `XorName`.
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
            "Checking existence on autonomi network: {}",
            hex::encode(address)
        );

        let Some(ref client) = self.client else {
            debug!("Legacy client not connected, returning false");
            return Ok(false);
        };

        // Create chunk address from XorName
        let chunk_addr = ChunkAddress::new(autonomi::XorName(*address));

        // Query the autonomi network with timeout - try to get the chunk
        // This is the most reliable way to check existence
        match tokio::time::timeout(self.timeout(), client.chunk_get(&chunk_addr)).await {
            Ok(Ok(_chunk)) => {
                debug!("Data {} exists on autonomi network", hex::encode(address));
                Ok(true)
            }
            Ok(Err(autonomi::client::GetError::RecordNotFound)) => {
                debug!(
                    "Data {} not found on autonomi network",
                    hex::encode(address)
                );
                Ok(false)
            }
            Ok(Err(e)) => {
                warn!("Autonomi existence check error: {e}");
                Err(Error::Network(format!("Autonomi query failed: {e}")))
            }
            Err(_) => {
                warn!("Autonomi existence check timed out");
                Err(Error::Network("Autonomi query timed out".to_string()))
            }
        }
    }

    /// Get raw data from the autonomi network by address.
    ///
    /// This tries to retrieve any type of data at the given address.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` address
    ///
    /// # Returns
    ///
    /// The raw bytes if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_raw(&self, address: &XorName) -> Result<Option<Bytes>> {
        debug!(
            "Querying autonomi network for raw data: {}",
            hex::encode(address)
        );

        // Try to get as chunk first (most common case)
        if let Some(chunk) = self.get_chunk(address).await? {
            return Ok(Some(chunk.content));
        }

        // Could also try other record types here
        Ok(None)
    }
}

/// Convert autonomi chunk to our unified `DataChunk`.
///
/// This function takes raw autonomi chunk data and wraps it in our unified type.
#[must_use]
pub fn wrap_autonomi_chunk(address: XorName, content: Bytes) -> DataChunk {
    DataChunk::new(address, content, DataSource::Autonomi)
}

/// Convert autonomi scratchpad to our unified `ScratchpadEntry`.
///
/// This function takes raw autonomi scratchpad data and wraps it in our unified type.
///
/// Note: This function is provided for completeness and potential future use when
/// direct access to autonomi scratchpad data with known BLS keys is available.
/// Currently, the `get_scratchpad` method cannot retrieve data due to the BLS
/// key format mismatch (48 bytes vs 32 bytes).
#[must_use]
#[allow(dead_code)]
pub fn wrap_autonomi_scratchpad(
    owner: [u8; 32],
    content_type: u64,
    payload: Vec<u8>,
    counter: u64,
    signature: Vec<u8>,
) -> ScratchpadEntry {
    ScratchpadEntry {
        owner,
        content_type,
        payload,
        counter,
        signature,
        source: DataSource::Autonomi,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_config_default() {
        let config = LegacyConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert!(config.parallel_queries);
        assert!(config.bootstrap_peers.is_empty());
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_legacy_client_creation() {
        // With default config (no bootstrap peers), client is disabled
        let client = LegacyClient::with_defaults().await.expect("should create");
        assert_eq!(client.config.timeout_secs, 30);
        assert!(!client.is_enabled()); // No peers = disabled
    }

    #[tokio::test]
    async fn test_disabled_client() {
        let config = LegacyConfig {
            enabled: false,
            ..Default::default()
        };
        let client = LegacyClient::new(config).await.expect("should create");
        assert!(!client.is_enabled());
    }

    #[test]
    fn test_wrap_autonomi_chunk() {
        let address = [0xAB; 32];
        let content = Bytes::from("test chunk");
        let chunk = wrap_autonomi_chunk(address, content.clone());

        assert_eq!(chunk.address, address);
        assert_eq!(chunk.content, content);
        assert_eq!(chunk.source, DataSource::Autonomi);
    }

    #[test]
    fn test_wrap_autonomi_scratchpad() {
        let owner = [0xCD; 32];
        let scratchpad = wrap_autonomi_scratchpad(owner, 1, vec![1, 2, 3], 42, vec![4, 5, 6]);

        assert_eq!(scratchpad.owner, owner);
        assert_eq!(scratchpad.content_type, 1);
        assert_eq!(scratchpad.counter, 42);
        assert_eq!(scratchpad.source, DataSource::Autonomi);
    }

    #[tokio::test]
    async fn test_get_chunk_returns_none_when_disabled() {
        let client = LegacyClient::with_defaults().await.expect("should create");
        let address = [0; 32];

        // Client is disabled (no bootstrap peers), so returns None
        let result = client.get_chunk(&address).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_exists_returns_false_when_disabled() {
        let client = LegacyClient::with_defaults().await.expect("should create");
        let address = [0; 32];

        // Client is disabled (no bootstrap peers), so returns false
        let result = client.exists(&address).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_invalid_peer_address() {
        // Test that invalid multiaddr is handled gracefully
        let config = LegacyConfig {
            enabled: true,
            bootstrap_peers: vec!["not-a-valid-multiaddr".to_string()],
            ..Default::default()
        };
        let client = LegacyClient::new(config).await.expect("should create");

        // Should be disabled because the peer address was invalid
        assert!(!client.is_enabled());
    }
}
