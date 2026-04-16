use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};

use crate::{
    axiom1::Axiom1Genesis,
    checkpoint::{load_checkpoint_bundle, write_checkpoint_bundle},
    commit::{apply_commit_certificate, build_commit_certificate, CommitCertificate},
    config::NodeConfig,
    crypto::{sign_bytes, signing_key_from_hex},
    finality::{block_id, block_signing_bytes, vote_signing_bytes},
    network_auth::{authenticate_pair, verify_envelope, PeerAck, PeerHello, SessionInfo, SignedEnvelope},
    state::ChainState,
    state_transition::execute_block_deterministic,
    storage_engine::{PersistedCursor, StorageEngine},
    types::{Block, BlockHeader, Transaction, Vote},
};

#[derive(Debug)]
pub struct DevnetRuntime {
    pub config: NodeConfig,
    pub genesis: Axiom1Genesis,
    pub storage: StorageEngine,
    pub state: ChainState,
    pub cursor: PersistedCursor,
    pub data_dir: PathBuf,
    pub sessions: BTreeMap<String, SessionInfo>,
}

impl DevnetRuntime {
    pub fn bootstrap(config: NodeConfig, data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let storage = StorageEngine::open(data_dir.join("sled"))?;
        let genesis = Axiom1Genesis::from_path(&config.genesis_path)?;

        if config.chain_id != genesis.chain_id {
            return Err(anyhow!("config/genesis chain id mismatch"));
        }

        let checkpoint_root = data_dir.join("checkpoint_bundle");
        let mut cursor = storage.load_cursor()?;

        let mut state = if let Some((manifest, checkpoint_state)) =
            load_checkpoint_bundle(&checkpoint_root, &genesis.chain_id)?
        {
            cursor.checkpoint_id = Some(manifest.checkpoint_id);
            cursor.wal_entries_applied = manifest.wal_entries_applied;
            checkpoint_state
        } else if let Some(persisted_state) = storage.load_chain_state()? {
            persisted_state
        } else {
            let genesis_state = genesis.to_chain_state();
            storage.save_chain_state(&genesis_state)?;
            storage.save_cursor(&cursor)?;
            storage.flush()?;
            genesis_state
        };

        let wal_rows = storage.read_wal_from(cursor.wal_entries_applied.saturating_add(1))?;
        for (seq, cert) in wal_rows {
            apply_commit_certificate(&cert, &mut state)?;
            cursor.wal_entries_applied = seq;
        }

        storage.save_chain_state(&state)?;
        storage.save_cursor(&cursor)?;
        storage.flush()?;

        Ok(Self {
            config,
            genesis,
            storage,
            state,
            cursor,
            data_dir,
            sessions: BTreeMap::new(),
        })
    }

    pub fn authenticate_peer(&mut self, hello: &PeerHello, ack: &PeerAck) -> Result<SessionInfo> {
        let session = authenticate_pair(hello, ack, &self.config.chain_id)?;
        self.sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(session)
    }

    pub fn verify_peer_envelope(&self, session_id: &str, env: &SignedEnvelope) -> Result<()> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("unknown session"))?;
        verify_envelope(env, session, &self.state, &self.config.chain_id)
    }

    pub fn propose_block(&self, txs: Vec<Transaction>, timestamp_ms: u64) -> Result<Block> {
        let header = BlockHeader {
            chain_id: self.config.chain_id.clone(),
            height: self.state.height + 1,
            parent_hash: self.state.tip_hash.clone(),
            proposer: self.config.node_name.clone(),
            slot: self.state.height + 1,
            round: 0,
            timestamp_ms,
            state_root: String::new(),
            tx_root: String::new(),
            base_fee: self.genesis.base_fee,
            gas_used: 0,
        };

        let unsigned_block = Block {
            header,
            txs,
            signature: String::new(),
        };

        let execution =
            execute_block_deterministic(&unsigned_block, &self.state, &self.config.chain_id)?;

        let mut signed_block = Block {
            header: BlockHeader {
                state_root: execution.state_root,
                tx_root: execution.tx_root,
                gas_used: execution.gas_used,
                ..unsigned_block.header
            },
            txs: unsigned_block.txs,
            signature: String::new(),
        };

        let proposer_key = signing_key_from_hex(&self.config.private_key_hex)?;
        signed_block.signature = sign_bytes(&proposer_key, &block_signing_bytes(&signed_block)?);
        Ok(signed_block)
    }

    pub fn sign_vote_for_block(
        &self,
        validator_name: &str,
        validator_private_key_hex: &str,
        block: &Block,
    ) -> Result<Vote> {
        let mut vote = Vote {
            chain_id: self.config.chain_id.clone(),
            block_hash: block_id(block)?,
            height: block.header.height,
            round: block.header.round,
            validator: validator_name.to_string(),
            signature: String::new(),
        };
        let sk = signing_key_from_hex(validator_private_key_hex)?;
        vote.signature = sign_bytes(&sk, &vote_signing_bytes(&vote)?);
        Ok(vote)
    }

    pub fn commit_block(&mut self, block: &Block, votes: &[Vote]) -> Result<CommitCertificate> {
        let cert = build_commit_certificate(block, votes, &self.state)?;

        let execution = execute_block_deterministic(block, &self.state, &self.config.chain_id)?;

        if execution.state_root != block.header.state_root {
            return Err(anyhow!("block state root mismatch against deterministic execution"));
        }
        if execution.tx_root != block.header.tx_root {
            return Err(anyhow!("block tx root mismatch against deterministic execution"));
        }
        if execution.gas_used != block.header.gas_used {
            return Err(anyhow!("block gas_used mismatch against deterministic execution"));
        }

        let next_seq = self.cursor.wal_entries_applied.saturating_add(1);
        self.storage.append_wal_certificate(next_seq, &cert)?;

        self.state = execution.post_state;
        apply_commit_certificate(&cert, &mut self.state)?;
        self.cursor.wal_entries_applied = next_seq;

        let checkpoint_every = self.genesis.checkpoint_interval();
        if checkpoint_every > 0 && self.state.height > 0 && self.state.height.is_multiple_of(checkpoint_every)
        {
            let checkpoint_id = format!("h{:010}", self.state.height);
            self.storage.put_checkpoint(&checkpoint_id, &self.state)?;
            self.cursor.checkpoint_id = Some(checkpoint_id.clone());
            write_checkpoint_bundle(
                self.data_dir.join("checkpoint_bundle"),
                &self.genesis.chain_id,
                &checkpoint_id,
                &self.cursor,
                &self.state,
            )?;
            self.storage.prune_wal_through(self.cursor.wal_entries_applied)?;
        }

        self.storage.save_chain_state(&self.state)?;
        self.storage.save_cursor(&self.cursor)?;
        self.storage.flush()?;
        Ok(cert)
    }
}
