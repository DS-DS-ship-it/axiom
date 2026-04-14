use axiom_node::{
    finality::{
        approving_power, block_id, finalize_block, has_quorum, quorum_threshold,
        total_voting_power,
    },
    state::{ChainState, Validator},
    types::{Block, BlockHeader, Vote},
};

fn sample_state() -> ChainState {
    let mut state = ChainState {
        tip_hash: "GENESIS".to_string(),
        ..Default::default()
    };

    state.validators.insert(
        "val_a".to_string(),
        Validator {
            public_key: "pk_a".to_string(),
            power: 40,
            stake: 1_000,
        },
    );
    state.validators.insert(
        "val_b".to_string(),
        Validator {
            public_key: "pk_b".to_string(),
            power: 35,
            stake: 900,
        },
    );
    state.validators.insert(
        "val_c".to_string(),
        Validator {
            public_key: "pk_c".to_string(),
            power: 25,
            stake: 800,
        },
    );

    state
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

fn vote(validator: &str, block_hash: &str, height: u64, round: u32) -> Vote {
    Vote {
        chain_id: "axiom-local".to_string(),
        block_hash: block_hash.to_string(),
        height,
        round,
        validator: validator.to_string(),
        signature: String::new(),
    }
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
    let state = sample_state();
    assert_eq!(total_voting_power(&state), 100);
}

#[test]
fn approving_power_ignores_duplicates_and_mismatches() {
    let state = sample_state();
    let block = sample_block();
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        vote("val_a", &block_hash, 1, 0),
        vote("val_a", &block_hash, 1, 0),
        vote("val_b", &block_hash, 1, 0),
        vote("val_c", "wrong_hash", 1, 0),
        vote("unknown", &block_hash, 1, 0),
    ];

    assert_eq!(approving_power(&votes, &state, &block_hash, 1, 0), 75);
}

#[test]
fn has_quorum_requires_two_thirds_plus_one() {
    let state = sample_state();
    let block = sample_block();
    let block_hash = block_id(&block).unwrap();

    let yes_votes = vec![
        vote("val_a", &block_hash, 1, 0),
        vote("val_b", &block_hash, 1, 0),
    ];
    let no_votes = vec![
        vote("val_a", &block_hash, 1, 0),
        vote("val_c", &block_hash, 1, 0),
    ];

    assert!(has_quorum(&yes_votes, &state, &block_hash, 1, 0));
    assert!(!has_quorum(&no_votes, &state, &block_hash, 1, 0));
}

#[test]
fn finalize_block_updates_height_and_tip_when_quorum_exists() {
    let mut state = sample_state();
    let block = sample_block();
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        vote("val_a", &block_hash, 1, 0),
        vote("val_b", &block_hash, 1, 0),
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
fn finalize_block_rejects_insufficient_quorum() {
    let mut state = sample_state();
    let block = sample_block();
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        vote("val_a", &block_hash, 1, 0),
        vote("val_c", &block_hash, 1, 0),
    ];

    let err = finalize_block(&block, &votes, &mut state)
        .unwrap_err()
        .to_string();

    assert!(err.contains("insufficient quorum"));
    assert_eq!(state.height, 0);
    assert_eq!(state.tip_hash, "GENESIS");
}

#[test]
fn finalize_block_rejects_bad_parent_hash() {
    let mut state = sample_state();
    let mut block = sample_block();
    block.header.parent_hash = "NOT_GENESIS".to_string();
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        vote("val_a", &block_hash, 1, 0),
        vote("val_b", &block_hash, 1, 0),
    ];

    let err = finalize_block(&block, &votes, &mut state)
        .unwrap_err()
        .to_string();

    assert!(err.contains("bad parent hash"));
}
