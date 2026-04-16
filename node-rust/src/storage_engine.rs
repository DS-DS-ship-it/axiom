use anyhow::{anyhow, ensure, Result};
use serde::{de::DeserializeOwned, Serialize};
use sled::{self, Db, Tree};
use std::path::Path;

use crate::{commit::CommitCertificate, state::ChainState};

const META: &str = "meta";
const STATE: &str = "state";
const WAL: &str = "wal";
const CHECKPOINTS: &str = "checkpoints";

#[derive(Debug, Clone)]
pub struct StorageEngine {
    db: Db,
    meta: Tree,
    state: Tree,
    wal: Tree,
    checkpoints: Tree,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PersistedCursor {
    pub checkpoint_id: Option<String>,
    pub wal_entries_applied: u64,
}

impl StorageEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self {
            meta: db.open_tree(META)?,
            state: db.open_tree(STATE)?,
            wal: db.open_tree(WAL)?,
            checkpoints: db.open_tree(CHECKPOINTS)?,
            db,
        })
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    fn get_json<T: DeserializeOwned>(tree: &Tree, key: &[u8]) -> Result<Option<T>> {
        match tree.get(key)? {
            Some(v) => Ok(Some(serde_json::from_slice(&v)?)),
            None => Ok(None),
        }
    }

    fn set_json<T: Serialize>(tree: &Tree, key: &[u8], value: &T) -> Result<()> {
        tree.insert(key, serde_json::to_vec(value)?)?;
        Ok(())
    }

    pub fn save_chain_state(&self, state: &ChainState) -> Result<()> {
        Self::set_json(&self.state, b"chain_state", state)?;
        Ok(())
    }

    pub fn load_chain_state(&self) -> Result<Option<ChainState>> {
        Self::get_json(&self.state, b"chain_state")
    }

    pub fn save_cursor(&self, cursor: &PersistedCursor) -> Result<()> {
        Self::set_json(&self.meta, b"cursor", cursor)?;
        Ok(())
    }

    pub fn load_cursor(&self) -> Result<PersistedCursor> {
        Ok(
            Self::get_json(&self.meta, b"cursor")?.unwrap_or(PersistedCursor {
                checkpoint_id: None,
                wal_entries_applied: 0,
            }),
        )
    }

    pub fn append_wal_certificate(&self, seq: u64, cert: &CommitCertificate) -> Result<()> {
        let key = seq.to_be_bytes();
        Self::set_json(&self.wal, &key, cert)?;
        Ok(())
    }

    pub fn read_wal_from(&self, start_seq: u64) -> Result<Vec<(u64, CommitCertificate)>> {
        let mut out = Vec::new();
        for item in self.wal.range(start_seq.to_be_bytes()..) {
            let (k, v) = item?;
            ensure!(k.len() == 8, "bad WAL key length");
            let seq =
                u64::from_be_bytes(k.as_ref().try_into().map_err(|_| anyhow!("bad WAL key"))?);
            let cert = serde_json::from_slice::<CommitCertificate>(&v)?;
            out.push((seq, cert));
        }
        Ok(out)
    }

    pub fn put_checkpoint(&self, checkpoint_id: &str, state: &ChainState) -> Result<()> {
        Self::set_json(&self.checkpoints, checkpoint_id.as_bytes(), state)?;
        Ok(())
    }

    pub fn load_checkpoint(&self, checkpoint_id: &str) -> Result<Option<ChainState>> {
        Self::get_json(&self.checkpoints, checkpoint_id.as_bytes())
    }

    pub fn prune_wal_through(&self, inclusive_seq: u64) -> Result<()> {
        let keys: Vec<_> = self
            .wal
            .range(..=inclusive_seq.to_be_bytes())
            .keys()
            .collect::<Result<Vec<_>, _>>()?;
        for k in keys {
            self.wal.remove(k)?;
        }
        Ok(())
    }
}
