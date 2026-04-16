use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, ensure, Result};
use serde_json::json;

use crate::{
    crypto::{hash_hex, verify_bytes},
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

pub fn block_signing_bytes(block: &Block) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({
        "header": block.header,
        "txs": block.txs,
    }))?)
}

pub fn vote_signing_bytes(vote: &Vote) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({
        "chain_id": vote.chain_id,
        "block_hash": vote.block_hash,
        "height": vote.height,
        "round": vote.round,
        "validator": vote.validator,
    }))?)
}

pub fn block_id(block: &Block) -> Result<String> {
    Ok(hash_hex(&block_signing_bytes(block)?))
}

pub fn verify_block_signature(block: &Block, state: &ChainState) -> Result<()> {
    let proposer = state
        .validators
        .get(&block.header.proposer)
        .ok_or_else(|| anyhow!("unknown proposer"))?;

    let bytes = block_signing_bytes(block)?;
    verify_bytes(&proposer.public_key, &bytes, &block.signature)?;
    Ok(())
}

pub fn verify_vote_signature(vote: &Vote, state: &ChainState) -> Result<()> {
    let validator = state
        .validators
        .get(&vote.validator)
        .ok_or_else(|| anyhow!("unknown validator"))?;

    let bytes = vote_signing_bytes(vote)?;
    verify_bytes(&validator.public_key, &bytes, &vote.signature)?;
    Ok(())
}

pub fn total_voting_power(state: &ChainState) -> u128 {
    state.validators.values().map(|v| u128::from(v.power)).sum()
}

pub fn quorum_threshold(total_power: u128) -> u128 {
    if total_power == 0 {
        0
    } else {
        (total_power * 2) / 3 + 1
    }
}

fn ensure_no_conflicting_votes(votes: &[Vote], height: u64, round: u32) -> Result<()> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();

    for vote in votes {
        if vote.height != height || vote.round != round {
            continue;
        }

        if let Some(prev_hash) = seen.get(&vote.validator) {
            if prev_hash != &vote.block_hash {
                return Err(anyhow!(
                    "conflicting votes from validator {}",
                    vote.validator
                ));
            }
        } else {
            seen.insert(vote.validator.clone(), vote.block_hash.clone());
        }
    }

    Ok(())
}

pub fn approving_power(
    votes: &[Vote],
    state: &ChainState,
    block_hash: &str,
    height: u64,
    round: u32,
) -> u128 {
    if ensure_no_conflicting_votes(votes, height, round).is_err() {
        return 0;
    }

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
        if verify_vote_signature(vote, state).is_err() {
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
    ensure!(
        block.header.parent_hash == state.tip_hash,
        "bad parent hash"
    );

    verify_block_signature(block, state)?;
    ensure_no_conflicting_votes(votes, block.header.height, block.header.round)?;

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
