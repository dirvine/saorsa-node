//! ML-DSA signature verification for binary upgrades.
//!
//! Provides quantum-resistant signature verification for auto-upgrade binaries
//! using FIPS 204 ML-DSA-65.

use crate::error::{Error, Result};
use saorsa_pqc::api::sig::{ml_dsa_65, MlDsaPublicKey, MlDsaSignature, MlDsaVariant};
use std::fs;
use std::path::Path;
use tracing::debug;

/// Signing context for domain separation (prevents cross-protocol attacks).
pub const SIGNING_CONTEXT: &[u8] = b"saorsa-node-release-v1";

/// ML-DSA-65 signature size in bytes.
pub const SIGNATURE_SIZE: usize = 3309;

/// ML-DSA-65 public key size in bytes.
pub const PUBLIC_KEY_SIZE: usize = 1952;

/// Embedded release signing public key (ML-DSA-65).
///
/// This key is used to verify signatures on released binaries.
/// The corresponding private key is held by authorized release signers.
///
/// TODO: Replace with actual ML-DSA-65 public key before production release.
const RELEASE_SIGNING_KEY: &[u8] = &[];

/// Verify the ML-DSA signature on a binary file using the embedded release key.
///
/// # Arguments
///
/// * `binary_path` - Path to the binary to verify
/// * `signature` - The ML-DSA-65 signature bytes
///
/// # Errors
///
/// Returns an error if:
/// - The release signing key is not configured
/// - The binary file cannot be read
/// - The signature format is invalid
/// - The signature verification fails
pub fn verify_binary_signature(binary_path: &Path, signature: &[u8]) -> Result<()> {
    // Allow: This is intentionally empty until production key is embedded.
    #[allow(clippy::const_is_empty)]
    if RELEASE_SIGNING_KEY.is_empty() {
        return Err(Error::Crypto(
            "Release signing key not configured".to_string(),
        ));
    }

    let public_key = MlDsaPublicKey::from_bytes(MlDsaVariant::MlDsa65, RELEASE_SIGNING_KEY)
        .map_err(|e| Error::Crypto(format!("Invalid release key: {e}")))?;

    verify_binary_signature_with_key(binary_path, signature, &public_key)
}

/// Verify signature with an explicit public key.
///
/// This function is useful for testing and for cases where the public key
/// is provided externally rather than embedded in the binary.
///
/// # Arguments
///
/// * `binary_path` - Path to the binary to verify
/// * `signature` - The ML-DSA-65 signature bytes
/// * `public_key` - The public key to verify against
///
/// # Errors
///
/// Returns an error if:
/// - The binary file cannot be read
/// - The signature has an invalid size
/// - The signature format is invalid
/// - The signature verification fails
pub fn verify_binary_signature_with_key(
    binary_path: &Path,
    signature: &[u8],
    public_key: &MlDsaPublicKey,
) -> Result<()> {
    debug!("Verifying signature for: {}", binary_path.display());

    // Read binary content
    let binary_content = fs::read(binary_path).map_err(|e| {
        Error::Crypto(format!(
            "Failed to read binary '{}': {e}",
            binary_path.display()
        ))
    })?;

    // Validate signature size
    if signature.len() != SIGNATURE_SIZE {
        return Err(Error::Crypto(format!(
            "Invalid signature size: expected {SIGNATURE_SIZE}, got {}",
            signature.len()
        )));
    }

    // Parse signature
    let sig = MlDsaSignature::from_bytes(MlDsaVariant::MlDsa65, signature)
        .map_err(|e| Error::Crypto(format!("Invalid signature format: {e}")))?;

    // Verify with context
    let dsa = ml_dsa_65();
    let valid = dsa
        .verify_with_context(public_key, &binary_content, &sig, SIGNING_CONTEXT)
        .map_err(|e| Error::Crypto(format!("Signature verification error: {e}")))?;

    if valid {
        debug!("Signature verified successfully");
        Ok(())
    } else {
        Err(Error::Crypto(
            "Signature verification failed: invalid signature".to_string(),
        ))
    }
}

/// Verify a signature from a detached .sig file.
///
/// # Arguments
///
/// * `binary_path` - Path to the binary to verify
/// * `signature_path` - Path to the detached signature file
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

    let signature = fs::read(signature_path)?;
    verify_binary_signature(binary_path, &signature)
}

/// Verify from file with explicit key.
///
/// # Arguments
///
/// * `binary_path` - Path to the binary to verify
/// * `signature_path` - Path to the detached signature file
/// * `public_key` - The public key to verify against
///
/// # Errors
///
/// Returns an error if the signature file cannot be read or verification fails.
pub fn verify_from_file_with_key(
    binary_path: &Path,
    signature_path: &Path,
    public_key: &MlDsaPublicKey,
) -> Result<()> {
    debug!(
        "Verifying {} with signature from {}",
        binary_path.display(),
        signature_path.display()
    );

    let signature = fs::read(signature_path)?;
    verify_binary_signature_with_key(binary_path, &signature, public_key)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;
    use saorsa_pqc::api::sig::ml_dsa_65;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Test 1: Valid signature verification
    #[test]
    fn test_verify_valid_signature() {
        let dsa = ml_dsa_65();
        let (public_key, secret_key) = dsa.generate_keypair().unwrap();
        let binary_content = b"test binary content for signing";

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(binary_content).unwrap();

        let signature = dsa
            .sign_with_context(&secret_key, binary_content, SIGNING_CONTEXT)
            .unwrap();

        let result =
            verify_binary_signature_with_key(file.path(), &signature.to_bytes(), &public_key);
        assert!(result.is_ok(), "Valid signature should verify: {result:?}");
    }

    /// Test 2: Invalid signature rejected
    #[test]
    fn test_reject_invalid_signature() {
        let dsa = ml_dsa_65();
        let (public_key, _) = dsa.generate_keypair().unwrap();
        let binary_content = b"test binary content";

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(binary_content).unwrap();

        // All-zero signature is invalid
        let invalid_sig = vec![0u8; SIGNATURE_SIZE];

        let result = verify_binary_signature_with_key(file.path(), &invalid_sig, &public_key);
        assert!(result.is_err(), "Invalid signature should be rejected");
    }

    /// Test 3: Wrong key rejected
    #[test]
    fn test_reject_wrong_key() {
        let dsa = ml_dsa_65();
        let (_, secret_key) = dsa.generate_keypair().unwrap();
        let (wrong_key, _) = dsa.generate_keypair().unwrap();
        let binary_content = b"test binary content";

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(binary_content).unwrap();

        let signature = dsa
            .sign_with_context(&secret_key, binary_content, SIGNING_CONTEXT)
            .unwrap();

        let result =
            verify_binary_signature_with_key(file.path(), &signature.to_bytes(), &wrong_key);
        assert!(result.is_err(), "Wrong key should fail verification");
    }

    /// Test 4: Modified binary rejected
    #[test]
    fn test_reject_modified_binary() {
        let dsa = ml_dsa_65();
        let (public_key, secret_key) = dsa.generate_keypair().unwrap();
        let original_content = b"original binary content";

        // Sign the original content
        let signature = dsa
            .sign_with_context(&secret_key, original_content, SIGNING_CONTEXT)
            .unwrap();

        // Write modified content to file
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"MODIFIED binary content").unwrap();

        let result =
            verify_binary_signature_with_key(file.path(), &signature.to_bytes(), &public_key);
        assert!(result.is_err(), "Modified binary should fail verification");
    }

    /// Test 5: Malformed signature rejected
    #[test]
    fn test_reject_malformed_signature() {
        let dsa = ml_dsa_65();
        let (public_key, _) = dsa.generate_keypair().unwrap();
        let binary_content = b"test content";

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(binary_content).unwrap();

        // Too short signature
        let short_sig = vec![0u8; 100];

        let result = verify_binary_signature_with_key(file.path(), &short_sig, &public_key);
        assert!(result.is_err(), "Malformed signature should be rejected");
        assert!(
            result.unwrap_err().to_string().contains("Invalid signature size"),
            "Error should mention invalid size"
        );
    }

    /// Test 6: Empty file handling
    #[test]
    fn test_empty_file() {
        let dsa = ml_dsa_65();
        let (public_key, secret_key) = dsa.generate_keypair().unwrap();
        let empty_content: &[u8] = b"";

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(empty_content).unwrap();

        let signature = dsa
            .sign_with_context(&secret_key, empty_content, SIGNING_CONTEXT)
            .unwrap();

        let result =
            verify_binary_signature_with_key(file.path(), &signature.to_bytes(), &public_key);
        assert!(result.is_ok(), "Empty file should verify: {result:?}");
    }

    /// Test 7: Non-existent file
    #[test]
    fn test_nonexistent_file() {
        let dsa = ml_dsa_65();
        let (public_key, _) = dsa.generate_keypair().unwrap();
        let path = Path::new("/nonexistent/path/to/binary");
        let sig = vec![0u8; SIGNATURE_SIZE];

        let result = verify_binary_signature_with_key(path, &sig, &public_key);
        assert!(result.is_err(), "Non-existent file should fail");
    }

    /// Test 8: Context binding (cross-protocol attack prevention)
    #[test]
    fn test_wrong_context_rejected() {
        let dsa = ml_dsa_65();
        let (public_key, secret_key) = dsa.generate_keypair().unwrap();
        let content = b"binary content";

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content).unwrap();

        // Sign with wrong context
        let signature = dsa
            .sign_with_context(&secret_key, content, b"wrong-context-string")
            .unwrap();

        // Verify with correct context should fail
        let result =
            verify_binary_signature_with_key(file.path(), &signature.to_bytes(), &public_key);
        assert!(
            result.is_err(),
            "Wrong context should fail verification: {result:?}"
        );
    }

    /// Test 9: Verify from detached .sig file
    #[test]
    fn test_verify_from_sig_file() {
        let dsa = ml_dsa_65();
        let (public_key, secret_key) = dsa.generate_keypair().unwrap();
        let content = b"binary content for sig file test";

        let mut binary_file = NamedTempFile::new().unwrap();
        binary_file.write_all(content).unwrap();

        let signature = dsa
            .sign_with_context(&secret_key, content, SIGNING_CONTEXT)
            .unwrap();

        let mut sig_file = NamedTempFile::new().unwrap();
        sig_file.write_all(&signature.to_bytes()).unwrap();

        let result =
            verify_from_file_with_key(binary_file.path(), sig_file.path(), &public_key);
        assert!(
            result.is_ok(),
            "Detached sig file verification should succeed: {result:?}"
        );
    }

    /// Test 10: Large binary file
    #[test]
    fn test_large_binary() {
        let dsa = ml_dsa_65();
        let (public_key, secret_key) = dsa.generate_keypair().unwrap();

        // Create 1MB of test content
        let large_content: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&large_content).unwrap();

        let signature = dsa
            .sign_with_context(&secret_key, &large_content, SIGNING_CONTEXT)
            .unwrap();

        let result =
            verify_binary_signature_with_key(file.path(), &signature.to_bytes(), &public_key);
        assert!(
            result.is_ok(),
            "Large binary should verify: {result:?}"
        );
    }

    /// Test: Embedded release key not configured
    #[test]
    fn test_release_key_not_configured() {
        let path = Path::new("/tmp/test");
        let sig = vec![0u8; SIGNATURE_SIZE];

        let result = verify_binary_signature(path, &sig);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Release signing key not configured"));
    }
}
