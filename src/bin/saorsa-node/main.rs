//! saorsa-node CLI entry point.

mod cli;

use clap::Parser;
use cli::Cli;
use saorsa_node::{NodeBuilder, NodeConfig};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cli.log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    info!("saorsa-node v{}", env!("CARGO_PKG_VERSION"));

    // Build configuration
    let config = cli.into_config()?;

    // Build and run the node
    let mut node = NodeBuilder::new(config).build().await?;

    // Run until shutdown
    node.run().await?;

    info!("Goodbye!");
    Ok(())
}
