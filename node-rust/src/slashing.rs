use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::hash_hex,
    finality::{verify_vote_signature, vote_signing_bytes},
    state::ChainState,
    types::Vote,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivocationEvidence {
    pub validator: String,
    pub height: u64,
    pub round: u32,
    pub vote_a: Vote,
    pub vote_b: Vote,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashingOutcome {
    pub validator: String,
    pub evidence_id: String,
    pub slashed_amount: u128,
    pub old_stake: u128,
    pub new_stake: u128,
    pub old_power: u64,
    pub new_power: u64,
}

pub fn votes_conflict(a: &Vote, b: &Vote) -> bool {
    a.validator == b.validator
        && a.height == b.height
        && a.round == b.round
        && a.block_hash != b.block_hash
}

pub fn evidence_id(a: &Vote, b: &Vote) -> Result<String> {
    let (left, right) = if a.block_hash <= b.block_hash {
        (a, b)
    } else {
        (b, a)
    };

    let payload = serde_json::to_vec(&(vote_signing_bytes(left)?, vote_signing_bytes(right)?))?;
    Ok(hash_hex(&payload))
}

pub fn build_equivocation_evidence(
    a: &Vote,
    b: &Vote,
    state: &ChainState,
) -> Result<EquivocationEvidence> {
    ensure!(votes_conflict(a, b), "votes do not conflict");
    verify_vote_signature(a, state)?;
    verify_vote_signature(b, state)?;

    let id = evidence_id(a, b)?;
    Ok(EquivocationEvidence {
        validator: a.validator.clone(),
        height: a.height,
        round: a.round,
        vote_a: a.clone(),
        vote_b: b.clone(),
        evidence_id: id,
    })
}

pub fn slash_for_equivocation(
    state: &mut ChainState,
    evidence: &EquivocationEvidence,
    penalty_bps: u64,
) -> Result<SlashingOutcome> {
    ensure!(penalty_bps > 0, "penalty_bps must be > 0");
    ensure!(
        !state.applied_evidence_ids.contains(&evidence.evidence_id),
        "evidence already applied"
    );

    let validator = state
        .validators
        .get_mut(&evidence.validator)
        .ok_or_else(|| anyhow!("unknown validator"))?;

    ensure!(validator.stake > 0, "validator has no stake");

    let old_stake = validator.stake;
    let old_power = validator.power;

    let mut slashed_amount = (old_stake * u128::from(penalty_bps)) / 10_000u128;
    if slashed_amount == 0 {
        slashed_amount = 1;
    }
    if slashed_amount > old_stake {
        slashed_amount = old_stake;
    }

    let new_stake = old_stake - slashed_amount;
    let new_power = if old_stake == 0 || new_stake == 0 {
        0
    } else {
        ((u128::from(old_power) * new_stake) / old_stake) as u64
    };

    validator.stake = new_stake;
    validator.power = new_power;

    state.total_slashed += slashed_amount;
    state
        .applied_evidence_ids
        .insert(evidence.evidence_id.clone());

    Ok(SlashingOutcome {
        validator: evidence.validator.clone(),
        evidence_id: evidence.evidence_id.clone(),
        slashed_amount,
        old_stake,
        new_stake,
        old_power,
        new_power,
    })
}
