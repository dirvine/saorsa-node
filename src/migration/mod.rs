//! ant-node data migration module.
//!
//! This module handles discovering and migrating data from existing ant-node
//! installations to the saorsa-network.

mod scanner;

use crate::error::{Error, Result};
use crate::event::{NodeEvent, NodeEventsSender};
use std::path::PathBuf;
use tracing::{debug, info};

/// Statistics from a migration operation.
#[derive(Debug, Default)]
pub struct MigrationStats {
    /// Number of records successfully migrated.
    pub migrated: u64,
    /// Number of records that failed to migrate.
    pub failed: u64,
    /// Number of records skipped (already exist).
    pub skipped: u64,
}

/// Migrates data from ant-node storage to saorsa-network.
pub struct AntDataMigrator {
    /// Path to ant-node data directory.
    ant_data_dir: PathBuf,
}

impl AntDataMigrator {
    /// Create a new migrator for the given ant-node data directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory doesn't exist or isn't readable.
    pub fn new(ant_data_dir: PathBuf) -> Result<Self> {
        if !ant_data_dir.exists() {
            return Err(Error::Migration(format!(
                "ant-node data directory does not exist: {}",
                ant_data_dir.display()
            )));
        }

        if !ant_data_dir.is_dir() {
            return Err(Error::Migration(format!(
                "path is not a directory: {}",
                ant_data_dir.display()
            )));
        }

        info!("Created migrator for ant-node data at: {}", ant_data_dir.display());
        Ok(Self { ant_data_dir })
    }

    /// Auto-detect ant-node data directories.
    ///
    /// Searches common locations for ant-node data.
    ///
    /// # Errors
    ///
    /// Returns an error if no ant-node data is found.
    pub fn auto_detect() -> Result<Option<Self>> {
        let paths = scanner::find_ant_node_data_dirs();

        if paths.is_empty() {
            debug!("No ant-node data directories found");
            return Ok(None);
        }

        // Use the first found directory
        let path = paths.into_iter().next().expect("paths is not empty");
        info!("Auto-detected ant-node data at: {}", path.display());
        Ok(Some(Self::new(path)?))
    }

    /// Migrate all data from ant-node storage.
    ///
    /// # Errors
    ///
    /// Returns an error if migration fails.
    pub async fn migrate(&self, events: &NodeEventsSender) -> Result<MigrationStats> {
        info!("Starting migration from: {}", self.ant_data_dir.display());

        let mut stats = MigrationStats::default();

        // TODO: Implement actual migration logic
        // 1. Enumerate all records in ant-node storage
        // 2. For each record:
        //    a. Read and decrypt using AES-256-GCM-SIV
        //    b. Upload to saorsa-network via NetworkCoordinator
        //    c. Update progress

        // For now, just emit a progress event
        let _ = events.send(NodeEvent::MigrationProgress {
            migrated: 0,
            total: 0,
        });

        info!(
            "Migration complete: {} migrated, {} failed, {} skipped",
            stats.migrated, stats.failed, stats.skipped
        );

        Ok(stats)
    }

    /// Get the ant-node data directory path.
    #[must_use]
    pub fn data_dir(&self) -> &PathBuf {
        &self.ant_data_dir
    }
}
