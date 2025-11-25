//! Payment verification system for saorsa-node.
//!
//! This module implements the payment verification strategy:
//! 1. Check if data already exists on the autonomi network (already paid)
//! 2. If not, require and verify EVM/Arbitrum payment for new data
//!
//! # Architecture
//!
//! ```text
//! PUT request received
//!        │
//!        ▼
//! ┌─────────────────────┐
//! │ Check LRU cache     │
//! └─────────┬───────────┘
//!           │
//!    ┌──────┴──────┐
//!    │             │
//!   HIT          MISS
//!    │             │
//!    ▼             ▼
//! Store FREE   Query autonomi
//!                  │
//!           ┌──────┴──────┐
//!           │             │
//!        EXISTS      NOT FOUND
//!           │             │
//!           ▼             ▼
//!      Cache + FREE   Require EVM payment
//! ```

mod autonomi_verifier;
mod cache;
mod verifier;

pub use autonomi_verifier::AutonomVerifier;
pub use cache::VerifiedCache;
pub use verifier::{PaymentStatus, PaymentVerifier, PaymentVerifierConfig};
