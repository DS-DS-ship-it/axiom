use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use axiom_node::{config::NodeConfig, devnet_runtime::DevnetRuntime};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    config: String,
    #[arg(long)]
    data_dir: String,
}

fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let args = Args::parse();
    let config = NodeConfig::load(&args.config)?;
    let runtime = DevnetRuntime::bootstrap(config, &args.data_dir)?;

    tracing::info!(
        chain_id = %runtime.genesis.chain_id,
        node = %runtime.config.node_name,
        height = runtime.state.height,
        tip = %runtime.state.tip_hash,
        wal_entries_applied = runtime.cursor.wal_entries_applied,
        checkpoint_id = ?runtime.cursor.checkpoint_id,
        "axiom-1 devnet runtime bootstrapped"
    );

    Ok(())
}
