use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub chain_id: String,
    pub node_name: String,
    pub bind_addr: String,
    pub p2p_peers: Vec<String>,
    pub private_key_hex: String,
    pub validator_power: u64,
    pub genesis_path: String,
    #[serde(default)]
    pub state_path: Option<String>,
    #[serde(default)]
    pub wal_path: Option<String>,
}

impl NodeConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}
