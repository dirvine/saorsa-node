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

use super::data_types::{DataChunk, DataSource, GraphEntry, PointerRecord, ScratchpadEntry, XorName};
use crate::error::Result;
use bytes::Bytes;
use tracing::debug;

/// Configuration for the legacy autonomi client.
#[derive(Debug, Clone)]
pub struct LegacyConfig {
    /// Timeout for network operations in seconds.
    pub timeout_secs: u64,
    /// Number of retries for failed operations.
    pub max_retries: u32,
    /// Enable parallel queries for faster lookups.
    pub parallel_queries: bool,
}

impl Default for LegacyConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
            parallel_queries: true,
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
    // The actual autonomi client would be stored here
    // For now, we provide the interface without the full implementation
}

impl LegacyClient {
    /// Create a new legacy client with the given configuration.
    #[must_use]
    pub fn new(config: LegacyConfig) -> Self {
        debug!("Creating legacy autonomi client");
        Self { config }
    }

    /// Create a legacy client with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(LegacyConfig::default())
    }

    /// Get a chunk from the autonomi network.
    ///
    /// # Arguments
    ///
    /// * `address` - The XorName address of the chunk
    ///
    /// # Returns
    ///
    /// The chunk data if found, or None if not present on the network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_chunk(&self, address: &XorName) -> Result<Option<DataChunk>> {
        debug!("Querying autonomi network for chunk: {}", hex::encode(address));

        // In a full implementation, this would:
        // 1. Create an autonomi::Client connection
        // 2. Call client.chunk_get(&ChunkAddress::new(*address))
        // 3. Return the chunk data
        //
        // The autonomi network handles BLS verification internally,
        // so we trust the returned data is authentic.

        // Placeholder: Simulates network query
        // Real implementation would connect to autonomi network
        let _ = self.config.timeout_secs; // Use config to avoid warning

        // For now, return None to indicate not found
        // Real implementation would query the network
        debug!("Chunk not found on autonomi network (placeholder)");
        Ok(None)
    }

    /// Get a scratchpad from the autonomi network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key (32 bytes)
    ///
    /// # Returns
    ///
    /// The scratchpad entry if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_scratchpad(&self, owner: &[u8; 32]) -> Result<Option<ScratchpadEntry>> {
        debug!(
            "Querying autonomi network for scratchpad: {}",
            hex::encode(owner)
        );

        // In a full implementation, this would:
        // 1. Create an autonomi::Client connection
        // 2. Call client.scratchpad_get_from_public_key(...)
        // 3. Return the scratchpad data

        let _ = self.config.max_retries; // Use config to avoid warning
        debug!("Scratchpad not found on autonomi network (placeholder)");
        Ok(None)
    }

    /// Get a pointer from the autonomi network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key (32 bytes)
    ///
    /// # Returns
    ///
    /// The pointer record if found.
    ///
    /// # Errors
    ///
    /// Returns an error if the network operation fails.
    pub async fn get_pointer(&self, owner: &[u8; 32]) -> Result<Option<PointerRecord>> {
        debug!(
            "Querying autonomi network for pointer: {}",
            hex::encode(owner)
        );

        // In a full implementation, this would:
        // 1. Create an autonomi::Client connection
        // 2. Call client.pointer_get(...)
        // 3. Return the pointer data

        let _ = self.config.parallel_queries; // Use config to avoid warning
        debug!("Pointer not found on autonomi network (placeholder)");
        Ok(None)
    }

    /// Get a graph entry from the autonomi network.
    ///
    /// # Arguments
    ///
    /// * `address` - The XorName address of the graph entry
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
            "Querying autonomi network for graph entry: {}",
            hex::encode(address)
        );

        // In a full implementation, this would query the autonomi network
        // for graph entry data

        debug!("Graph entry not found on autonomi network (placeholder)");
        Ok(None)
    }

    /// Check if data exists on the autonomi network.
    ///
    /// This is a lightweight check that doesn't retrieve the full data.
    ///
    /// # Arguments
    ///
    /// * `address` - The XorName to check
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

        // In a full implementation, this would use a lightweight existence check
        // rather than retrieving the full data

        Ok(false) // Placeholder
    }

    /// Get raw data from the autonomi network by address.
    ///
    /// This tries to retrieve any type of data at the given address.
    ///
    /// # Arguments
    ///
    /// * `address` - The XorName address
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

/// Convert autonomi chunk to our unified DataChunk.
///
/// This function takes raw autonomi chunk data and wraps it in our unified type.
#[must_use]
#[allow(dead_code)] // Will be used when full autonomi integration is added
pub fn wrap_autonomi_chunk(address: XorName, content: Bytes) -> DataChunk {
    DataChunk::new(address, content, DataSource::Autonomi)
}

/// Convert autonomi scratchpad to our unified ScratchpadEntry.
///
/// This function takes raw autonomi scratchpad data and wraps it in our unified type.
#[must_use]
#[allow(dead_code)] // Will be used when full autonomi integration is added
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
    }

    #[test]
    fn test_legacy_client_creation() {
        let client = LegacyClient::with_defaults();
        assert_eq!(client.config.timeout_secs, 30);
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
    async fn test_get_chunk_returns_none() {
        let client = LegacyClient::with_defaults();
        let address = [0; 32];

        let result = client.get_chunk(&address).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_exists_returns_false() {
        let client = LegacyClient::with_defaults();
        let address = [0; 32];

        let result = client.exists(&address).await.unwrap();
        assert!(!result);
    }
}
