use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Account {
    pub balance: u128,
    pub nonce: u64,
    pub staked: u128,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contract {
    pub owner: String,
    pub code_hash: String,
    pub balance: u128,
    pub storage: BTreeMap<String, i128>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Validator {
    pub public_key: String,
    pub power: u64,
    pub stake: u128,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainState {
    pub accounts: BTreeMap<String, Account>,
    pub contracts: BTreeMap<String, Contract>,
    pub validators: BTreeMap<String, Validator>,
    pub total_burned: u128,
    pub total_tipped: u128,
    pub height: u64,
    pub tip_hash: String,
}
