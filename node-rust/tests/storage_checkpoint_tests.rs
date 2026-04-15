use axiom_node::{
    checkpoint::{load_checkpoint_bundle, write_checkpoint_bundle},
    state::{Account, ChainState},
    storage_engine::{PersistedCursor, StorageEngine},
};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
}

#[test]
fn sled_storage_round_trip_state_and_cursor() {
    let path = temp_path("axiom-sled");
    let store = StorageEngine::open(&path).unwrap();

    let mut state = ChainState {
        tip_hash: "tip_1".to_string(),
        height: 1,
        ..Default::default()
    };
    state.accounts.insert(
        "alice".to_string(),
        Account {
            balance: 123,
            nonce: 2,
            staked: 0,
        },
    );

    store.save_chain_state(&state).unwrap();
    store
        .save_cursor(&PersistedCursor {
            checkpoint_id: Some("cp-1".to_string()),
            wal_entries_applied: 7,
        })
        .unwrap();
    store.flush().unwrap();

    let restored = store.load_chain_state().unwrap().unwrap();
    let cursor = store.load_cursor().unwrap();

    assert_eq!(restored.height, 1);
    assert_eq!(restored.tip_hash, "tip_1");
    assert_eq!(restored.accounts["alice"].balance, 123);
    assert_eq!(cursor.checkpoint_id.as_deref(), Some("cp-1"));
    assert_eq!(cursor.wal_entries_applied, 7);

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn checkpoint_bundle_round_trip() {
    let root = temp_path("axiom-checkpoint");
    let state = ChainState {
        height: 9,
        tip_hash: "tip_9".to_string(),
        ..Default::default()
    };

    let cursor = PersistedCursor {
        checkpoint_id: Some("cp-9".to_string()),
        wal_entries_applied: 19,
    };

    write_checkpoint_bundle(&root, "axiom-local", "cp-9", &cursor, &state).unwrap();
    let loaded = load_checkpoint_bundle(&root, "axiom-local").unwrap().unwrap();

    assert_eq!(loaded.0.checkpoint_id, "cp-9");
    assert_eq!(loaded.0.wal_entries_applied, 19);
    assert_eq!(loaded.1.height, 9);
    assert_eq!(loaded.1.tip_hash, "tip_9");

    let _ = std::fs::remove_dir_all(root);
}
