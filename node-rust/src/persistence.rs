use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

use crate::state::ChainState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub chain_id: String,
    pub node_name: String,
    pub state: ChainState,
}

fn temp_path_for(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", process::id()))
}

pub fn save_snapshot(
    path: impl AsRef<Path>,
    chain_id: &str,
    node_name: &str,
    state: &ChainState,
) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let snapshot = StateSnapshot {
        chain_id: chain_id.to_string(),
        node_name: node_name.to_string(),
        state: state.clone(),
    };

    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    let tmp_path = temp_path_for(path);

    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn load_snapshot(path: impl AsRef<Path>) -> Result<StateSnapshot> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn load_state_for_chain(
    path: impl AsRef<Path>,
    expected_chain_id: &str,
) -> Result<ChainState> {
    let snapshot = load_snapshot(path)?;
    ensure!(
        snapshot.chain_id == expected_chain_id,
        "wrong chain id in persisted state"
    );
    Ok(snapshot.state)
}
