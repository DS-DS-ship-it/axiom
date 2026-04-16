use std::collections::BTreeMap;

use axiom_node::{
    crypto::{generate_key, public_key_hex, sign_bytes},
    finality::{
        approving_power, block_id, block_signing_bytes, finalize_block, has_quorum,
        quorum_threshold, total_voting_power, verify_block_signature, verify_vote_signature,
        vote_signing_bytes,
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

fn sample_block() -> Block {
    Block {
        header: BlockHeader {
            chain_id: "axiom-local".to_string(),
            height: 1,
            parent_hash: "GENESIS".to_string(),
            proposer: "val_a".to_string(),
            slot: 1,
            round: 0,
            timestamp_ms: 1_700_000_000_000,
            state_root: "state_root_1".to_string(),
            tx_root: "tx_root_1".to_string(),
            base_fee: 2,
            gas_used: 0,
        },
        txs: vec![],
        signature: String::new(),
    }
}

fn sign_block(block: &mut Block, key: &SigningKey) {
    block.signature = sign_bytes(key, &block_signing_bytes(block).unwrap());
}

fn signed_vote(
    validator: &str,
    block_hash: &str,
    height: u64,
    round: u32,
    key: &SigningKey,
) -> Vote {
    let mut vote = Vote {
        chain_id: "axiom-local".to_string(),
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
fn quorum_threshold_math_is_correct() {
    assert_eq!(quorum_threshold(0), 0);
    assert_eq!(quorum_threshold(1), 1);
    assert_eq!(quorum_threshold(3), 3);
    assert_eq!(quorum_threshold(4), 3);
    assert_eq!(quorum_threshold(100), 67);
}

#[test]
fn total_voting_power_sums_validator_power() {
    let (state, _) = sample_state_and_keys();
    assert_eq!(total_voting_power(&state), 100);
}

#[test]
fn verify_block_signature_accepts_valid_block() {
    let (state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());

    verify_block_signature(&block, &state).unwrap();
}

#[test]
fn verify_block_signature_rejects_forged_block() {
    let (state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_b").unwrap());

    assert!(verify_block_signature(&block, &state).is_err());
}

#[test]
fn verify_vote_signature_accepts_valid_vote() {
    let (state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let vote = signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap());
    verify_vote_signature(&vote, &state).unwrap();
}

#[test]
fn verify_vote_signature_rejects_forged_vote() {
    let (state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let vote = signed_vote("val_b", &block_hash, 1, 0, keys.get("val_c").unwrap());
    assert!(verify_vote_signature(&vote, &state).is_err());
}

#[test]
fn approving_power_counts_only_valid_signed_votes_and_ignores_duplicates() {
    let (state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap()),
        signed_vote("val_c", "wrong_hash", 1, 0, keys.get("val_c").unwrap()),
        signed_vote("val_b", &block_hash, 2, 0, keys.get("val_b").unwrap()),
    ];

    assert_eq!(approving_power(&votes, &state, &block_hash, 1, 0), 75);
}

#[test]
fn has_quorum_requires_signed_votes_over_two_thirds_plus_one() {
    let (state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let yes_votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap()),
    ];
    let no_votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_c", &block_hash, 1, 0, keys.get("val_c").unwrap()),
    ];

    assert!(has_quorum(&yes_votes, &state, &block_hash, 1, 0));
    assert!(!has_quorum(&no_votes, &state, &block_hash, 1, 0));
}

#[test]
fn finalize_block_updates_height_and_tip_when_signatures_and_quorum_exist() {
    let (mut state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap()),
    ];

    let out = finalize_block(&block, &votes, &mut state).unwrap();

    assert_eq!(out.block_hash, block_hash);
    assert_eq!(out.total_power, 100);
    assert_eq!(out.quorum_threshold, 67);
    assert_eq!(out.approving_power, 75);
    assert_eq!(state.height, 1);
    assert_eq!(state.tip_hash, out.block_hash);
}

#[test]
fn finalize_block_rejects_forged_block_signature() {
    let (mut state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_b").unwrap());
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap()),
    ];

    let err = finalize_block(&block, &votes, &mut state)
        .unwrap_err()
        .to_string();

    assert!(err.contains("signature") || err.contains("verify"));
}

#[test]
fn finalize_block_rejects_forged_vote_signature() {
    let (mut state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_c").unwrap()),
    ];

    let err = finalize_block(&block, &votes, &mut state)
        .unwrap_err()
        .to_string();

    assert!(err.contains("insufficient quorum"));
}

#[test]
fn finalize_block_rejects_conflicting_votes() {
    let (mut state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote(
            "val_a",
            "different_block_hash",
            1,
            0,
            keys.get("val_a").unwrap(),
        ),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap()),
    ];

    let err = finalize_block(&block, &votes, &mut state)
        .unwrap_err()
        .to_string();

    assert!(err.contains("conflicting votes"));
}

#[test]
fn finalize_block_rejects_bad_parent_hash() {
    let (mut state, keys) = sample_state_and_keys();
    let mut block = sample_block();
    block.header.parent_hash = "NOT_GENESIS".to_string();
    sign_block(&mut block, keys.get("val_a").unwrap());
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("val_a", &block_hash, 1, 0, keys.get("val_a").unwrap()),
        signed_vote("val_b", &block_hash, 1, 0, keys.get("val_b").unwrap()),
    ];

    let err = finalize_block(&block, &votes, &mut state)
        .unwrap_err()
        .to_string();

    assert!(err.contains("bad parent hash"));
}
