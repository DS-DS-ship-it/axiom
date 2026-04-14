use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpListener,
};
use tracing::{info, warn};

use crate::{
    finality::{verify_block_signature, verify_vote_signature},
    state::ChainState,
    types::{Block, Vote},
};

pub const MAX_MESSAGE_BYTES: usize = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum NetworkMessage {
    Block(Block),
    Vote(Vote),
}

pub fn encode_message(msg: &NetworkMessage) -> Result<String> {
    let line = serde_json::to_string(msg)?;
    ensure!(line.len() <= MAX_MESSAGE_BYTES, "oversized message");
    Ok(line)
}

pub fn decode_message_line(line: &str) -> Result<NetworkMessage> {
    ensure!(line.len() <= MAX_MESSAGE_BYTES, "oversized message");
    Ok(serde_json::from_str(line)?)
}

pub fn verify_message(msg: &NetworkMessage, state: &ChainState) -> Result<()> {
    match msg {
        NetworkMessage::Block(block) => verify_block_signature(block, state),
        NetworkMessage::Vote(vote) => verify_vote_signature(vote, state),
    }
}

pub async fn run_listener(bind_addr: &str) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, "p2p listener started");

    loop {
        let (stream, peer) = listener.accept().await?;
        info!(%peer, "peer connected");

        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.len() > MAX_MESSAGE_BYTES {
                    warn!(%peer, "oversized message dropped");
                    break;
                }

                match decode_message_line(&line) {
                    Ok(NetworkMessage::Block(block)) => {
                        info!(
                            %peer,
                            height = block.header.height,
                            proposer = %block.header.proposer,
                            "received block message"
                        );
                    }
                    Ok(NetworkMessage::Vote(vote)) => {
                        info!(
                            %peer,
                            height = vote.height,
                            round = vote.round,
                            validator = %vote.validator,
                            "received vote message"
                        );
                    }
                    Err(err) => {
                        warn!(%peer, error = %err, "invalid message dropped");
                    }
                }
            }
        });
    }
}
