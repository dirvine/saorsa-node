//! Node implementation - thin wrapper around saorsa-core's NetworkCoordinator.

use crate::config::NodeConfig;
use crate::error::{Error, Result};
use crate::event::{create_event_channel, NodeEvent, NodeEventsChannel, NodeEventsSender};
use crate::migration::AntDataMigrator;
use crate::upgrade::UpgradeMonitor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{info, warn};

/// Builder for constructing a saorsa node.
pub struct NodeBuilder {
    config: NodeConfig,
}

impl NodeBuilder {
    /// Create a new node builder with the given configuration.
    #[must_use]
    pub fn new(config: NodeConfig) -> Self {
        Self { config }
    }

    /// Build and start the node.
    ///
    /// # Errors
    ///
    /// Returns an error if the node fails to start.
    pub async fn build(self) -> Result<RunningNode> {
        info!("Building saorsa-node with config: {:?}", self.config);

        // Ensure root directory exists
        std::fs::create_dir_all(&self.config.root_dir)?;

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create event channel
        let (events_tx, events_rx) = create_event_channel();

        // TODO: Initialize saorsa-core's NetworkCoordinator
        // This will be implemented when we integrate with saorsa-core
        //
        // let network_config = saorsa_core::messaging::NetworkConfig::default();
        // let identity = saorsa_core::identity::NodeIdentity::generate()?;
        // let coordinator = NetworkCoordinator::new(network_config, identity).await?;

        // Create upgrade monitor if enabled
        let upgrade_monitor = if self.config.upgrade.enabled {
            Some(Arc::new(UpgradeMonitor::new(
                "dirvine/saorsa-node".to_string(),
                self.config.upgrade.channel,
                self.config.upgrade.check_interval_hours,
            )))
        } else {
            None
        };

        // Create migrator if configured
        let migrator = if let Some(ref path) = self.config.migration.ant_data_path {
            Some(AntDataMigrator::new(path.clone())?)
        } else if self.config.migration.auto_detect {
            AntDataMigrator::auto_detect()?
        } else {
            None
        };

        let node = RunningNode {
            config: self.config,
            shutdown_tx,
            shutdown_rx,
            events_tx,
            events_rx: Some(events_rx),
            upgrade_monitor,
            migrator,
        };

        Ok(node)
    }
}

/// A running saorsa node.
pub struct RunningNode {
    config: NodeConfig,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    events_tx: NodeEventsSender,
    events_rx: Option<NodeEventsChannel>,
    upgrade_monitor: Option<Arc<UpgradeMonitor>>,
    migrator: Option<AntDataMigrator>,
}

impl RunningNode {
    /// Get the node's root directory.
    #[must_use]
    pub fn root_dir(&self) -> &PathBuf {
        &self.config.root_dir
    }

    /// Get a receiver for node events.
    ///
    /// Note: Can only be called once. Subsequent calls return None.
    pub fn events(&mut self) -> Option<NodeEventsChannel> {
        self.events_rx.take()
    }

    /// Subscribe to node events.
    #[must_use]
    pub fn subscribe_events(&self) -> NodeEventsChannel {
        self.events_tx.subscribe()
    }

    /// Run the node until shutdown is requested.
    ///
    /// # Errors
    ///
    /// Returns an error if the node encounters a fatal error.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting saorsa-node");

        // Emit started event
        let _ = self.events_tx.send(NodeEvent::Started);

        // Run migration if configured
        if let Some(ref migrator) = self.migrator {
            info!("Starting ant-node data migration");
            match migrator.migrate(&self.events_tx).await {
                Ok(stats) => {
                    info!("Migration complete: {} records migrated", stats.migrated);
                    let _ = self.events_tx.send(NodeEvent::MigrationComplete {
                        total: stats.migrated,
                    });
                }
                Err(e) => {
                    warn!("Migration failed: {}", e);
                    let _ = self.events_tx.send(NodeEvent::Error {
                        message: format!("Migration failed: {e}"),
                    });
                }
            }
        }

        // Start upgrade monitor if enabled
        if let Some(ref monitor) = self.upgrade_monitor {
            let monitor = Arc::clone(monitor);
            let events_tx = self.events_tx.clone();
            let mut shutdown_rx = self.shutdown_rx.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                break;
                            }
                        }
                        result = monitor.check_for_updates() => {
                            if let Ok(Some(version)) = result {
                                let _ = events_tx.send(NodeEvent::UpgradeAvailable {
                                    version: version.to_string(),
                                });
                            }
                            // Wait for next check interval
                            tokio::time::sleep(monitor.check_interval()).await;
                        }
                    }
                }
            });
        }

        // TODO: Run the main node loop via saorsa-core's NetworkCoordinator
        // For now, just wait for shutdown signal
        info!("Node running, waiting for shutdown signal");

        loop {
            tokio::select! {
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Shutdown signal received");
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Ctrl-C received, initiating shutdown");
                    self.shutdown();
                    break;
                }
            }
        }

        let _ = self.events_tx.send(NodeEvent::ShuttingDown);
        info!("Node shutdown complete");
        Ok(())
    }

    /// Request the node to shut down.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}
