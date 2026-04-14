use std::collections::BTreeMap;

use axiom_node::{
    crypto::{generate_key, public_key_hex, sign_bytes},
    finality::{block_id, block_signing_bytes, vote_signing_bytes},
    slashing::{
        build_equivocation_evidence, evidence_id, slash_for_equivocation, votes_conflict,
    },
    state::{ChainState, Validator},
    types::{Block, BlockHeader, Vote},
};
use ed25519_dalek::SigningKey;

fn sample_state_and_keys() -> (ChainState, BTreeMap<String, SigningKey>) {
    let key_a = generate_key();
    let key_b = generate_key();
    let key_c = generate_key();

    let mut state = ChainState {
        tip_hash: "GENESIS".to_string(),
        ..Default::default()
    };

    state.validators.insert(
        "val_a".to_string(),
        Validator {
            public_key: public_key_hex(&key_a),
            power: 40,
            stake: 1_000,
        },
    );
    state.validators.insert(
        "val_b".to_string(),
        Validator {
            public_key: public_key_hex(&key_b),
            power: 35,
            stake: 900,
        },
    );
    state.validators.insert(
        "val_c".to_string(),
        Validator {
            public_key: public_key_hex(&key_c),
            power: 25,
            stake: 800,
        },
    );

    let mut keys = BTreeMap::new();
    keys.insert("val_a".to_string(), key_a);
    keys.insert("val_b".to_string(), key_b);
    keys.insert("val_c".to_string(), key_c);

    (state, keys)
}

fn signed_block(chain_id: &str, proposer: &str, key: &SigningKey, parent: &str, height: u64) -> Block {
    let mut block = Block {
        header: BlockHeader {
            chain_id: chain_id.to_string(),
            height,
            parent_hash: parent.to_string(),
            proposer: proposer.to_string(),
            slot: height,
            round: 0,
            timestamp_ms: 1_700_000_000_000 + height,
            state_root: format!("state_root_{height}"),
            tx_root: format!("tx_root_{height}"),
            base_fee: 2,
            gas_used: 0,
        },
        txs: vec![],
        signature: String::new(),
    };

    block.signature = sign_bytes(key, &block_signing_bytes(&block).unwrap());
    block
}

fn signed_vote(
    chain_id: &str,
    block_hash: &str,
    validator: &str,
    height: u64,
    round: u32,
    key: &SigningKey,
) -> Vote {
    let mut vote = Vote {
        chain_id: chain_id.to_string(),
        block_hash: block_hash.to_string(),
        height,
        round,
        validator: validator.to_string(),
        signature: String::new(),
    };

    vote.signature = sign_bytes(key, &vote_signing_bytes(&vote).unwrap());
    vote
}

#[test]
fn votes_conflict_only_for_same_validator_height_round_and_different_hash() {
    let (state, keys) = sample_state_and_keys();
    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "OTHER_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_b", 1, 0, keys.get("val_b").unwrap());
    let b = signed_vote("axiom-local", &h2, "val_b", 1, 0, keys.get("val_b").unwrap());
    let c = signed_vote("axiom-local", &h2, "val_c", 1, 0, keys.get("val_c").unwrap());
    let d = signed_vote("axiom-local", &h2, "val_b", 2, 0, keys.get("val_b").unwrap());

    assert!(votes_conflict(&a, &b));
    assert!(!votes_conflict(&a, &c));
    assert!(!votes_conflict(&a, &d));
    assert!(state.validators.contains_key("val_b"));
}

#[test]
fn build_equivocation_evidence_accepts_valid_conflicting_signed_votes() {
    let (state, keys) = sample_state_and_keys();
    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "ALT_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_b", 1, 0, keys.get("val_b").unwrap());
    let b = signed_vote("axiom-local", &h2, "val_b", 1, 0, keys.get("val_b").unwrap());

    let evidence = build_equivocation_evidence(&a, &b, &state).unwrap();

    assert_eq!(evidence.validator, "val_b");
    assert_eq!(evidence.height, 1);
    assert_eq!(evidence.round, 0);
    assert_eq!(evidence.evidence_id, evidence_id(&a, &b).unwrap());
}

#[test]
fn build_equivocation_evidence_rejects_forged_vote() {
    let (state, keys) = sample_state_and_keys();
    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "ALT_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_b", 1, 0, keys.get("val_b").unwrap());
    let forged = signed_vote("axiom-local", &h2, "val_b", 1, 0, keys.get("val_c").unwrap());

    assert!(build_equivocation_evidence(&a, &forged, &state).is_err());
}

#[test]
fn slash_for_equivocation_reduces_stake_and_power_and_records_evidence() {
    let (mut state, keys) = sample_state_and_keys();
    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "ALT_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_b", 1, 0, keys.get("val_b").unwrap());
    let b = signed_vote("axiom-local", &h2, "val_b", 1, 0, keys.get("val_b").unwrap());

    let evidence = build_equivocation_evidence(&a, &b, &state).unwrap();
    let out = slash_for_equivocation(&mut state, &evidence, 2_000).unwrap();

    assert_eq!(out.validator, "val_b");
    assert_eq!(out.old_stake, 900);
    assert_eq!(out.slashed_amount, 180);
    assert_eq!(out.new_stake, 720);
    assert_eq!(out.old_power, 35);
    assert_eq!(out.new_power, 28);

    assert_eq!(state.validators["val_b"].stake, 720);
    assert_eq!(state.validators["val_b"].power, 28);
    assert_eq!(state.total_slashed, 180);
    assert!(state.applied_evidence_ids.contains(&evidence.evidence_id));
}

#[test]
fn slash_for_equivocation_rejects_duplicate_evidence() {
    let (mut state, keys) = sample_state_and_keys();
    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "ALT_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_b", 1, 0, keys.get("val_b").unwrap());
    let b = signed_vote("axiom-local", &h2, "val_b", 1, 0, keys.get("val_b").unwrap());

    let evidence = build_equivocation_evidence(&a, &b, &state).unwrap();
    slash_for_equivocation(&mut state, &evidence, 1_000).unwrap();

    let err = slash_for_equivocation(&mut state, &evidence, 1_000)
        .unwrap_err()
        .to_string();

    assert!(err.contains("evidence already applied"));
}

#[test]
fn slash_for_equivocation_minimum_penalty_is_one() {
    let (mut state, keys) = sample_state_and_keys();
    state.validators.get_mut("val_c").unwrap().stake = 1;
    state.validators.get_mut("val_c").unwrap().power = 1;

    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "ALT_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_c", 1, 0, keys.get("val_c").unwrap());
    let b = signed_vote("axiom-local", &h2, "val_c", 1, 0, keys.get("val_c").unwrap());

    let evidence = build_equivocation_evidence(&a, &b, &state).unwrap();
    let out = slash_for_equivocation(&mut state, &evidence, 1).unwrap();

    assert_eq!(out.slashed_amount, 1);
    assert_eq!(out.new_stake, 0);
    assert_eq!(out.new_power, 0);
}

#[test]
fn slash_for_equivocation_rejects_zero_penalty() {
    let (state, keys) = sample_state_and_keys();
    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block2 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "ALT_PARENT", 1);
    let h1 = block_id(&block1).unwrap();
    let h2 = block_id(&block2).unwrap();

    let a = signed_vote("axiom-local", &h1, "val_b", 1, 0, keys.get("val_b").unwrap());
    let b = signed_vote("axiom-local", &h2, "val_b", 1, 0, keys.get("val_b").unwrap());

    let evidence = build_equivocation_evidence(&a, &b, &state).unwrap();
    let mut state2 = state.clone();

    let err = slash_for_equivocation(&mut state2, &evidence, 0)
        .unwrap_err()
        .to_string();

    assert!(err.contains("penalty_bps must be > 0"));
}
