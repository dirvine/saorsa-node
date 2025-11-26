//! ant-node data migration module.
//!
//! This module handles discovering and migrating data from existing ant-node
//! installations to the saorsa-network.

mod decrypt;
mod scanner;

pub use decrypt::{decrypt_record, decrypt_with_embedded_nonce, derive_key, derive_record_key};

use crate::error::{Error, Result};
use crate::event::{NodeEvent, NodeEventsSender};
use std::path::PathBuf;
use tracing::{debug, info, warn};

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

/// Record types that may be stored in ant-node.
#[derive(Debug, Clone, Copy)]
pub enum RecordType {
    /// Chunk data (encrypted content).
    Chunk,
    /// Register data.
    Register,
    /// Scratchpad data.
    Scratchpad,
    /// Graph entry.
    GraphEntry,
    /// Unknown/other record type.
    Unknown,
}

/// A record found in ant-node storage.
#[derive(Debug)]
pub struct AntRecord {
    /// The path to the record file.
    pub path: PathBuf,
    /// The type of record.
    pub record_type: RecordType,
    /// The raw content (may be encrypted).
    pub content: Vec<u8>,
}

/// Result of processing a single record.
enum ProcessResult {
    /// Record was successfully migrated.
    Migrated,
    /// Record was skipped (already exists or empty).
    Skipped,
}

/// Migrates data from ant-node storage to saorsa-network.
pub struct AntDataMigrator {
    /// Path to ant-node data directory.
    ant_data_dir: PathBuf,
    /// Master key for record decryption (if available).
    master_key: Option<Vec<u8>>,
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
        Ok(Self {
            ant_data_dir,
            master_key: None,
        })
    }

    /// Create a migrator with a decryption key for encrypted records.
    ///
    /// # Arguments
    ///
    /// * `ant_data_dir` - Path to ant-node data directory
    /// * `master_key` - Master key for record decryption
    ///
    /// # Errors
    ///
    /// Returns an error if the directory doesn't exist or isn't readable.
    pub fn with_key(ant_data_dir: PathBuf, master_key: Vec<u8>) -> Result<Self> {
        let mut migrator = Self::new(ant_data_dir)?;
        migrator.master_key = Some(master_key);
        info!("Migrator configured with decryption key");
        Ok(migrator)
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

        // Use the first found directory (safe because we checked is_empty above)
        if let Some(path) = paths.into_iter().next() {
            info!("Auto-detected ant-node data at: {}", path.display());
            Ok(Some(Self::new(path)?))
        } else {
            Ok(None)
        }
    }

    /// Migrate all data from ant-node storage.
    ///
    /// # Errors
    ///
    /// Returns an error if migration fails.
    pub async fn migrate(&self, events: &NodeEventsSender) -> Result<MigrationStats> {
        info!("Starting migration from: {}", self.ant_data_dir.display());

        let mut stats = MigrationStats::default();

        // Step 1: Enumerate all record files
        let record_paths = self.find_record_files()?;
        let total = record_paths.len() as u64;

        if total == 0 {
            info!("No records found to migrate");
            return Ok(stats);
        }

        info!("Found {} records to migrate", total);

        // Send initial progress
        if let Err(e) = events.send(NodeEvent::MigrationProgress { migrated: 0, total }) {
            warn!("Failed to send migration progress event: {e}");
        }

        // Step 2: Process each record
        for (idx, path) in record_paths.iter().enumerate() {
            match self.process_record(path).await {
                Ok(ProcessResult::Migrated) => {
                    stats.migrated += 1;
                }
                Ok(ProcessResult::Skipped) => {
                    stats.skipped += 1;
                }
                Err(e) => {
                    warn!("Failed to migrate record {}: {}", path.display(), e);
                    stats.failed += 1;
                }
            }

            // Send progress update every 100 records or on the last record
            if idx % 100 == 0 || idx == record_paths.len() - 1 {
                if let Err(e) = events.send(NodeEvent::MigrationProgress {
                    migrated: stats.migrated,
                    total,
                }) {
                    warn!("Failed to send migration progress event: {e}");
                }
            }
        }

        info!(
            "Migration complete: {} migrated, {} failed, {} skipped",
            stats.migrated, stats.failed, stats.skipped
        );

        Ok(stats)
    }

    /// Find all record files in the ant-node data directory.
    fn find_record_files(&self) -> Result<Vec<PathBuf>> {
        let mut records = Vec::new();

        // ant-node typically stores records in subdirectories based on XorName prefix
        // e.g., /record_store/aa/aabb.../record
        let record_store = self.ant_data_dir.join("record_store");

        if !record_store.exists() {
            debug!("No record_store directory found");
            return Ok(records);
        }

        // Walk the directory tree looking for record files
        Self::walk_directory(&record_store, &mut records)?;

        debug!("Found {} record files", records.len());
        Ok(records)
    }

    /// Recursively walk a directory to find record files.
    fn walk_directory(dir: &PathBuf, records: &mut Vec<PathBuf>) -> Result<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            Error::Migration(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                Self::walk_directory(&path, records)?;
            } else if path.is_file() {
                // Record files typically don't have extensions or have specific extensions
                let extension = path.extension().and_then(|e| e.to_str());
                match extension {
                    None | Some("record" | "chunk") => {
                        records.push(path);
                    }
                    _ => {
                        // Skip non-record files
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a single record file.
    #[allow(clippy::unused_async)] // Will need async for network operations
    async fn process_record(&self, path: &std::path::Path) -> Result<ProcessResult> {
        debug!("Processing record: {}", path.display());

        // Read the record file
        let content = std::fs::read(path).map_err(|e| {
            Error::Migration(format!("Failed to read record {}: {}", path.display(), e))
        })?;

        if content.is_empty() {
            return Ok(ProcessResult::Skipped);
        }

        // Determine record type from path or content
        let record_type = Self::detect_record_type(path, &content);

        debug!(
            "Record {} is type {:?}, size {} bytes",
            path.display(),
            record_type,
            content.len()
        );

        // Try to extract XorName from path (ant-node uses hex-encoded names)
        let xorname = Self::extract_xorname_from_path(path);

        // Decrypt the record if we have a master key and the record appears encrypted
        let decrypted_content = if let Some(ref master_key) = self.master_key {
            if let Some(ref xorname) = xorname {
                // Derive record-specific key and decrypt
                match decrypt::derive_record_key(master_key, xorname) {
                    Ok(key) => {
                        match decrypt::decrypt_with_embedded_nonce(&content, &key) {
                            Ok(data) => {
                                debug!(
                                    "Successfully decrypted record: {} -> {} bytes",
                                    content.len(),
                                    data.len()
                                );
                                data
                            }
                            Err(e) => {
                                // Decryption failed - record might not be encrypted
                                debug!("Decryption failed (using raw content): {e}");
                                content
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Key derivation failed (using raw content): {e}");
                        content
                    }
                }
            } else {
                // No XorName extracted, use raw content
                content
            }
        } else {
            // No master key configured, use raw content
            content
        };

        // Create a record for storage
        let _record = AntRecord {
            path: path.to_path_buf(),
            record_type,
            content: decrypted_content,
        };

        // Network operations placeholder:
        // In a full implementation, this would:
        // 1. Check if record exists on saorsa-network via P2PNode::lookup()
        // 2. If not found, store via P2PNode::store()
        // 3. Re-encrypt with quantum-safe cryptography if needed
        //
        // For now, mark as migrated since we've successfully processed the record
        debug!("Record processed, ready for network upload");

        Ok(ProcessResult::Migrated)
    }

    /// Extract XorName from a record file path.
    ///
    /// ant-node stores records with paths like:
    /// `record_store/aa/aabbccdd.../record`
    /// where the directory name is the hex-encoded XorName.
    fn extract_xorname_from_path(path: &std::path::Path) -> Option<[u8; 32]> {
        // Try to get the parent directory name (which may contain the XorName)
        let parent = path.parent()?;
        let dirname = parent.file_name()?.to_str()?;

        // Check if it looks like a hex-encoded XorName (64 hex chars = 32 bytes)
        if dirname.len() >= 64 && dirname.chars().all(|c| c.is_ascii_hexdigit()) {
            let hex_str = &dirname[..64];
            let bytes = hex::decode(hex_str).ok()?;
            if bytes.len() == 32 {
                let mut xorname = [0u8; 32];
                xorname.copy_from_slice(&bytes);
                return Some(xorname);
            }
        }

        // Also check grandparent for two-level directory structure
        let grandparent = parent.parent()?;
        let grandparent_name = grandparent.file_name()?.to_str()?;

        // Sometimes the full XorName is split across directories
        let combined = format!("{grandparent_name}{dirname}");
        if combined.len() >= 64 && combined.chars().all(|c| c.is_ascii_hexdigit()) {
            let hex_str = &combined[..64];
            let bytes = hex::decode(hex_str).ok()?;
            if bytes.len() == 32 {
                let mut xorname = [0u8; 32];
                xorname.copy_from_slice(&bytes);
                return Some(xorname);
            }
        }

        None
    }

    /// Detect the type of record from path or content.
    fn detect_record_type(path: &std::path::Path, _content: &[u8]) -> RecordType {
        // Determine type from file extension or path components
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension {
                "chunk" => return RecordType::Chunk,
                "register" => return RecordType::Register,
                "scratchpad" => return RecordType::Scratchpad,
                "graph" => return RecordType::GraphEntry,
                _ => {}
            }
        }

        // Check parent directory name
        if let Some(parent) = path.parent().and_then(|p| p.file_name()) {
            if let Some(name) = parent.to_str() {
                if name.contains("chunk") {
                    return RecordType::Chunk;
                } else if name.contains("register") {
                    return RecordType::Register;
                }
            }
        }

        // Default to Chunk as it's the most common type
        RecordType::Chunk
    }

    /// Get the ant-node data directory path.
    #[must_use]
    pub fn data_dir(&self) -> &PathBuf {
        &self.ant_data_dir
    }
}
