//! Auto-upgrade system with ML-DSA signature verification.
//!
//! This module handles:
//! - Polling GitHub releases for new versions
//! - Verifying ML-DSA-65 signatures on binaries
//! - Replacing the running binary with rollback support

mod monitor;
mod signature;

pub use monitor::UpgradeMonitor;

use crate::config::UpgradeChannel;
use crate::error::Result;
use semver::Version;
use std::path::PathBuf;
use std::time::Duration;

/// Information about an available upgrade.
#[derive(Debug, Clone)]
pub struct UpgradeInfo {
    /// The new version.
    pub version: Version,
    /// Download URL for the binary.
    pub download_url: String,
    /// Signature URL.
    pub signature_url: String,
    /// Release notes.
    pub release_notes: String,
}

/// Result of an upgrade operation.
#[derive(Debug)]
pub enum UpgradeResult {
    /// Upgrade was successful.
    Success {
        /// The new version.
        version: Version,
    },
    /// Upgrade failed, rolled back.
    RolledBack {
        /// Error that caused the rollback.
        reason: String,
    },
    /// No upgrade available.
    NoUpgrade,
}

/// Perform an upgrade to the specified version.
///
/// # Errors
///
/// Returns an error if the upgrade fails and rollback is not possible.
pub async fn perform_upgrade(
    info: &UpgradeInfo,
    current_binary: &PathBuf,
    rollback_dir: &PathBuf,
) -> Result<UpgradeResult> {
    // TODO: Implement upgrade logic
    // 1. Download new binary to temp location
    // 2. Download and verify ML-DSA signature
    // 3. Backup current binary to rollback_dir
    // 4. Replace current binary with new one
    // 5. On failure, restore from backup

    tracing::info!(
        "Would upgrade to version {} from {}",
        info.version,
        info.download_url
    );

    Ok(UpgradeResult::NoUpgrade)
}
