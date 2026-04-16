use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    crypto::hash_hex,
    execution::ApplyResult,
    state::ChainState,
    types::{Block, Transaction},
    validation::validate_transaction,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxReceipt {
    pub tx_hash: String,
    pub success: bool,
    pub gas_used: u64,
    pub burned_fee: u128,
    pub tipped_fee: u128,
    pub transferred_value: u128,
    pub post_state_root: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockExecutionResult {
    pub block_hash: String,
    pub tx_root: String,
    pub state_root: String,
    pub gas_used: u64,
    pub receipts: Vec<TxReceipt>,
    pub post_state: ChainState,
}

pub fn canonical_state_root(state: &ChainState) -> Result<String> {
    let bytes = serde_json::to_vec(state)?;
    Ok(hash_hex(&bytes))
}

pub fn canonical_tx_hash(tx: &Transaction) -> Result<String> {
    let bytes = serde_json::to_vec(tx)?;
    Ok(hash_hex(&bytes))
}

pub fn canonical_block_hash(block: &Block) -> Result<String> {
    let bytes = serde_json::to_vec(&json!({
        "header": block.header,
        "txs": block.txs,
    }))?;
    Ok(hash_hex(&bytes))
}

pub fn canonical_receipt_root(receipts: &[TxReceipt]) -> Result<String> {
    let bytes = serde_json::to_vec(receipts)?;
    Ok(hash_hex(&bytes))
}

pub fn apply_transfer_deterministic(
    tx: &Transaction,
    state: &mut ChainState,
    expected_chain_id: &str,
    base_fee_per_gas: u64,
) -> Result<ApplyResult> {
    validate_transaction(tx, state, expected_chain_id)?;

    ensure!(tx.kind == "transfer", "unsupported tx kind");
    ensure!(base_fee_per_gas > 0, "base_fee_per_gas must be > 0");
    ensure!(
        tx.max_fee_per_gas >= base_fee_per_gas,
        "max_fee_per_gas below base fee"
    );

    let recipient_addr = tx
        .recipient
        .clone()
        .ok_or_else(|| anyhow!("missing recipient"))?;

    let burned_fee = u128::from(tx.gas_limit) * u128::from(base_fee_per_gas);
    let tipped_fee = u128::from(tx.gas_limit) * u128::from(tx.max_fee_per_gas - base_fee_per_gas);
    let transferred_value = u128::from(tx.value);
    let total_cost = burned_fee + tipped_fee + transferred_value;

    let sender = state
        .accounts
        .get_mut(&tx.sender)
        .ok_or_else(|| anyhow!("unknown sender"))?;

    ensure!(sender.balance >= total_cost, "insufficient balance");

    sender.balance -= total_cost;
    sender.nonce += 1;

    let recipient = state.accounts.entry(recipient_addr).or_default();
    recipient.balance += transferred_value;

    state.total_burned += burned_fee;
    state.total_tipped += tipped_fee;

    Ok(ApplyResult {
        burned_fee,
        tipped_fee,
        transferred_value,
    })
}

pub fn apply_transaction_deterministic(
    tx: &Transaction,
    state: &mut ChainState,
    expected_chain_id: &str,
    base_fee_per_gas: u64,
) -> TxReceipt {
    let tx_hash = canonical_tx_hash(tx).unwrap_or_else(|_| "tx_hash_error".to_string());

    let outcome = match tx.kind.as_str() {
        "transfer" => apply_transfer_deterministic(tx, state, expected_chain_id, base_fee_per_gas),
        _ => Err(anyhow!("unsupported tx kind")),
    };

    match outcome {
        Ok(out) => TxReceipt {
            tx_hash,
            success: true,
            gas_used: tx.gas_limit,
            burned_fee: out.burned_fee,
            tipped_fee: out.tipped_fee,
            transferred_value: out.transferred_value,
            post_state_root: canonical_state_root(state)
                .unwrap_or_else(|_| "state_root_error".to_string()),
            error: None,
        },
        Err(err) => TxReceipt {
            tx_hash,
            success: false,
            gas_used: 0,
            burned_fee: 0,
            tipped_fee: 0,
            transferred_value: 0,
            post_state_root: canonical_state_root(state)
                .unwrap_or_else(|_| "state_root_error".to_string()),
            error: Some(err.to_string()),
        },
    }
}

pub fn execute_block_deterministic(
    block: &Block,
    pre_state: &ChainState,
    expected_chain_id: &str,
) -> Result<BlockExecutionResult> {
    ensure!(
        block.header.chain_id == expected_chain_id,
        "wrong block chain id"
    );

    let mut working_state = pre_state.clone();
    let mut receipts = Vec::with_capacity(block.txs.len());
    let mut gas_used = 0u64;

    for tx in &block.txs {
        let receipt = apply_transaction_deterministic(
            tx,
            &mut working_state,
            expected_chain_id,
            block.header.base_fee,
        );
        gas_used = gas_used.saturating_add(receipt.gas_used);
        receipts.push(receipt);
    }

    let tx_root = canonical_receipt_root(&receipts)?;
    let state_root = canonical_state_root(&working_state)?;
    let block_hash = canonical_block_hash(block)?;

    Ok(BlockExecutionResult {
        block_hash,
        tx_root,
        state_root,
        gas_used,
        receipts,
        post_state: working_state,
    })
}
