//! Data type E2E tests for saorsa-node.
//!
//! This module contains comprehensive tests for all four saorsa data types:
//! - **Chunk**: Immutable, content-addressed data (up to 4MB)
//! - **Scratchpad**: Mutable, owner-indexed with counter versioning (up to 4MB)
//! - **Pointer**: Lightweight mutable pointers to other addresses
//! - **`GraphEntry`**: DAG entries with parent links and multi-owner support
//!
//! ## Test Categories
//!
//! Each data type has tests covering:
//! 1. **Basic Operations**: Store and retrieve
//! 2. **Payment Verification**: EVM payment proofs
//! 3. **Signature Validation**: ML-DSA-65 signature verification
//! 4. **Replication**: Cross-node retrieval
//! 5. **Edge Cases**: Max size, empty data, etc.
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all data type tests (requires testnet)
//! cargo test --test e2e data_types -- --ignored
//!
//! # Run specific data type tests
//! cargo test --test e2e chunk -- --ignored
//! cargo test --test e2e scratchpad -- --ignored
//! cargo test --test e2e pointer -- --ignored
//! cargo test --test e2e graph_entry -- --ignored
//! ```

mod chunk;
mod graph_entry;
mod pointer;
mod scratchpad;

/// Test data generator for consistent test fixtures.
pub struct TestData;

impl TestData {
    /// Generate test data of specified size.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn generate(size: usize) -> Vec<u8> {
        (0..size).map(|i| (i % 256) as u8).collect()
    }

    /// Generate a unique identifier for test isolation.
    #[must_use]
    pub fn unique_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("test-{nanos}")
    }

    /// Generate a test owner public key (32 bytes).
    #[must_use]
    pub fn test_owner() -> [u8; 32] {
        let mut owner = [0u8; 32];
        let id = Self::unique_id();
        let bytes = id.as_bytes();
        let len = bytes.len().min(32);
        owner[..len].copy_from_slice(&bytes[..len]);
        owner
    }
}

/// Maximum chunk size (4MB).
pub const MAX_CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Maximum scratchpad size (4MB).
pub const MAX_SCRATCHPAD_SIZE: usize = 4 * 1024 * 1024;

/// Maximum graph entry size (100KB).
pub const MAX_GRAPH_ENTRY_SIZE: usize = 100 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generation() {
        let data = TestData::generate(100);
        assert_eq!(data.len(), 100);
        assert_eq!(data[0], 0);
        assert_eq!(data[99], 99);
    }

    #[test]
    fn test_unique_id() {
        let id1 = TestData::unique_id();
        let id2 = TestData::unique_id();
        // IDs should be unique (with nanosecond precision, very unlikely to collide)
        assert!(id1.starts_with("test-"));
        assert!(id2.starts_with("test-"));
    }

    #[test]
    fn test_owner_generation() {
        let owner = TestData::test_owner();
        assert_eq!(owner.len(), 32);
    }
}
