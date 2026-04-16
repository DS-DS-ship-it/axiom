use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};

use crate::{
    finality::{
        block_id, quorum_threshold, total_voting_power, verify_block_signature,
        verify_vote_signature,
    },
    state::ChainState,
    types::{Block, Vote},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitCertificate {
    pub block: Block,
    pub block_hash: String,
    pub votes: Vec<Vote>,
    pub approving_power: u128,
    pub total_power: u128,
    pub quorum_threshold: u128,
}

fn ensure_no_conflicting_votes(votes: &[Vote], height: u64, round: u32) -> Result<()> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();

    for vote in votes {
        if vote.height != height || vote.round != round {
            return Err(anyhow!("vote height/round mismatch"));
        }

        if let Some(prev) = seen.get(&vote.validator) {
            if prev != &vote.block_hash {
                return Err(anyhow!("conflicting votes in certificate"));
            }
        } else {
            seen.insert(vote.validator.clone(), vote.block_hash.clone());
        }
    }

    Ok(())
}

fn compute_approving_power(
    votes: &[Vote],
    state: &ChainState,
    block_hash: &str,
    height: u64,
    round: u32,
) -> Result<u128> {
    ensure_no_conflicting_votes(votes, height, round)?;

    let mut seen = BTreeSet::new();
    let mut power = 0u128;

    for vote in votes {
        ensure!(vote.block_hash == block_hash, "vote block hash mismatch");
        verify_vote_signature(vote, state)?;

        if !seen.insert(vote.validator.clone()) {
            continue;
        }

        let validator = state
            .validators
            .get(&vote.validator)
            .ok_or_else(|| anyhow!("unknown validator"))?;

        power += u128::from(validator.power);
    }

    Ok(power)
}

pub fn build_commit_certificate(
    block: &Block,
    votes: &[Vote],
    state: &ChainState,
) -> Result<CommitCertificate> {
    verify_block_signature(block, state)?;

    let block_hash = block_id(block)?;
    let total_power = total_voting_power(state);
    ensure!(total_power > 0, "no validator power");

    let quorum = quorum_threshold(total_power);
    let approving = compute_approving_power(
        votes,
        state,
        &block_hash,
        block.header.height,
        block.header.round,
    )?;

    ensure!(approving >= quorum, "insufficient quorum");

    Ok(CommitCertificate {
        block: block.clone(),
        block_hash,
        votes: votes.to_vec(),
        approving_power: approving,
        total_power,
        quorum_threshold: quorum,
    })
}

pub fn verify_commit_certificate(cert: &CommitCertificate, state: &ChainState) -> Result<()> {
    verify_block_signature(&cert.block, state)?;

    let recomputed_hash = block_id(&cert.block)?;
    ensure!(
        recomputed_hash == cert.block_hash,
        "certificate block hash mismatch"
    );

    let total_power = total_voting_power(state);
    ensure!(
        total_power == cert.total_power,
        "certificate total power mismatch"
    );

    let quorum = quorum_threshold(total_power);
    ensure!(
        quorum == cert.quorum_threshold,
        "certificate quorum mismatch"
    );

    let approving = compute_approving_power(
        &cert.votes,
        state,
        &cert.block_hash,
        cert.block.header.height,
        cert.block.header.round,
    )?;
    ensure!(
        approving == cert.approving_power,
        "certificate approving power mismatch"
    );
    ensure!(approving >= quorum, "insufficient quorum");

    Ok(())
}

pub fn apply_commit_certificate(cert: &CommitCertificate, state: &mut ChainState) -> Result<()> {
    ensure!(
        cert.block.header.height == state.height + 1,
        "bad commit height"
    );
    ensure!(
        cert.block.header.parent_hash == state.tip_hash,
        "bad commit parent hash"
    );

    verify_commit_certificate(cert, state)?;

    state.height = cert.block.header.height;
    state.tip_hash = cert.block_hash.clone();

    Ok(())
}
