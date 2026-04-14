use anyhow::{anyhow, Result};

use crate::{
    state::ChainState,
    types::Transaction,
    validation::{max_total_cost, validate_transaction},
};

#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub burned_fee: u128,
    pub tipped_fee: u128,
    pub transferred_value: u128,
}

pub fn apply_transfer(
    tx: &Transaction,
    state: &mut ChainState,
    expected_chain_id: &str,
    base_fee_per_gas: u64,
) -> Result<ApplyResult> {
    validate_transaction(tx, state, expected_chain_id)?;

    if base_fee_per_gas == 0 {
        return Err(anyhow!("base_fee_per_gas must be > 0"));
    }
    if tx.max_fee_per_gas < base_fee_per_gas {
        return Err(anyhow!("max_fee_per_gas below base fee"));
    }

    let recipient_addr = tx
        .recipient
        .clone()
        .ok_or_else(|| anyhow!("missing recipient"))?;

    let burned_fee = u128::from(tx.gas_limit) * u128::from(base_fee_per_gas);
    let max_total = max_total_cost(tx)?;
    let tip_fee = u128::from(tx.gas_limit) * u128::from(tx.max_fee_per_gas - base_fee_per_gas);
    let value = u128::from(tx.value);

    let sender = state
        .accounts
        .get_mut(&tx.sender)
        .ok_or_else(|| anyhow!("unknown sender"))?;

    if sender.balance < max_total {
        return Err(anyhow!("insufficient balance"));
    }

    sender.balance -= max_total;
    sender.nonce += 1;

    let recipient = state
        .accounts
        .entry(recipient_addr)
        .or_default();

    recipient.balance += value;

    state.total_burned += burned_fee;
    state.total_tipped += tip_fee;

    Ok(ApplyResult {
        burned_fee,
        tipped_fee: tip_fee,
        transferred_value: value,
    })
}
