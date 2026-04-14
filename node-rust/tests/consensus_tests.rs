use axiom_node::{
    config::NodeConfig,
    consensus::ConsensusEngine,
    state::Validator,
};

fn sample_config() -> NodeConfig {
    NodeConfig {
        chain_id: "axiom-local".to_string(),
        node_name: "test-node".to_string(),
        bind_addr: "127.0.0.1:7999".to_string(),
        p2p_peers: vec![],
        private_key_hex: "11".repeat(32),
        validator_power: 1,
        genesis_path: "testnet/genesis/genesis.json".to_string(),
        state_path: None,
        wal_path: None,
    }
}

#[test]
fn consensus_engine_starts_at_genesis() {
    let engine = ConsensusEngine::new(sample_config());

    assert_eq!(engine.state.height, 0);
    assert_eq!(engine.state.tip_hash, "GENESIS");
    assert!(engine.state.validators.is_empty());
    assert_eq!(engine.wal_entries_applied, 0);
}

#[test]
fn proposer_is_none_without_validators() {
    let engine = ConsensusEngine::new(sample_config());

    assert!(engine.proposer_for_height(1).is_none());
}

#[test]
fn proposer_rotates_in_sorted_validator_order() {
    let mut engine = ConsensusEngine::new(sample_config());

    engine.state.validators.insert(
        "val_b".to_string(),
        Validator {
            public_key: "pk_b".to_string(),
            power: 1,
            stake: 100,
        },
    );
    engine.state.validators.insert(
        "val_a".to_string(),
        Validator {
            public_key: "pk_a".to_string(),
            power: 1,
            stake: 100,
        },
    );
    engine.state.validators.insert(
        "val_c".to_string(),
        Validator {
            public_key: "pk_c".to_string(),
            power: 1,
            stake: 100,
        },
    );

    assert_eq!(engine.proposer_for_height(1).unwrap(), "val_a");
    assert_eq!(engine.proposer_for_height(2).unwrap(), "val_b");
    assert_eq!(engine.proposer_for_height(3).unwrap(), "val_c");
    assert_eq!(engine.proposer_for_height(4).unwrap(), "val_a");
}
