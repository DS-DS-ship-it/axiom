use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub chain_id: String,
    pub kind: String,
    pub sender: String,
    pub sender_pubkey: String,
    pub nonce: u64,
    pub gas_limit: u64,
    pub max_fee_per_gas: u64,
    pub value: u64,
    pub recipient: Option<String>,
    pub data: Option<serde_json::Value>,
    pub timestamp_ms: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub chain_id: String,
    pub block_hash: String,
    pub height: u64,
    pub round: u32,
    pub validator: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub chain_id: String,
    pub height: u64,
    pub parent_hash: String,
    pub proposer: String,
    pub slot: u64,
    pub round: u32,
    pub timestamp_ms: u64,
    pub state_root: String,
    pub tx_root: String,
    pub base_fee: u64,
    pub gas_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Transaction>,
    pub signature: String,
}
