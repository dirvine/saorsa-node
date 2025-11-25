//! ML-DSA signature verification for binary upgrades.

use crate::error::{Error, Result};
use std::path::Path;
use tracing::{debug, info};

/// Embedded release signing public key (ML-DSA-65).
///
/// This key is used to verify signatures on released binaries.
/// The corresponding private key is held by authorized release signers.
///
/// TODO: Replace with actual ML-DSA-65 public key before release.
const RELEASE_SIGNING_KEY: &[u8] = &[];

/// Verify the ML-DSA signature on a binary file.
///
/// # Arguments
///
/// * `binary_path` - Path to the binary to verify
/// * `signature` - The ML-DSA-65 signature bytes
///
/// # Errors
///
/// Returns an error if verification fails.
pub fn verify_binary_signature(binary_path: &Path, signature: &[u8]) -> Result<()> {
    debug!("Verifying signature for: {}", binary_path.display());

    if RELEASE_SIGNING_KEY.is_empty() {
        return Err(Error::Crypto(
            "Release signing key not configured".to_string(),
        ));
    }

    // TODO: Implement ML-DSA-65 signature verification
    // 1. Read binary file
    // 2. Create ML-DSA verifier with RELEASE_SIGNING_KEY
    // 3. Verify signature against binary content

    // For now, fail if any signature is provided (no key configured)
    Err(Error::Crypto(
        "Signature verification not implemented".to_string(),
    ))
}

/// Verify a signature from a detached .sig file.
///
/// # Errors
///
/// Returns an error if the signature file cannot be read or verification fails.
pub fn verify_from_file(binary_path: &Path, signature_path: &Path) -> Result<()> {
    debug!(
        "Verifying {} with signature from {}",
        binary_path.display(),
        signature_path.display()
    );

    let signature = std::fs::read(signature_path)?;
    verify_binary_signature(binary_path, &signature)
}
