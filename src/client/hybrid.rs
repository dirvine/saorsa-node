//! Hybrid client that orchestrates between saorsa and autonomi networks.
//!
//! The `HybridClient` provides a unified interface for data operations that:
//! 1. **Retrieval**: Tries saorsa network first, falls back to autonomi
//! 2. **Storage**: Always uses saorsa network with quantum-resistant cryptography
//! 3. **Migration**: Automatically migrates data from autonomi to saorsa on read
//!
//! ## Architecture
//!
//! ```text
//! GET request
//!     │
//!     ▼
//! ┌─────────────────┐
//! │  Check saorsa   │
//! └────────┬────────┘
//!          │
//!    ┌─────┴─────┐
//!    │           │
//!  FOUND     NOT FOUND
//!    │           │
//!    ▼           ▼
//! Return     ┌─────────────────┐
//!            │ Check autonomi  │
//!            └────────┬────────┘
//!                     │
//!               ┌─────┴─────┐
//!               │           │
//!             FOUND     NOT FOUND
//!               │           │
//!               ▼           ▼
//!         ┌──────────┐   Return
//!         │ Migrate  │   NotFound
//!         │ to saorsa│
//!         └────┬─────┘
//!              │
//!              ▼
//!           Return
//! ```
//!
//! ## PUT Operations
//!
//! ```text
//! PUT request
//!     │
//!     ▼
//! ┌─────────────────┐
//! │ Encrypt with    │
//! │ ML-KEM-768      │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Sign with       │
//! │ ML-DSA-65       │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Store on saorsa │
//! │ network         │
//! └────────┬────────┘
//!          │
//!          ▼
//!       Return
//!       address
//! ```

use super::data_types::{
    DataChunk, DataSource, GraphEntry, HybridStats, LookupResult, PointerRecord, ScratchpadEntry,
    XorName,
};
use super::legacy::{LegacyClient, LegacyConfig};
use super::quantum::{QuantumClient, QuantumConfig};
use crate::error::Result;
use bytes::Bytes;
use parking_lot::RwLock;
use saorsa_core::P2PNode;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Configuration for the hybrid client.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Configuration for quantum-resistant operations.
    pub quantum: QuantumConfig,
    /// Configuration for legacy autonomi operations.
    pub legacy: LegacyConfig,
    /// Whether to auto-migrate data from autonomi on read.
    pub auto_migrate: bool,
    /// Enable caching of lookup results.
    pub enable_cache: bool,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            quantum: QuantumConfig::default(),
            legacy: LegacyConfig::default(),
            auto_migrate: true,
            enable_cache: true,
        }
    }
}

/// A hybrid client that bridges saorsa and autonomi networks.
///
/// This client provides a unified interface for data operations:
/// - GET operations check saorsa first, then fall back to autonomi
/// - PUT operations always use saorsa with quantum-resistant cryptography
/// - Auto-migration copies data from autonomi to saorsa on read
pub struct HybridClient {
    config: HybridConfig,
    quantum: QuantumClient,
    legacy: LegacyClient,
    stats: RwLock<HybridStats>,
}

impl HybridClient {
    /// Create a new hybrid client with the given configuration.
    #[must_use]
    pub fn new(config: HybridConfig) -> Self {
        info!("Creating hybrid client (saorsa + autonomi)");
        Self {
            quantum: QuantumClient::new(config.quantum.clone()),
            legacy: LegacyClient::new(config.legacy.clone()),
            config,
            stats: RwLock::new(HybridStats::default()),
        }
    }

    /// Create a hybrid client with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(HybridConfig::default())
    }

    /// Set the P2P node for saorsa network operations.
    #[must_use]
    pub fn with_node(mut self, node: Arc<P2PNode>) -> Self {
        self.quantum = self.quantum.with_node(node);
        self
    }

    /// Get a chunk from either network.
    ///
    /// Tries saorsa network first, falls back to autonomi if not found.
    /// If found on autonomi and auto-migrate is enabled, the data is
    /// migrated to saorsa with quantum-resistant encryption.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` address of the chunk
    ///
    /// # Returns
    ///
    /// The chunk data and its source, or None if not found anywhere.
    ///
    /// # Errors
    ///
    /// Returns an error if both network operations fail.
    pub async fn get_chunk(&self, address: &XorName) -> Result<Option<DataChunk>> {
        debug!("Hybrid lookup for chunk: {}", hex::encode(address));

        // Try saorsa network first
        match self.quantum.get_chunk(address).await {
            Ok(Some(chunk)) => {
                debug!("Chunk found on saorsa network");
                self.stats.write().saorsa_hits += 1;
                return Ok(Some(chunk));
            }
            Ok(None) => {
                debug!("Chunk not on saorsa, trying autonomi...");
            }
            Err(e) => {
                warn!("Saorsa lookup failed: {e}, trying autonomi...");
            }
        }

        // Fall back to autonomi
        if let Some(chunk) = self.legacy.get_chunk(address).await? {
            debug!("Chunk found on autonomi network");
            self.stats.write().autonomi_hits += 1;

            // Auto-migrate to saorsa if enabled
            if self.config.auto_migrate {
                self.migrate_chunk(&chunk).await?;
            }

            Ok(Some(chunk))
        } else {
            debug!("Chunk not found on any network");
            self.stats.write().misses += 1;
            Ok(None)
        }
    }

    /// Store a chunk on the saorsa network.
    ///
    /// The chunk is encrypted with ML-KEM-768 and signed with ML-DSA-65.
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
        debug!("Storing chunk via hybrid client ({} bytes)", content.len());

        // Always use saorsa network with PQC
        let address = self.quantum.put_chunk(content).await?;
        self.stats.write().saorsa_writes += 1;

        Ok(address)
    }

    /// Get a scratchpad from either network.
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
    /// Returns an error if both network operations fail.
    pub async fn get_scratchpad(&self, owner: &[u8; 32]) -> Result<Option<ScratchpadEntry>> {
        debug!("Hybrid lookup for scratchpad: {}", hex::encode(owner));

        // Try saorsa first
        if let Ok(Some(entry)) = self.quantum.get_scratchpad(owner).await {
            self.stats.write().saorsa_hits += 1;
            return Ok(Some(entry));
        }

        // Fall back to autonomi
        if let Some(entry) = self.legacy.get_scratchpad(owner).await? {
            self.stats.write().autonomi_hits += 1;

            // Auto-migrate
            if self.config.auto_migrate {
                self.migrate_scratchpad(&entry).await?;
            }

            Ok(Some(entry))
        } else {
            self.stats.write().misses += 1;
            Ok(None)
        }
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
        let entry = self
            .quantum
            .put_scratchpad(owner, content_type, payload, counter)
            .await?;
        self.stats.write().saorsa_writes += 1;
        Ok(entry)
    }

    /// Get a pointer from either network.
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
    /// Returns an error if both network operations fail.
    pub async fn get_pointer(&self, owner: &[u8; 32]) -> Result<Option<PointerRecord>> {
        debug!("Hybrid lookup for pointer: {}", hex::encode(owner));

        // Try saorsa first
        if let Ok(Some(record)) = self.quantum.get_pointer(owner).await {
            self.stats.write().saorsa_hits += 1;
            return Ok(Some(record));
        }

        // Fall back to autonomi
        if let Some(record) = self.legacy.get_pointer(owner).await? {
            self.stats.write().autonomi_hits += 1;

            if self.config.auto_migrate {
                self.migrate_pointer(&record).await?;
            }

            Ok(Some(record))
        } else {
            self.stats.write().misses += 1;
            Ok(None)
        }
    }

    /// Store a pointer on the saorsa network.
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner's public key
    /// * `target` - The target `XorName`
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
        let record = self.quantum.put_pointer(owner, target, counter).await?;
        self.stats.write().saorsa_writes += 1;
        Ok(record)
    }

    /// Get a graph entry from either network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` address
    ///
    /// # Returns
    ///
    /// The graph entry if found.
    ///
    /// # Errors
    ///
    /// Returns an error if both network operations fail.
    pub async fn get_graph_entry(&self, address: &XorName) -> Result<Option<GraphEntry>> {
        debug!("Hybrid lookup for graph entry: {}", hex::encode(address));

        // Try saorsa first
        if let Ok(Some(entry)) = self.quantum.get_graph_entry(address).await {
            self.stats.write().saorsa_hits += 1;
            return Ok(Some(entry));
        }

        // Fall back to autonomi
        if let Some(entry) = self.legacy.get_graph_entry(address).await? {
            self.stats.write().autonomi_hits += 1;

            if self.config.auto_migrate {
                self.migrate_graph_entry(&entry).await?;
            }

            Ok(Some(entry))
        } else {
            self.stats.write().misses += 1;
            Ok(None)
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
        let entry = self
            .quantum
            .put_graph_entry(owner, parents, content)
            .await?;
        self.stats.write().saorsa_writes += 1;
        Ok(entry)
    }

    /// Perform a unified lookup that returns the result type.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` to look up
    ///
    /// # Returns
    ///
    /// The lookup result indicating what type of data was found.
    ///
    /// # Errors
    ///
    /// Returns an error if network operations fail.
    pub async fn lookup(&self, address: &XorName) -> Result<LookupResult> {
        // Try as chunk first (most common)
        if let Some(chunk) = self.get_chunk(address).await? {
            return Ok(LookupResult::Chunk(chunk));
        }

        // Try as graph entry
        if let Some(entry) = self.get_graph_entry(address).await? {
            return Ok(LookupResult::GraphEntry(entry));
        }

        Ok(LookupResult::NotFound)
    }

    /// Get current statistics.
    #[must_use]
    pub fn stats(&self) -> HybridStats {
        self.stats.read().clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        *self.stats.write() = HybridStats::default();
    }

    /// Check if data exists on either network.
    ///
    /// # Arguments
    ///
    /// * `address` - The `XorName` to check
    ///
    /// # Returns
    ///
    /// The data source if found, None if not found anywhere.
    ///
    /// # Errors
    ///
    /// Returns an error if network operations fail.
    pub async fn exists(&self, address: &XorName) -> Result<Option<DataSource>> {
        // Check saorsa first
        if matches!(self.quantum.exists(address).await, Ok(true)) {
            return Ok(Some(DataSource::Saorsa));
        }

        // Check autonomi
        if self.legacy.exists(address).await? {
            return Ok(Some(DataSource::Autonomi));
        }

        Ok(None)
    }

    /// Migrate a chunk from autonomi to saorsa.
    async fn migrate_chunk(&self, chunk: &DataChunk) -> Result<()> {
        debug!("Migrating chunk to saorsa: {}", hex::encode(chunk.address));

        // Re-store on saorsa with PQC encryption
        let new_address = self.quantum.put_chunk(chunk.content.clone()).await?;

        if new_address != chunk.address {
            warn!(
                "Migration address mismatch: {} vs {}",
                hex::encode(chunk.address),
                hex::encode(new_address)
            );
        }

        self.stats.write().migrations += 1;
        info!("Migrated chunk: {}", hex::encode(chunk.address));
        Ok(())
    }

    /// Migrate a scratchpad from autonomi to saorsa.
    async fn migrate_scratchpad(&self, entry: &ScratchpadEntry) -> Result<()> {
        debug!(
            "Migrating scratchpad to saorsa: {}",
            hex::encode(entry.owner)
        );

        self.quantum
            .put_scratchpad(
                entry.owner,
                entry.content_type,
                entry.payload.clone(),
                entry.counter,
            )
            .await?;

        self.stats.write().migrations += 1;
        info!("Migrated scratchpad: {}", hex::encode(entry.owner));
        Ok(())
    }

    /// Migrate a pointer from autonomi to saorsa.
    async fn migrate_pointer(&self, record: &PointerRecord) -> Result<()> {
        debug!("Migrating pointer to saorsa: {}", hex::encode(record.owner));

        self.quantum
            .put_pointer(record.owner, record.target, record.counter)
            .await?;

        self.stats.write().migrations += 1;
        info!("Migrated pointer: {}", hex::encode(record.owner));
        Ok(())
    }

    /// Migrate a graph entry from autonomi to saorsa.
    async fn migrate_graph_entry(&self, entry: &GraphEntry) -> Result<()> {
        debug!(
            "Migrating graph entry to saorsa: {}",
            hex::encode(entry.owner)
        );

        self.quantum
            .put_graph_entry(entry.owner, entry.parents.clone(), entry.content.clone())
            .await?;

        self.stats.write().migrations += 1;
        info!("Migrated graph entry: {}", hex::encode(entry.owner));
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert!(config.auto_migrate);
        assert!(config.enable_cache);
        assert_eq!(config.quantum.timeout_secs, 30);
        assert_eq!(config.legacy.timeout_secs, 30);
    }

    #[test]
    fn test_hybrid_client_creation() {
        let client = HybridClient::with_defaults();
        assert!(client.config.auto_migrate);

        let stats = client.stats();
        assert_eq!(stats.saorsa_hits, 0);
        assert_eq!(stats.autonomi_hits, 0);
    }

    #[test]
    fn test_stats_reset() {
        let client = HybridClient::with_defaults();
        {
            let mut stats = client.stats.write();
            stats.saorsa_hits = 10;
            stats.autonomi_hits = 5;
        }

        client.reset_stats();
        let stats = client.stats();
        assert_eq!(stats.saorsa_hits, 0);
        assert_eq!(stats.autonomi_hits, 0);
    }

    #[tokio::test]
    async fn test_get_chunk_returns_none_when_not_found() {
        let client = HybridClient::with_defaults();
        let address = [0; 32];

        // Without a P2P node, quantum client fails, then legacy returns None
        let result = client.get_chunk(&address).await.unwrap();
        assert!(result.is_none());

        let stats = client.stats();
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_lookup_returns_not_found() {
        let client = HybridClient::with_defaults();
        let address = [0; 32];

        let result = client.lookup(&address).await.unwrap();
        assert!(!result.is_found());
    }

    #[tokio::test]
    async fn test_exists_returns_none_when_not_found() {
        let client = HybridClient::with_defaults();
        let address = [0; 32];

        let result = client.exists(&address).await.unwrap();
        assert!(result.is_none());
    }
}
