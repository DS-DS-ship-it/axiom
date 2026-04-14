use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axiom_node::{
    commit::{build_commit_certificate, CommitCertificate},
    config::NodeConfig,
    consensus::ConsensusEngine,
    crypto::{generate_key, public_key_hex, sign_bytes},
    finality::{block_id, block_signing_bytes, vote_signing_bytes},
    state::{ChainState, Validator},
    types::{Block, BlockHeader, Vote},
    wal::append_certificate,
};
use ed25519_dalek::SigningKey;

fn temp_path(name: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "axiom-runtime-{}-{}-{}.{}",
        name,
        std::process::id(),
        nanos,
        ext
    ))
}

fn sample_config(state_path: PathBuf, wal_path: PathBuf) -> NodeConfig {
    NodeConfig {
        chain_id: "axiom-local".to_string(),
        node_name: "node1".to_string(),
        bind_addr: "127.0.0.1:7999".to_string(),
        p2p_peers: vec![],
        private_key_hex: "11".repeat(32),
        validator_power: 100,
        genesis_path: "testnet/genesis/genesis.json".to_string(),
        state_path: Some(state_path.to_string_lossy().to_string()),
        wal_path: Some(wal_path.to_string_lossy().to_string()),
    }
}

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

fn signed_block(
    chain_id: &str,
    proposer: &str,
    key: &SigningKey,
    parent_hash: &str,
    height: u64,
) -> Block {
    let mut block = Block {
        header: BlockHeader {
            chain_id: chain_id.to_string(),
            height,
            parent_hash: parent_hash.to_string(),
            proposer: proposer.to_string(),
            slot: height,
            round: 0,
            timestamp_ms: 1_700_000_000_000 + height,
            state_root: format!("state_root_{}", height),
            tx_root: format!("tx_root_{}", height),
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

fn build_cert(
    state: &ChainState,
    keys: &BTreeMap<String, SigningKey>,
    parent_hash: &str,
    height: u64,
    proposer: &str,
) -> CommitCertificate {
    let block = signed_block("axiom-local", proposer, keys.get(proposer).unwrap(), parent_hash, height);
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("axiom-local", &block_hash, "val_a", height, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &block_hash, "val_b", height, 0, keys.get("val_b").unwrap()),
    ];

    build_commit_certificate(&block, &votes, state).unwrap()
}

#[test]
fn runtime_engine_appends_commit_to_wal_and_restores_after_restart() {
    let state_path = temp_path("state", "json");
    let wal_path = temp_path("wal", "jsonl");
    let config = sample_config(state_path.clone(), wal_path.clone());

    let (base_state, keys) = sample_state_and_keys();
    let mut engine = ConsensusEngine::new(config.clone());
    engine.state = base_state.clone();

    let cert1 = build_cert(&engine.state, &keys, "GENESIS", 1, "val_a");
    engine.apply_commit_and_record(&cert1).unwrap();

    let cert2 = build_cert(&engine.state, &keys, &engine.state.tip_hash, 2, "val_b");
    engine.apply_commit_and_record(&cert2).unwrap();

    assert_eq!(engine.state.height, 2);
    assert_eq!(engine.wal_entries_applied, 2);

    let restored = ConsensusEngine::load_or_new(config).unwrap();
    assert_eq!(restored.state.height, 2);
    assert_eq!(restored.state.tip_hash, cert2.block_hash);
    assert_eq!(restored.wal_entries_applied, 2);

    let _ = fs::remove_file(state_path);
    let _ = fs::remove_file(wal_path);
}

#[test]
fn runtime_engine_replays_only_new_wal_entries_after_snapshot() {
    let state_path = temp_path("state-skip", "json");
    let wal_path = temp_path("wal-skip", "jsonl");
    let config = sample_config(state_path.clone(), wal_path.clone());

    let (base_state, keys) = sample_state_and_keys();
    let mut engine = ConsensusEngine::new(config.clone());
    engine.state = base_state.clone();

    let cert1 = build_cert(&engine.state, &keys, "GENESIS", 1, "val_a");
    engine.apply_commit_and_record(&cert1).unwrap();

    assert_eq!(engine.state.height, 1);
    assert_eq!(engine.wal_entries_applied, 1);

    let cert2 = build_cert(&engine.state, &keys, &engine.state.tip_hash, 2, "val_b");
    append_certificate(&wal_path, &cert2).unwrap();

    let restored = ConsensusEngine::load_or_new(config).unwrap();

    assert_eq!(restored.state.height, 2);
    assert_eq!(restored.state.tip_hash, cert2.block_hash);
    assert_eq!(restored.wal_entries_applied, 2);

    let _ = fs::remove_file(state_path);
    let _ = fs::remove_file(wal_path);
}
