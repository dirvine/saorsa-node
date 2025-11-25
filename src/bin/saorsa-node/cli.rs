//! Command-line interface definition.

use clap::{Parser, ValueEnum};
use saorsa_node::config::{IpVersion, MigrationConfig, NodeConfig, UpgradeChannel, UpgradeConfig};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Pure quantum-proof network node for the Saorsa decentralized network.
#[derive(Parser, Debug)]
#[command(name = "saorsa-node")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Root directory for node data.
    #[arg(long, env = "SAORSA_ROOT_DIR")]
    pub root_dir: Option<PathBuf>,

    /// Listening port (0 for auto-select).
    #[arg(long, short, default_value = "0", env = "SAORSA_PORT")]
    pub port: u16,

    /// IP version to use.
    #[arg(long, value_enum, default_value = "dual", env = "SAORSA_IP_VERSION")]
    pub ip_version: CliIpVersion,

    /// Bootstrap peer addresses.
    #[arg(long, short, env = "SAORSA_BOOTSTRAP")]
    pub bootstrap: Vec<SocketAddr>,

    /// Path to ant-node data directory to migrate.
    #[arg(long, env = "SAORSA_MIGRATE_ANT_DATA")]
    pub migrate_ant_data: Option<PathBuf>,

    /// Auto-detect ant-node data directories for migration.
    #[arg(long)]
    pub auto_migrate: bool,

    /// Enable automatic upgrades.
    #[arg(long, env = "SAORSA_AUTO_UPGRADE")]
    pub auto_upgrade: bool,

    /// Release channel for upgrades.
    #[arg(long, value_enum, default_value = "stable", env = "SAORSA_UPGRADE_CHANNEL")]
    pub upgrade_channel: CliUpgradeChannel,

    /// Log level.
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    pub log_level: String,

    /// Path to configuration file.
    #[arg(long, short)]
    pub config: Option<PathBuf>,
}

/// IP version CLI enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliIpVersion {
    /// IPv4 only.
    Ipv4,
    /// IPv6 only.
    Ipv6,
    /// Dual-stack (both IPv4 and IPv6).
    Dual,
}

/// Upgrade channel CLI enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliUpgradeChannel {
    /// Stable releases only.
    Stable,
    /// Beta releases.
    Beta,
}

impl Cli {
    /// Convert CLI arguments into a NodeConfig.
    ///
    /// # Errors
    ///
    /// Returns an error if a config file is specified but cannot be loaded.
    pub fn into_config(self) -> color_eyre::Result<NodeConfig> {
        // Start with default config or load from file
        let mut config = if let Some(ref path) = self.config {
            NodeConfig::from_file(path)?
        } else {
            NodeConfig::default()
        };

        // Override with CLI arguments
        if let Some(root_dir) = self.root_dir {
            config.root_dir = root_dir;
        }

        config.port = self.port;
        config.ip_version = self.ip_version.into();
        config.bootstrap = self.bootstrap;
        config.log_level = self.log_level;

        // Upgrade config
        config.upgrade = UpgradeConfig {
            enabled: self.auto_upgrade,
            channel: self.upgrade_channel.into(),
            ..config.upgrade
        };

        // Migration config
        config.migration = MigrationConfig {
            auto_detect: self.auto_migrate,
            ant_data_path: self.migrate_ant_data,
        };

        Ok(config)
    }
}

impl From<CliIpVersion> for IpVersion {
    fn from(v: CliIpVersion) -> Self {
        match v {
            CliIpVersion::Ipv4 => IpVersion::Ipv4,
            CliIpVersion::Ipv6 => IpVersion::Ipv6,
            CliIpVersion::Dual => IpVersion::Dual,
        }
    }
}

impl From<CliUpgradeChannel> for UpgradeChannel {
    fn from(c: CliUpgradeChannel) -> Self {
        match c {
            CliUpgradeChannel::Stable => UpgradeChannel::Stable,
            CliUpgradeChannel::Beta => UpgradeChannel::Beta,
        }
    }
}
