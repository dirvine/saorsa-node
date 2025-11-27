//! Hybrid client module for saorsa-node.
//!
//! This module provides a unified client interface that bridges the saorsa network
//! (quantum-resistant) and the legacy autonomi network. It enables seamless
//! retrieval of existing data while ensuring all new data uses post-quantum
//! cryptography.
//!
//! # Architecture
//!
//! The hybrid client provides:
//!
//! 1. **Quantum-first retrieval**: Try saorsa network first for best security
//! 2. **Legacy fallback**: Fall back to autonomi for data not yet migrated
//! 3. **Auto-migration**: Automatically migrate data to saorsa on read
//! 4. **PQC-only writes**: All new data uses ML-KEM-768 and ML-DSA-65
//!
//! # Data Types
//!
//! The module supports all autonomi data types:
//!
//! - **Chunk**: Immutable content-addressed data
//! - **Scratchpad**: Mutable single-owner data with counter
//! - **Pointer**: References to other data addresses
//! - **`GraphEntry`**: Linked data structures with parents/descendants
//!
//! # Example
//!
//! ```rust,ignore
//! use saorsa_node::client::{HybridClient, HybridConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client with default config
//!     let client = HybridClient::with_defaults();
//!
//!     // Store new data (uses saorsa with PQC)
//!     let address = client.put_chunk(bytes::Bytes::from("hello world")).await?;
//!
//!     // Retrieve data (tries saorsa first, falls back to autonomi)
//!     let data = client.get_chunk(&address).await?;
//!
//!     // Check statistics
//!     let stats = client.stats();
//!     println!("Saorsa hits: {}", stats.saorsa_hits);
//!     println!("Autonomi hits: {}", stats.autonomi_hits);
//!     println!("Migrations: {}", stats.migrations);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Security Model
//!
//! ## Quantum-Resistant Cryptography
//!
//! All data stored through this client uses:
//! - **ML-KEM-768** (NIST FIPS 203): Key encapsulation for encryption
//! - **ML-DSA-65** (NIST FIPS 204): Digital signatures for authentication
//! - **ChaCha20-Poly1305**: Symmetric encryption for data at rest
//!
//! ## Legacy Data Trust
//!
//! Data retrieved from autonomi is trusted because:
//! - The autonomi network already verified BLS signatures
//! - We are performing read-only operations
//! - Data is re-encrypted with PQC before being stored on saorsa
//!
//! # Migration Strategy
//!
//! When auto-migration is enabled (default), data retrieved from autonomi is
//! automatically stored on saorsa with quantum-resistant encryption. This
//! provides:
//!
//! 1. **Gradual migration**: Data migrates as it's accessed
//! 2. **No downtime**: Users can access data throughout migration
//! 3. **Security upgrade**: Legacy data gets PQC protection
//! 4. **Redundancy**: Data exists on both networks during transition

mod data_types;
mod hybrid;
mod legacy;
mod quantum;

pub use data_types::{
    DataChunk, DataSource, GraphEntry, HybridStats, LookupResult, PointerRecord, RecordKind,
    ScratchpadEntry, XorName,
};
pub use hybrid::{HybridClient, HybridConfig};
pub use legacy::{LegacyClient, LegacyConfig};
pub use quantum::{QuantumClient, QuantumConfig};
