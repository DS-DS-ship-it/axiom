use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

use crate::state::{Account, ChainState, Validator};

pub const AXIOM1_CHAIN_ID: &str = "axiom-1";
pub const AXIOM1_PROTOCOL_VERSION: u16 = 1;
pub const AXIOM1_BASE_FEE: u64 = 2;
pub const AXIOM1_CHECKPOINT_INTERVAL: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisValidator {
    pub name: String,
    pub public_key: String,
    pub bind_addr: String,
    pub power: u64,
    pub stake: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisAccount {
    pub address: String,
    pub balance: u128,
    pub nonce: u64,
    pub staked: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Axiom1Genesis {
    pub chain_id: String,
    pub protocol_version: u16,
    pub genesis_time_ms: u64,
    pub base_fee: u64,
    pub checkpoint_interval: u64,
    pub validators: Vec<GenesisValidator>,
    pub accounts: Vec<GenesisAccount>,
    pub metadata: BTreeMap<String, String>,
}

impl Axiom1Genesis {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let genesis: Self = serde_json::from_str(&text)?;
        ensure!(genesis.chain_id == AXIOM1_CHAIN_ID, "unexpected chain id");
        ensure!(
            genesis.protocol_version == AXIOM1_PROTOCOL_VERSION,
            "unexpected protocol version"
        );
        Ok(genesis)
    }

    pub fn write_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)?;
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn to_chain_state(&self) -> ChainState {
        let mut state = ChainState {
            tip_hash: "GENESIS".to_string(),
            ..Default::default()
        };

        for validator in &self.validators {
            state.validators.insert(
                validator.name.clone(),
                Validator {
                    public_key: validator.public_key.clone(),
                    power: validator.power,
                    stake: validator.stake,
                },
            );
        }

        for account in &self.accounts {
            state.accounts.insert(
                account.address.clone(),
                Account {
                    balance: account.balance,
                    nonce: account.nonce,
                    staked: account.staked,
                },
            );
        }

        state
    }

    pub fn checkpoint_interval(&self) -> u64 {
        self.checkpoint_interval
    }
}
