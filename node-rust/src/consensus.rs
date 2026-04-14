use crate::{config::NodeConfig, state::ChainState};
use tracing::info;

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
