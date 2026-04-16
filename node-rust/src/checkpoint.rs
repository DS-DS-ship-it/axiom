use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use crate::{state::ChainState, storage_engine::PersistedCursor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointManifest {
    pub chain_id: String,
    pub checkpoint_id: String,
    pub wal_entries_applied: u64,
}

fn temp_path_for(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", process::id()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let tmp_path = temp_path_for(path);
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn write_checkpoint_bundle(
    root: impl AsRef<Path>,
    chain_id: &str,
    checkpoint_id: &str,
    cursor: &PersistedCursor,
    state: &ChainState,
) -> Result<()> {
    let root = root.as_ref();
    let checkpoints_dir = root.join("checkpoints");
    let manifest_path = root.join("manifest.json");
    let snapshot_path = checkpoints_dir.join(format!("{}.json", checkpoint_id));

    let snapshot_bytes = serde_json::to_vec_pretty(state)?;
    atomic_write(&snapshot_path, &snapshot_bytes)?;

    let manifest = CheckpointManifest {
        chain_id: chain_id.to_string(),
        checkpoint_id: checkpoint_id.to_string(),
        wal_entries_applied: cursor.wal_entries_applied,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    atomic_write(&manifest_path, &manifest_bytes)?;
    Ok(())
}

pub fn load_checkpoint_bundle(
    root: impl AsRef<Path>,
    expected_chain_id: &str,
) -> Result<Option<(CheckpointManifest, ChainState)>> {
    let root = root.as_ref();
    let manifest_path = root.join("manifest.json");
    if !manifest_path.exists() {
        return Ok(None);
    }

    let manifest: CheckpointManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    ensure!(
        manifest.chain_id == expected_chain_id,
        "wrong chain id in checkpoint manifest"
    );

    let snapshot_path = root
        .join("checkpoints")
        .join(format!("{}.json", manifest.checkpoint_id));
    let state: ChainState = serde_json::from_slice(&fs::read(snapshot_path)?)?;
    Ok(Some((manifest, state)))
}
