use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axiom_node::{
    config::NodeConfig,
    consensus::ConsensusEngine,
    persistence::{load_snapshot, save_snapshot},
    state::{Account, ChainState, Validator},
};

fn temp_state_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "axiom-{}-{}-{}.json",
        name,
        std::process::id(),
        nanos
    ))
}

fn sample_config(state_path: PathBuf, chain_id: &str) -> NodeConfig {
    NodeConfig {
        chain_id: chain_id.to_string(),
        node_name: "node1".to_string(),
        bind_addr: "127.0.0.1:7999".to_string(),
        p2p_peers: vec![],
        private_key_hex: "11".repeat(32),
        validator_power: 100,
        genesis_path: "testnet/genesis/genesis.json".to_string(),
        state_path: Some(state_path.to_string_lossy().to_string()),
        wal_path: None,
    }
}

#[test]
fn snapshot_round_trip_restores_saved_state() {
    let path = temp_state_path("roundtrip");

    let mut state = ChainState {
        height: 7,
        tip_hash: "tip_7".to_string(),
        total_burned: 12,
        total_tipped: 4,
        ..Default::default()
    };
    state.accounts.insert(
        "alice".to_string(),
        Account {
            balance: 1234,
            nonce: 9,
            staked: 55,
        },
    );
    state.validators.insert(
        "node1".to_string(),
        Validator {
            public_key: "pk_1".to_string(),
            power: 100,
            stake: 1000,
        },
    );

    save_snapshot(&path, "axiom-local", "node1", 3, &state).unwrap();
    let snapshot = load_snapshot(&path).unwrap();

    assert_eq!(snapshot.chain_id, "axiom-local");
    assert_eq!(snapshot.node_name, "node1");
    assert_eq!(snapshot.wal_entries_applied, 3);
    assert_eq!(snapshot.state.height, 7);
    assert_eq!(snapshot.state.tip_hash, "tip_7");
    assert_eq!(snapshot.state.total_burned, 12);
    assert_eq!(snapshot.state.accounts["alice"].balance, 1234);
    assert_eq!(snapshot.state.validators["node1"].power, 100);

    let _ = fs::remove_file(path);
}

#[test]
fn engine_load_or_new_restores_state_from_disk() {
    let path = temp_state_path("restore");
    let config = sample_config(path.clone(), "axiom-local");

    let mut engine = ConsensusEngine::new(config.clone());
    engine.state.height = 42;
    engine.state.tip_hash = "tip_42".to_string();
    engine.state.total_burned = 99;
    engine.wal_entries_applied = 5;
    engine.state.accounts.insert(
        "alice".to_string(),
        Account {
            balance: 9999,
            nonce: 3,
            staked: 0,
        },
    );
    engine.state.validators.insert(
        "node1".to_string(),
        Validator {
            public_key: "pk_1".to_string(),
            power: 100,
            stake: 1000,
        },
    );

    engine.persist_if_configured().unwrap();

    let restored = ConsensusEngine::load_or_new(config).unwrap();

    assert_eq!(restored.state.height, 42);
    assert_eq!(restored.state.tip_hash, "tip_42");
    assert_eq!(restored.state.total_burned, 99);
    assert_eq!(restored.state.accounts["alice"].balance, 9999);
    assert_eq!(restored.state.validators["node1"].stake, 1000);
    assert_eq!(restored.wal_entries_applied, 5);

    let _ = fs::remove_file(path);
}

#[test]
fn engine_rejects_wrong_chain_id_snapshot() {
    let path = temp_state_path("wrong-chain");
    let config = sample_config(path.clone(), "axiom-local");

    let state = ChainState {
        height: 3,
        tip_hash: "tip_3".to_string(),
        ..Default::default()
    };

    save_snapshot(&path, "different-chain", "node1", 0, &state).unwrap();

    let err = ConsensusEngine::load_or_new(config)
        .unwrap_err()
        .to_string();

    assert!(err.contains("wrong chain id"));

    let _ = fs::remove_file(path);
}

#[test]
fn repeated_persist_overwrites_snapshot_for_restart() {
    let path = temp_state_path("overwrite");
    let config = sample_config(path.clone(), "axiom-local");

    let mut engine = ConsensusEngine::new(config.clone());
    engine.state.height = 1;
    engine.state.tip_hash = "tip_1".to_string();
    engine.wal_entries_applied = 1;
    engine.persist_if_configured().unwrap();

    engine.state.height = 2;
    engine.state.tip_hash = "tip_2".to_string();
    engine.state.total_tipped = 77;
    engine.wal_entries_applied = 4;
    engine.persist_if_configured().unwrap();

    let restored = ConsensusEngine::load_or_new(config).unwrap();

    assert_eq!(restored.state.height, 2);
    assert_eq!(restored.state.tip_hash, "tip_2");
    assert_eq!(restored.state.total_tipped, 77);
    assert_eq!(restored.wal_entries_applied, 4);

    let _ = fs::remove_file(path);
}
