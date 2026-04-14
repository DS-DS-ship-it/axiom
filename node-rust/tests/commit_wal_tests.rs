use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axiom_node::{
    commit::{apply_commit_certificate, build_commit_certificate, verify_commit_certificate},
    crypto::{generate_key, public_key_hex, sign_bytes},
    finality::{block_id, block_signing_bytes, vote_signing_bytes},
    state::{ChainState, Validator},
    types::{Block, BlockHeader, Vote},
    wal::{append_certificate, read_certificates, replay_wal},
};
use ed25519_dalek::SigningKey;

fn temp_wal_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "axiom-commit-wal-{}-{}-{}.jsonl",
        name,
        std::process::id(),
        nanos
    ))
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

#[test]
fn build_and_verify_commit_certificate_for_signed_quorum() {
    let (state, keys) = sample_state_and_keys();
    let block = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("axiom-local", &block_hash, "val_a", 1, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &block_hash, "val_b", 1, 0, keys.get("val_b").unwrap()),
    ];

    let cert = build_commit_certificate(&block, &votes, &state).unwrap();
    verify_commit_certificate(&cert, &state).unwrap();

    assert_eq!(cert.block_hash, block_hash);
    assert_eq!(cert.approving_power, 75);
    assert_eq!(cert.total_power, 100);
    assert_eq!(cert.quorum_threshold, 67);
}

#[test]
fn apply_commit_certificate_updates_height_and_tip() {
    let (mut state, keys) = sample_state_and_keys();
    let block = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let block_hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("axiom-local", &block_hash, "val_a", 1, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &block_hash, "val_b", 1, 0, keys.get("val_b").unwrap()),
    ];

    let cert = build_commit_certificate(&block, &votes, &state).unwrap();
    apply_commit_certificate(&cert, &mut state).unwrap();

    assert_eq!(state.height, 1);
    assert_eq!(state.tip_hash, block_hash);
}

#[test]
fn wal_replay_restores_two_commits_after_restart() {
    let wal_path = temp_wal_path("replay");
    let (mut state, keys) = sample_state_and_keys();
    let restored_template = state.clone();

    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let hash1 = block_id(&block1).unwrap();
    let votes1 = vec![
        signed_vote("axiom-local", &hash1, "val_a", 1, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &hash1, "val_b", 1, 0, keys.get("val_b").unwrap()),
    ];
    let cert1 = build_commit_certificate(&block1, &votes1, &state).unwrap();
    append_certificate(&wal_path, &cert1).unwrap();
    apply_commit_certificate(&cert1, &mut state).unwrap();

    let block2 = signed_block("axiom-local", "val_b", keys.get("val_b").unwrap(), &state.tip_hash, 2);
    let hash2 = block_id(&block2).unwrap();
    let votes2 = vec![
        signed_vote("axiom-local", &hash2, "val_a", 2, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &hash2, "val_b", 2, 0, keys.get("val_b").unwrap()),
    ];
    let cert2 = build_commit_certificate(&block2, &votes2, &state).unwrap();
    append_certificate(&wal_path, &cert2).unwrap();

    let mut restored = restored_template.clone();
    let applied = replay_wal(&wal_path, &mut restored).unwrap();

    assert_eq!(applied, 2);
    assert_eq!(restored.height, 2);
    assert_eq!(restored.tip_hash, hash2);

    let certs = read_certificates(&wal_path).unwrap();
    assert_eq!(certs.len(), 2);

    let _ = fs::remove_file(wal_path);
}

#[test]
fn wal_replay_rejects_tampered_second_certificate_after_first_commit() {
    let wal_path = temp_wal_path("tamper");
    let (mut state, keys) = sample_state_and_keys();
    let restored_template = state.clone();

    let block1 = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let hash1 = block_id(&block1).unwrap();
    let votes1 = vec![
        signed_vote("axiom-local", &hash1, "val_a", 1, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &hash1, "val_b", 1, 0, keys.get("val_b").unwrap()),
    ];
    let cert1 = build_commit_certificate(&block1, &votes1, &state).unwrap();
    append_certificate(&wal_path, &cert1).unwrap();
    apply_commit_certificate(&cert1, &mut state).unwrap();

    let block2 = signed_block("axiom-local", "val_b", keys.get("val_b").unwrap(), &state.tip_hash, 2);
    let hash2 = block_id(&block2).unwrap();
    let votes2 = vec![
        signed_vote("axiom-local", &hash2, "val_a", 2, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &hash2, "val_b", 2, 0, keys.get("val_b").unwrap()),
    ];
    let mut cert2 = build_commit_certificate(&block2, &votes2, &state).unwrap();
    cert2.block_hash = "tampered_hash".to_string();
    append_certificate(&wal_path, &cert2).unwrap();

    let mut restored = restored_template.clone();
    let err = replay_wal(&wal_path, &mut restored).unwrap_err().to_string();

    assert!(err.contains("certificate block hash mismatch"));
    assert_eq!(restored.height, 1);
    assert_eq!(restored.tip_hash, hash1);

    let _ = fs::remove_file(wal_path);
}

#[test]
fn verify_commit_certificate_rejects_tampered_metadata() {
    let (state, keys) = sample_state_and_keys();
    let block = signed_block("axiom-local", "val_a", keys.get("val_a").unwrap(), "GENESIS", 1);
    let hash = block_id(&block).unwrap();

    let votes = vec![
        signed_vote("axiom-local", &hash, "val_a", 1, 0, keys.get("val_a").unwrap()),
        signed_vote("axiom-local", &hash, "val_b", 1, 0, keys.get("val_b").unwrap()),
    ];

    let mut cert = build_commit_certificate(&block, &votes, &state).unwrap();
    cert.approving_power = 999;

    let err = verify_commit_certificate(&cert, &state).unwrap_err().to_string();
    assert!(err.contains("certificate approving power mismatch"));
}
