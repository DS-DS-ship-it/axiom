use std::collections::BTreeSet;

use anyhow::{ensure, Result};
use serde_json::json;

use crate::{
    crypto::hash_hex,
    state::ChainState,
    types::{Block, Vote},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizationOutcome {
    pub block_hash: String,
    pub approving_power: u128,
    pub total_power: u128,
    pub quorum_threshold: u128,
}

pub fn block_id(block: &Block) -> Result<String> {
    let bytes = serde_json::to_vec(&json!({
        "header": block.header,
        "txs": block.txs,
    }))?;
    Ok(hash_hex(&bytes))
}

pub fn total_voting_power(state: &ChainState) -> u128 {
    state
        .validators
        .values()
        .map(|v| u128::from(v.power))
        .sum()
}

pub fn quorum_threshold(total_power: u128) -> u128 {
    if total_power == 0 {
        0
    } else {
        (total_power * 2) / 3 + 1
    }
}

pub fn approving_power(
    votes: &[Vote],
    state: &ChainState,
    block_hash: &str,
    height: u64,
    round: u32,
) -> u128 {
    let mut seen = BTreeSet::new();
    let mut power = 0u128;

    for vote in votes {
        if vote.block_hash != block_hash {
            continue;
        }
        if vote.height != height {
            continue;
        }
        if vote.round != round {
            continue;
        }
        if !seen.insert(vote.validator.clone()) {
            continue;
        }
        if let Some(v) = state.validators.get(&vote.validator) {
            power += u128::from(v.power);
        }
    }

    power
}

pub fn has_quorum(
    votes: &[Vote],
    state: &ChainState,
    block_hash: &str,
    height: u64,
    round: u32,
) -> bool {
    let total = total_voting_power(state);
    if total == 0 {
        return false;
    }
    let approving = approving_power(votes, state, block_hash, height, round);
    approving >= quorum_threshold(total)
}

pub fn finalize_block(
    block: &Block,
    votes: &[Vote],
    state: &mut ChainState,
) -> Result<FinalizationOutcome> {
    ensure!(block.header.height == state.height + 1, "bad block height");
    ensure!(block.header.parent_hash == state.tip_hash, "bad parent hash");

    let block_hash = block_id(block)?;
    let total = total_voting_power(state);
    let threshold = quorum_threshold(total);
    let approving = approving_power(
        votes,
        state,
        &block_hash,
        block.header.height,
        block.header.round,
    );

    ensure!(approving >= threshold, "insufficient quorum");

    state.height = block.header.height;
    state.tip_hash = block_hash.clone();

    Ok(FinalizationOutcome {
        block_hash,
        approving_power: approving,
        total_power: total,
        quorum_threshold: threshold,
    })
}
