//! Scanner for finding ant-node data directories.

use std::path::PathBuf;
use tracing::debug;

/// Common locations where ant-node stores data.
const ANT_NODE_DATA_PATHS: &[&str] = &[
    // Linux
    ".local/share/safe/node",
    ".safe/node",
    // macOS
    "Library/Application Support/safe/node",
    // Windows (via home dir)
    "AppData/Roaming/safe/node",
    "AppData/Local/safe/node",
];

/// Find ant-node data directories in common locations.
#[must_use]
pub fn find_ant_node_data_dirs() -> Vec<PathBuf> {
    let mut found = Vec::new();

    // Get home directory
    let Some(home) = dirs_home() else {
        debug!("Could not determine home directory");
        return found;
    };

    // Check each common location
    for relative_path in ANT_NODE_DATA_PATHS {
        let path = home.join(relative_path);
        if path.exists() && path.is_dir() {
            debug!("Found ant-node data at: {}", path.display());
            found.push(path);
        }
    }

    // Also check for environment variable override
    if let Ok(path) = std::env::var("ANT_NODE_DATA_DIR") {
        let path = PathBuf::from(path);
        if path.exists() && path.is_dir() && !found.contains(&path) {
            debug!(
                "Found ant-node data via ANT_NODE_DATA_DIR: {}",
                path.display()
            );
            found.push(path);
        }
    }

    found
}

/// Get the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    // Try directories crate first
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_ant_node_data_dirs_no_panic() {
        // Just ensure it doesn't panic
        let _ = find_ant_node_data_dirs();
    }
}
