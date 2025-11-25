//! Configuration for saorsa-node.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

/// IP version configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    /// IPv4 only.
    Ipv4,
    /// IPv6 only.
    Ipv6,
    /// Dual-stack (both IPv4 and IPv6).
    #[default]
    Dual,
}

/// Upgrade channel for auto-updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpgradeChannel {
    /// Stable releases only.
    #[default]
    Stable,
    /// Beta releases (includes stable).
    Beta,
}

/// Node configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Root directory for node data.
    #[serde(default = "default_root_dir")]
    pub root_dir: PathBuf,

    /// Listening port (0 for auto-select).
    #[serde(default)]
    pub port: u16,

    /// IP version to use.
    #[serde(default)]
    pub ip_version: IpVersion,

    /// Bootstrap peer addresses.
    #[serde(default)]
    pub bootstrap: Vec<SocketAddr>,

    /// Upgrade configuration.
    #[serde(default)]
    pub upgrade: UpgradeConfig,

    /// Migration configuration.
    #[serde(default)]
    pub migration: MigrationConfig,

    /// Log level.
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Auto-upgrade configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeConfig {
    /// Enable automatic upgrades.
    #[serde(default)]
    pub enabled: bool,

    /// Release channel.
    #[serde(default)]
    pub channel: UpgradeChannel,

    /// Check interval in hours.
    #[serde(default = "default_check_interval")]
    pub check_interval_hours: u64,
}

/// Migration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Auto-detect ant-node data directories.
    #[serde(default)]
    pub auto_detect: bool,

    /// Explicit path to ant-node data.
    #[serde(default)]
    pub ant_data_path: Option<PathBuf>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            root_dir: default_root_dir(),
            port: 0,
            ip_version: IpVersion::default(),
            bootstrap: Vec::new(),
            upgrade: UpgradeConfig::default(),
            migration: MigrationConfig::default(),
            log_level: default_log_level(),
        }
    }
}

impl Default for UpgradeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            channel: UpgradeChannel::default(),
            check_interval_hours: default_check_interval(),
        }
    }
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_detect: false,
            ant_data_path: None,
        }
    }
}

fn default_root_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "saorsa")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".saorsa"))
}

fn default_log_level() -> String {
    "info".to_string()
}

const fn default_check_interval() -> u64 {
    1 // 1 hour
}

impl NodeConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| crate::Error::Config(e.to_string()))
    }

    /// Save configuration to a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn to_file(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| crate::Error::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
