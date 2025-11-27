//! Chunk data type E2E tests.
//!
//! Chunks are immutable, content-addressed data blocks (up to 4MB).
//! The address is derived from the content hash (SHA256 -> `XorName`).
//!
//! ## Test Coverage
//!
//! - Basic store and retrieve
//! - Content addressing verification
//! - Cross-node replication
//! - Maximum size handling (4MB)
//! - Payment verification
//! - ML-DSA-65 signature verification

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{TestData, MAX_CHUNK_SIZE};

/// Test fixture for chunk operations.
#[allow(clippy::struct_field_names)]
pub struct ChunkTestFixture {
    /// Small test data (1KB).
    pub small: Vec<u8>,
    /// Medium test data (1MB).
    pub medium: Vec<u8>,
    /// Large test data (4MB - max size).
    pub large: Vec<u8>,
}

impl Default for ChunkTestFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkTestFixture {
    /// Create a new test fixture with pre-generated data.
    #[must_use]
    pub fn new() -> Self {
        Self {
            small: TestData::generate(1024),           // 1KB
            medium: TestData::generate(1024 * 1024),   // 1MB
            large: TestData::generate(MAX_CHUNK_SIZE), // 4MB
        }
    }

    /// Compute content address for data (SHA256 hash).
    #[must_use]
    pub fn compute_address(data: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut address = [0u8; 32];
        address.copy_from_slice(&hash);
        address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 1: Content address computation is deterministic
    #[test]
    fn test_content_address_deterministic() {
        let data = TestData::generate(100);
        let addr1 = ChunkTestFixture::compute_address(&data);
        let addr2 = ChunkTestFixture::compute_address(&data);
        assert_eq!(addr1, addr2, "Same data should produce same address");
    }

    /// Test 2: Different data produces different addresses
    #[test]
    fn test_different_data_different_address() {
        let data1 = TestData::generate(100);
        let mut data2 = TestData::generate(100);
        data2[0] = 255; // Modify first byte

        let addr1 = ChunkTestFixture::compute_address(&data1);
        let addr2 = ChunkTestFixture::compute_address(&data2);
        assert_ne!(
            addr1, addr2,
            "Different data should produce different addresses"
        );
    }

    /// Test 3: Empty data has valid address
    #[test]
    fn test_empty_data_address() {
        let addr = ChunkTestFixture::compute_address(&[]);
        // SHA256 of empty string is well-known
        assert_eq!(
            hex::encode(addr),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// Test 4: Fixture creates correct sizes
    #[test]
    fn test_fixture_data_sizes() {
        let fixture = ChunkTestFixture::new();
        assert_eq!(fixture.small.len(), 1024);
        assert_eq!(fixture.medium.len(), 1024 * 1024);
        assert_eq!(fixture.large.len(), MAX_CHUNK_SIZE);
    }

    /// Test 5: Max chunk size constant is correct
    #[test]
    fn test_max_chunk_size() {
        assert_eq!(MAX_CHUNK_SIZE, 4 * 1024 * 1024); // 4MB
    }

    // =========================================================================
    // Integration Tests (require testnet)
    // =========================================================================

    /// Test 6: Store and retrieve small chunk
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_store_retrieve_small() {
        // TODO: Implement with TestHarness when P2P integration is complete
        // let harness = TestHarness::setup().await.unwrap();
        // let fixture = ChunkTestFixture::new();
        //
        // // Store via node 5
        // let address = harness.node(5).store_chunk(&fixture.small_data).await.unwrap();
        //
        // // Retrieve via node 20 (different node)
        // let retrieved = harness.node(20).get_chunk(&address).await.unwrap();
        // assert_eq!(retrieved, fixture.small_data);
        //
        // harness.teardown().await.unwrap();
    }

    /// Test 7: Store and retrieve large chunk (4MB max)
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_store_retrieve_large() {
        // TODO: Implement with TestHarness
    }

    /// Test 8: Chunk replication across nodes
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_replication() {
        // TODO: Implement - store on one node, verify retrieval from multiple others
    }

    /// Test 9: Payment verification for chunk storage
    #[test]
    #[ignore = "Requires real P2P testnet and Anvil - run with --ignored"]
    fn test_chunk_payment_verification() {
        // TODO: Implement with TestHarness and TestAnvil
        // - Create payment proof via Anvil
        // - Store chunk with payment proof
        // - Verify payment was validated
    }

    /// Test 10: Reject oversized chunk
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_reject_oversized() {
        // TODO: Attempt to store > 4MB chunk, verify rejection
    }

    /// Test 11: Content address verification
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_content_address_verification() {
        // TODO: Store chunk, verify returned address matches computed address
    }

    /// Test 12: Retrieve non-existent chunk returns None
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_retrieve_nonexistent() {
        // TODO: Query random address, verify None returned
    }

    /// Test 13: Duplicate storage returns same address
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_duplicate_storage() {
        // TODO: Store same data twice, verify same address returned
        // (deduplication via content addressing)
    }

    /// Test 14: ML-DSA-65 signature on chunk
    #[test]
    #[ignore = "Requires real P2P testnet - run with --ignored"]
    fn test_chunk_signature_verification() {
        // TODO: Verify chunk is signed with ML-DSA-65 when stored
    }
}
