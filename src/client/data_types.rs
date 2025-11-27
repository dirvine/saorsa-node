//! Unified data type definitions for hybrid client operations.
//!
//! This module provides a unified view of data types across both the saorsa network
//! (quantum-resistant) and the legacy autonomi network.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// A content-addressed identifier (32 bytes).
///
/// Used to identify chunks, records, and other content-addressed data.
pub type XorName = [u8; 32];

/// The source network where data was retrieved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DataSource {
    /// Data retrieved from the saorsa network (quantum-resistant).
    #[default]
    Saorsa,
    /// Data retrieved from the legacy autonomi network.
    Autonomi,
    /// Data retrieved from local cache.
    Cache,
}

/// A chunk of data with its address.
#[derive(Debug, Clone)]
pub struct DataChunk {
    /// The content-addressed identifier.
    pub address: XorName,
    /// The raw data content.
    pub content: Bytes,
    /// The source of this data.
    pub source: DataSource,
}

impl DataChunk {
    /// Create a new data chunk.
    #[must_use]
    pub fn new(address: XorName, content: Bytes, source: DataSource) -> Self {
        Self {
            address,
            content,
            source,
        }
    }

    /// Get the size of the chunk in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.content.len()
    }
}

/// A scratchpad entry (mutable data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScratchpadEntry {
    /// The owner's public key (32 bytes).
    pub owner: [u8; 32],
    /// The content type identifier.
    pub content_type: u64,
    /// The encrypted payload.
    pub payload: Vec<u8>,
    /// Counter for updates (prevents replay).
    pub counter: u64,
    /// The signature over the entry.
    pub signature: Vec<u8>,
    /// The source network.
    #[serde(skip)]
    pub source: DataSource,
}

/// A pointer record (points to other data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerRecord {
    /// The owner's public key (32 bytes).
    pub owner: [u8; 32],
    /// The counter for updates.
    pub counter: u64,
    /// The target `XorName` this pointer references.
    pub target: XorName,
    /// The signature over the record.
    pub signature: Vec<u8>,
    /// The source network.
    #[serde(skip)]
    pub source: DataSource,
}

/// A graph entry (linked data structure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntry {
    /// The owner's public key (32 bytes).
    pub owner: [u8; 32],
    /// Parent entries this links to.
    pub parents: Vec<XorName>,
    /// The content payload.
    pub content: Vec<u8>,
    /// Descendant entries (populated on retrieval).
    pub descendants: Vec<XorName>,
    /// The source network.
    #[serde(skip)]
    pub source: DataSource,
}

/// Record type enumeration for discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordKind {
    /// Immutable chunk data.
    Chunk,
    /// Mutable scratchpad.
    Scratchpad,
    /// Pointer record.
    Pointer,
    /// Graph entry.
    GraphEntry,
}

/// Result of a lookup operation.
#[derive(Debug)]
pub enum LookupResult {
    /// Data found as a chunk.
    Chunk(DataChunk),
    /// Data found as a scratchpad.
    Scratchpad(ScratchpadEntry),
    /// Data found as a pointer.
    Pointer(PointerRecord),
    /// Data found as a graph entry.
    GraphEntry(GraphEntry),
    /// Data not found on any network.
    NotFound,
}

impl LookupResult {
    /// Check if data was found.
    #[must_use]
    pub fn is_found(&self) -> bool {
        !matches!(self, Self::NotFound)
    }

    /// Get the data source if found.
    #[must_use]
    pub fn source(&self) -> Option<DataSource> {
        match self {
            Self::Chunk(c) => Some(c.source),
            Self::Scratchpad(s) => Some(s.source),
            Self::Pointer(p) => Some(p.source),
            Self::GraphEntry(g) => Some(g.source),
            Self::NotFound => None,
        }
    }
}

/// Statistics about hybrid operations.
#[derive(Debug, Default, Clone)]
pub struct HybridStats {
    /// Number of lookups from saorsa network.
    pub saorsa_hits: u64,
    /// Number of lookups from autonomi network.
    pub autonomi_hits: u64,
    /// Number of cache hits.
    pub cache_hits: u64,
    /// Number of misses (not found anywhere).
    pub misses: u64,
    /// Number of writes to saorsa network.
    pub saorsa_writes: u64,
    /// Number of migrations from autonomi to saorsa.
    pub migrations: u64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_data_chunk_creation() {
        let address = [0xAB; 32];
        let content = Bytes::from("test data");
        let chunk = DataChunk::new(address, content.clone(), DataSource::Saorsa);

        assert_eq!(chunk.address, address);
        assert_eq!(chunk.content, content);
        assert_eq!(chunk.source, DataSource::Saorsa);
        assert_eq!(chunk.size(), 9);
    }

    #[test]
    fn test_lookup_result_is_found() {
        let chunk = DataChunk::new([0; 32], Bytes::new(), DataSource::Saorsa);
        let found = LookupResult::Chunk(chunk);
        let not_found = LookupResult::NotFound;

        assert!(found.is_found());
        assert!(!not_found.is_found());
    }

    #[test]
    fn test_lookup_result_source() {
        let chunk = DataChunk::new([0; 32], Bytes::new(), DataSource::Autonomi);
        let found = LookupResult::Chunk(chunk);
        let not_found = LookupResult::NotFound;

        assert_eq!(found.source(), Some(DataSource::Autonomi));
        assert_eq!(not_found.source(), None);
    }
}
