use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use axiom_node::{config::NodeConfig, consensus::ConsensusEngine, crypto, network};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let args = Args::parse();
    let config = NodeConfig::load(&args.config)?;
    let sk = crypto::signing_key_from_hex(&config.private_key_hex)?;
    let node_addr = crypto::address_from_vk(&sk.verifying_key());

    let mut engine = ConsensusEngine::new(config.clone());
    tracing::info!(node = %config.node_name, address = %node_addr, bind = %config.bind_addr, "validator starting");

    let bind_addr = config.bind_addr.clone();
    tokio::spawn(async move {
        if let Err(err) = network::run_listener(&bind_addr).await {
            tracing::error!(error = %err, "listener failed");
        }
    });

    loop {
        engine.tick();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}
