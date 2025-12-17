//! Node event system.

use tokio::sync::broadcast;

/// Events emitted by the node.
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// Node has started successfully.
    Started,

    /// Node is shutting down.
    ShuttingDown,

    /// Connected to a peer.
    PeerConnected {
        /// Peer identifier.
        peer_id: String,
    },

    /// Disconnected from a peer.
    PeerDisconnected {
        /// Peer identifier.
        peer_id: String,
    },

    /// Data stored successfully.
    DataStored {
        /// Data address/key.
        address: String,
    },

    /// Data retrieved successfully.
    DataRetrieved {
        /// Data address/key.
        address: String,
    },

    /// Upgrade available.
    UpgradeAvailable {
        /// New version.
        version: String,
    },

    /// Upgrade started.
    UpgradeStarted {
        /// Version being installed.
        version: String,
    },

    /// Upgrade completed.
    UpgradeComplete {
        /// New version.
        version: String,
    },

    /// Error occurred.
    Error {
        /// Error message.
        message: String,
    },
}

/// Channel for receiving node events.
pub type NodeEventsChannel = broadcast::Receiver<NodeEvent>;

/// Sender for node events.
pub type NodeEventsSender = broadcast::Sender<NodeEvent>;

/// Create a new event channel pair.
#[must_use]
pub fn create_event_channel() -> (NodeEventsSender, NodeEventsChannel) {
    broadcast::channel(256)
}
