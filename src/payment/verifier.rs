//! Main payment verifier combining autonomi lookup and LRU cache.
//!
//! This is the core payment verification logic for saorsa-node.

use crate::error::{Error, Result};
use crate::payment::autonomi_verifier::{AutonomVerifier, AutonomVerifierConfig};
use crate::payment::cache::{VerifiedCache, XorName};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for the payment verifier.
#[derive(Debug, Clone)]
pub struct PaymentVerifierConfig {
    /// Autonomi verifier configuration.
    pub autonomi: AutonomVerifierConfig,
    /// Cache capacity (number of XorNames to cache).
    pub cache_capacity: usize,
    /// Whether to require payment on autonomi lookup failure.
    pub require_payment_on_error: bool,
}

impl Default for PaymentVerifierConfig {
    fn default() -> Self {
        Self {
            autonomi: AutonomVerifierConfig::default(),
            cache_capacity: 100_000,
            require_payment_on_error: true,
        }
    }
}

/// Status returned by payment verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentStatus {
    /// Data exists on autonomi - no payment required.
    AlreadyPaid,
    /// Data was found in local cache - no payment required.
    CachedAsVerified,
    /// New data - payment required.
    PaymentRequired,
    /// Payment was provided and verified.
    PaymentVerified,
}

impl PaymentStatus {
    /// Returns true if the data can be stored (either already paid or payment verified).
    #[must_use]
    pub fn can_store(&self) -> bool {
        matches!(
            self,
            PaymentStatus::AlreadyPaid
                | PaymentStatus::CachedAsVerified
                | PaymentStatus::PaymentVerified
        )
    }

    /// Returns true if this status indicates the data was already paid for.
    #[must_use]
    pub fn is_free(&self) -> bool {
        matches!(
            self,
            PaymentStatus::AlreadyPaid | PaymentStatus::CachedAsVerified
        )
    }
}

/// Main payment verifier for saorsa-node.
///
/// Combines:
/// 1. LRU cache for fast lookups of previously verified XorNames
/// 2. Autonomi network verification for checking if data already exists
/// 3. EVM payment verification for new data (TODO)
pub struct PaymentVerifier {
    /// LRU cache of verified XorNames.
    cache: VerifiedCache,
    /// Autonomi network verifier.
    autonomi: AutonomVerifier,
    /// Configuration.
    config: PaymentVerifierConfig,
}

impl PaymentVerifier {
    /// Create a new payment verifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the autonomi verifier fails to initialize.
    pub async fn new(config: PaymentVerifierConfig) -> Result<Self> {
        let cache = VerifiedCache::with_capacity(config.cache_capacity);
        let autonomi = AutonomVerifier::new(config.autonomi.clone()).await?;

        info!(
            "Payment verifier initialized (cache_capacity={}, autonomi_enabled={})",
            config.cache_capacity,
            autonomi.is_enabled()
        );

        Ok(Self {
            cache,
            autonomi,
            config,
        })
    }

    /// Check if payment is required for the given XorName.
    ///
    /// This is the main entry point for payment verification:
    /// 1. Check LRU cache (fast path)
    /// 2. Query autonomi network
    /// 3. Return status indicating if payment is needed
    ///
    /// # Arguments
    ///
    /// * `xorname` - The content-addressed name of the data
    ///
    /// # Returns
    ///
    /// * `PaymentStatus::CachedAsVerified` - Found in local cache (no payment)
    /// * `PaymentStatus::AlreadyPaid` - Found on autonomi (no payment)
    /// * `PaymentStatus::PaymentRequired` - Not found (payment required)
    pub async fn check_payment_required(&self, xorname: &XorName) -> PaymentStatus {
        // Step 1: Check LRU cache (fast path)
        if self.cache.contains(xorname) {
            debug!(
                "Data {} found in verified cache",
                hex::encode(xorname)
            );
            return PaymentStatus::CachedAsVerified;
        }

        // Step 2: Query autonomi network
        match self.autonomi.data_exists(xorname).await {
            Ok(true) => {
                // Data exists on autonomi - cache it and return AlreadyPaid
                self.cache.insert(*xorname);
                info!(
                    "Data {} exists on autonomi - storing free",
                    hex::encode(xorname)
                );
                PaymentStatus::AlreadyPaid
            }
            Ok(false) => {
                // Data not found - payment required
                debug!(
                    "Data {} not found on autonomi - payment required",
                    hex::encode(xorname)
                );
                PaymentStatus::PaymentRequired
            }
            Err(e) => {
                // Network error - decide based on config
                warn!(
                    "Autonomi lookup failed for {}: {}",
                    hex::encode(xorname),
                    e
                );
                if self.config.require_payment_on_error {
                    PaymentStatus::PaymentRequired
                } else {
                    // Fail open - allow free storage on error
                    // This is less secure but more user-friendly during network issues
                    PaymentStatus::AlreadyPaid
                }
            }
        }
    }

    /// Verify that a PUT request has valid payment or data exists on autonomi.
    ///
    /// This is the complete payment verification flow:
    /// 1. Check if data exists (cache or autonomi)
    /// 2. If not, verify the provided payment proof
    ///
    /// # Arguments
    ///
    /// * `xorname` - The content-addressed name of the data
    /// * `payment_proof` - Optional payment proof (required if data doesn't exist)
    ///
    /// # Returns
    ///
    /// * `Ok(PaymentStatus)` - Verification succeeded
    /// * `Err(Error::PaymentRequired)` - No payment and data not found
    /// * `Err(Error::PaymentInvalid)` - Payment provided but invalid
    ///
    /// # Errors
    ///
    /// Returns an error if payment is required but not provided, or if payment is invalid.
    pub async fn verify_payment(
        &self,
        xorname: &XorName,
        payment_proof: Option<&[u8]>,
    ) -> Result<PaymentStatus> {
        // First check if payment is required
        let status = self.check_payment_required(xorname).await;

        match status {
            PaymentStatus::CachedAsVerified | PaymentStatus::AlreadyPaid => {
                // No payment needed
                Ok(status)
            }
            PaymentStatus::PaymentRequired => {
                // Payment is required - verify the proof
                match payment_proof {
                    Some(proof) => {
                        // TODO: Implement EVM payment verification
                        // This will involve:
                        // 1. Deserialize the ProofOfPayment
                        // 2. Verify signatures
                        // 3. Check EVM transaction on Arbitrum
                        // 4. Verify payment amount and recipient

                        if proof.is_empty() {
                            return Err(Error::Payment("Empty payment proof".to_string()));
                        }

                        // Placeholder: Accept any non-empty proof for now
                        // TODO: Implement actual EVM verification
                        warn!(
                            "EVM verification not yet implemented - accepting payment for {}",
                            hex::encode(xorname)
                        );
                        Ok(PaymentStatus::PaymentVerified)
                    }
                    None => {
                        // No payment provided
                        Err(Error::Payment(format!(
                            "Payment required for new data {}",
                            hex::encode(xorname)
                        )))
                    }
                }
            }
            PaymentStatus::PaymentVerified => {
                // This shouldn't happen from check_payment_required
                Ok(status)
            }
        }
    }

    /// Get cache statistics.
    #[must_use]
    pub fn cache_stats(&self) -> crate::payment::cache::CacheStats {
        self.cache.stats()
    }

    /// Get the number of cached entries.
    #[must_use]
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the autonomi verifier is enabled.
    #[must_use]
    pub fn autonomi_enabled(&self) -> bool {
        self.autonomi.is_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_verifier() -> PaymentVerifier {
        let config = PaymentVerifierConfig {
            autonomi: AutonomVerifierConfig {
                enabled: false, // Disabled for tests
                ..Default::default()
            },
            cache_capacity: 100,
            require_payment_on_error: true,
        };
        PaymentVerifier::new(config).await.expect("should create")
    }

    #[tokio::test]
    async fn test_payment_required_for_new_data() {
        let verifier = create_test_verifier().await;
        let xorname = [1u8; 32];

        // With autonomi disabled, all data should require payment
        let status = verifier.check_payment_required(&xorname).await;
        assert_eq!(status, PaymentStatus::PaymentRequired);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let verifier = create_test_verifier().await;
        let xorname = [1u8; 32];

        // Manually add to cache
        verifier.cache.insert(xorname);

        // Should return CachedAsVerified
        let status = verifier.check_payment_required(&xorname).await;
        assert_eq!(status, PaymentStatus::CachedAsVerified);
    }

    #[tokio::test]
    async fn test_verify_payment_without_proof() {
        let verifier = create_test_verifier().await;
        let xorname = [1u8; 32];

        // Should fail without payment proof
        let result = verifier.verify_payment(&xorname, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_payment_with_proof() {
        let verifier = create_test_verifier().await;
        let xorname = [1u8; 32];

        // Should succeed with any non-empty proof (for now)
        let proof = vec![1u8, 2, 3, 4];
        let result = verifier.verify_payment(&xorname, Some(&proof)).await;
        assert!(result.is_ok());
        assert_eq!(result.expect("verified"), PaymentStatus::PaymentVerified);
    }

    #[tokio::test]
    async fn test_verify_payment_cached() {
        let verifier = create_test_verifier().await;
        let xorname = [1u8; 32];

        // Add to cache
        verifier.cache.insert(xorname);

        // Should succeed without payment (cached)
        let result = verifier.verify_payment(&xorname, None).await;
        assert!(result.is_ok());
        assert_eq!(result.expect("cached"), PaymentStatus::CachedAsVerified);
    }

    #[test]
    fn test_payment_status_can_store() {
        assert!(PaymentStatus::AlreadyPaid.can_store());
        assert!(PaymentStatus::CachedAsVerified.can_store());
        assert!(PaymentStatus::PaymentVerified.can_store());
        assert!(!PaymentStatus::PaymentRequired.can_store());
    }

    #[test]
    fn test_payment_status_is_free() {
        assert!(PaymentStatus::AlreadyPaid.is_free());
        assert!(PaymentStatus::CachedAsVerified.is_free());
        assert!(!PaymentStatus::PaymentVerified.is_free());
        assert!(!PaymentStatus::PaymentRequired.is_free());
    }
}
