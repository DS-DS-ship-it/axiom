use std::path::Path;

use tracing::info;

use crate::{
    config::NodeConfig,
    error::Result,
    persistence,
    state::ChainState,
};

#[derive(Debug)]
pub struct ConsensusEngine {
    pub config: NodeConfig,
    pub state: ChainState,
}

impl ConsensusEngine {
    pub fn new(config: NodeConfig) -> Self {
        Self {
            config,
            state: ChainState {
                tip_hash: "GENESIS".to_string(),
                ..Default::default()
            },
        }
    }

    pub fn with_state(config: NodeConfig, state: ChainState) -> Self {
        Self { config, state }
    }

    pub fn load_or_new(config: NodeConfig) -> Result<Self> {
        if let Some(path) = config.state_path.as_deref() {
            if Path::new(path).exists() {
                let state = persistence::load_state_for_chain(path, &config.chain_id)?;
                return Ok(Self::with_state(config, state));
            }
        }

        Ok(Self::new(config))
    }

    pub fn persist_if_configured(&self) -> Result<()> {
        if let Some(path) = self.config.state_path.as_deref() {
            persistence::save_snapshot(
                path,
                &self.config.chain_id,
                &self.config.node_name,
                &self.state,
            )?;
        }
        Ok(())
    }

    pub fn proposer_for_height(&self, height: u64) -> Option<String> {
        let keys: Vec<String> = self.state.validators.keys().cloned().collect();
        if keys.is_empty() {
            None
        } else {
            Some(keys[((height - 1) as usize) % keys.len()].clone())
        }
    }

    pub fn tick(&mut self) {
        info!(height = self.state.height, tip = %self.state.tip_hash, "consensus tick");
        // Placeholder for block proposal, vote aggregation, and finalization.
    }
}
