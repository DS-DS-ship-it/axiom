use std::path::Path;

use tracing::info;

use crate::{
    commit::{apply_commit_certificate, CommitCertificate},
    config::NodeConfig,
    error::Result,
    persistence,
    state::ChainState,
    wal,
};

#[derive(Debug)]
pub struct ConsensusEngine {
    pub config: NodeConfig,
    pub state: ChainState,
    pub wal_entries_applied: usize,
}

impl ConsensusEngine {
    pub fn new(config: NodeConfig) -> Self {
        Self {
            config,
            state: ChainState {
                tip_hash: "GENESIS".to_string(),
                ..Default::default()
            },
            wal_entries_applied: 0,
        }
    }

    pub fn with_state(config: NodeConfig, state: ChainState, wal_entries_applied: usize) -> Self {
        Self {
            config,
            state,
            wal_entries_applied,
        }
    }

    pub fn load_or_new(config: NodeConfig) -> Result<Self> {
        let (mut state, mut wal_entries_applied) = if let Some(path) = config.state_path.as_deref()
        {
            if Path::new(path).exists() {
                let snapshot = persistence::load_snapshot_for_chain(path, &config.chain_id)?;
                (snapshot.state, snapshot.wal_entries_applied)
            } else {
                (
                    ChainState {
                        tip_hash: "GENESIS".to_string(),
                        ..Default::default()
                    },
                    0,
                )
            }
        } else {
            (
                ChainState {
                    tip_hash: "GENESIS".to_string(),
                    ..Default::default()
                },
                0,
            )
        };

        if let Some(path) = config.wal_path.as_deref() {
            if Path::new(path).exists() {
                wal_entries_applied += wal::replay_wal_from(path, &mut state, wal_entries_applied)?;
            }
        }

        Ok(Self::with_state(config, state, wal_entries_applied))
    }

    pub fn persist_if_configured(&self) -> Result<()> {
        if let Some(path) = self.config.state_path.as_deref() {
            persistence::save_snapshot(
                path,
                &self.config.chain_id,
                &self.config.node_name,
                self.wal_entries_applied,
                &self.state,
            )?;
        }
        Ok(())
    }

    pub fn apply_commit_and_record(&mut self, cert: &CommitCertificate) -> Result<()> {
        let mut next_state = self.state.clone();
        apply_commit_certificate(cert, &mut next_state)?;

        if let Some(path) = self.config.wal_path.as_deref() {
            wal::append_certificate(path, cert)?;
            self.wal_entries_applied += 1;
        }

        self.state = next_state;
        self.persist_if_configured()?;
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
    }
}
