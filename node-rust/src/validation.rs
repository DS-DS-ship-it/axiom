use anyhow::{anyhow, ensure, Result};
use ed25519_dalek::VerifyingKey;
use serde_json::json;

use crate::{
    crypto::{address_from_vk, verify_bytes},
    state::ChainState,
    types::Transaction,
};

pub fn signing_bytes(tx: &Transaction) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({
        "chain_id": tx.chain_id,
        "kind": tx.kind,
        "sender": tx.sender,
        "sender_pubkey": tx.sender_pubkey,
        "nonce": tx.nonce,
        "gas_limit": tx.gas_limit,
        "max_fee_per_gas": tx.max_fee_per_gas,
        "value": tx.value,
        "recipient": tx.recipient,
        "data": tx.data,
        "timestamp_ms": tx.timestamp_ms,
    }))?)
}

pub fn max_total_cost(tx: &Transaction) -> Result<u128> {
    let fee = u128::from(tx.gas_limit)
        .checked_mul(u128::from(tx.max_fee_per_gas))
        .ok_or_else(|| anyhow!("fee overflow"))?;

    u128::from(tx.value)
        .checked_add(fee)
        .ok_or_else(|| anyhow!("total cost overflow"))
}

pub fn validate_transaction(
    tx: &Transaction,
    state: &ChainState,
    expected_chain_id: &str,
) -> Result<()> {
    ensure!(tx.chain_id == expected_chain_id, "wrong chain id");
    ensure!(tx.kind == "transfer", "unsupported tx kind");
    ensure!(tx.sender.starts_with("axm_"), "bad sender address");
    ensure!(tx.gas_limit > 0, "gas_limit must be > 0");
    ensure!(tx.max_fee_per_gas > 0, "max_fee_per_gas must be > 0");
    ensure!(tx.timestamp_ms > 0, "timestamp_ms must be > 0");

    let sender = state
        .accounts
        .get(&tx.sender)
        .ok_or_else(|| anyhow!("unknown sender"))?;

    ensure!(tx.nonce == sender.nonce, "bad nonce");

    let recipient = tx
        .recipient
        .as_ref()
        .ok_or_else(|| anyhow!("missing recipient"))?;
    ensure!(recipient.starts_with("axm_"), "bad recipient address");

    let vk_raw = hex::decode(&tx.sender_pubkey)?;
    let vk_arr: [u8; 32] = vk_raw
        .try_into()
        .map_err(|_| anyhow!("bad public key len"))?;
    let vk = VerifyingKey::from_bytes(&vk_arr)?;

    let derived_sender = address_from_vk(&vk);
    ensure!(derived_sender == tx.sender, "sender/pubkey mismatch");

    let bytes = signing_bytes(tx)?;
    verify_bytes(&tx.sender_pubkey, &bytes, &tx.signature)?;

    let total_cost = max_total_cost(tx)?;
    ensure!(sender.balance >= total_cost, "insufficient balance");

    Ok(())
}
