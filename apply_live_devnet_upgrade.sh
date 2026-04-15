#!/usr/bin/env bash
set -euo pipefail

cd /Users/derekwardlaw/Downloads/axiom_public_repo

mkdir -p node-rust/src node-rust/src/bin scripts testnet/devnet docs

python3 - <<'PY'
from pathlib import Path
import re

root = Path("/Users/derekwardlaw/Downloads/axiom_public_repo")

# ---------- helpers ----------
def ensure_dep(cargo_path: Path, section: str, line: str) -> None:
    text = cargo_path.read_text()
    if line in text:
        return
    lines = text.splitlines()
    out = []
    inserted = False
    i = 0
    while i < len(lines):
        out.append(lines[i])
        if lines[i].strip() == section and not inserted:
            j = i + 1
            block = []
            while j < len(lines) and not (lines[j].startswith("[") and lines[j].endswith("]")):
                block.append(lines[j])
                j += 1
            if line not in block:
                out.append(line)
            inserted = True
        i += 1
    if not inserted:
        out.append(section)
        out.append(line)
    cargo_path.write_text("\n".join(out) + "\n")

def ensure_line(path: Path, line: str) -> None:
    text = path.read_text() if path.exists() else ""
    if line not in text:
        if text and not text.endswith("\n"):
            text += "\n"
        text += line + "\n"
        path.write_text(text)

# ---------- Cargo.toml ----------
cargo = root / "node-rust" / "Cargo.toml"
ensure_dep(cargo, "[dependencies]", 'axum = "0.7"')
ensure_dep(cargo, "[dependencies]", 'clap = { version = "4", features = ["derive"] }')
ensure_dep(cargo, "[dependencies]", 'serde = { version = "1", features = ["derive"] }')
ensure_dep(cargo, "[dependencies]", 'serde_json = "1"')
ensure_dep(cargo, "[dependencies]", 'tokio = { version = "1", features = ["full"] }')
ensure_dep(cargo, "[dependencies]", 'toml = "0.8"')

# ---------- lib.rs export ----------
lib = root / "node-rust" / "src" / "lib.rs"
ensure_line(lib, "pub mod devnet_wire;")

# ---------- .gitignore ----------
gitignore = root / ".gitignore"
for line in [
    "",
    "# AXIOM local runtime artifacts",
    "*.bak",
    "testnet/devnet/node*-data/",
    "testnet/devnet/*.wal.jsonl",
    "testnet/devnet/*-state.json",
]:
    ensure_line(gitignore, line)

# ---------- devnet wire module ----------
(root / "node-rust" / "src" / "devnet_wire.rs").write_text(
'''use serde::{Deserialize, Serialize};

use crate::types::{Block, Transaction, Vote};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DevnetMessage {
    GossipTx {
        tx: Transaction,
    },
    Proposal {
        block: Block,
        proposer: String,
    },
    Vote {
        block_hash: String,
        vote: Vote,
        proposer: String,
    },
    Commit {
        block: Block,
        votes: Vec<Vote>,
    },
    Ping,
}
'''
)

# ---------- overwrite generator with absolute-path output ----------
(root / "scripts" / "generate_axiom1_genesis.py").write_text(
r'''#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

KEYS = [
    ("node1", "1111111111111111111111111111111111111111111111111111111111111111", 7401),
    ("node2", "2222222222222222222222222222222222222222222222222222222222222222", 7402),
    ("node3", "3333333333333333333333333333333333333333333333333333333333333333", 7403),
    ("node4", "4444444444444444444444444444444444444444444444444444444444444444", 7404),
]

PUBKEYS = {
    "node1": "d04ab232742bb4ab3a1368bd4615e4e6d0224a7d61e5b1c5b8c9e7c2e7a6d36e",
    "node2": "a09aa5f47a6759802ff955f8dc2d2a14a5c99d23be97f864127ff9383455a4f0",
    "node3": "0b4c8667b9f9be1c0a4903f3fbd8f0ac7d44f084f0bea0d27968b6c6dd8c8f68",
    "node4": "517c2a5a1fce7b9a34a3b2d1fcb5f2b489fbf4d7d2a2f1b18a6dd1f4c8f9f1b0",
}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    genesis_path = (out_dir / "axiom-1-genesis.json").resolve()

    genesis = {
        "chain_id": "axiom-1",
        "protocol_version": 1,
        "genesis_time_ms": 1700000000000,
        "base_fee": 2,
        "checkpoint_interval": 10,
        "validators": [
            {
                "name": name,
                "public_key": PUBKEYS[name],
                "bind_addr": f"127.0.0.1:{port}",
                "power": 100,
                "stake": 1000000,
            }
            for (name, _key, port) in KEYS
        ],
        "accounts": [
            {
                "address": "axm_devnet_faucet",
                "balance": 1000000000,
                "nonce": 0,
                "staked": 0,
            }
        ],
        "metadata": {
            "network": "devnet",
            "asset": "AXM",
        },
    }

    genesis_path.write_text(json.dumps(genesis, indent=2) + "\n")

    peer_map = {
        "node1": ["127.0.0.1:7402", "127.0.0.1:7403", "127.0.0.1:7404"],
        "node2": ["127.0.0.1:7401", "127.0.0.1:7403", "127.0.0.1:7404"],
        "node3": ["127.0.0.1:7401", "127.0.0.1:7402", "127.0.0.1:7404"],
        "node4": ["127.0.0.1:7401", "127.0.0.1:7402", "127.0.0.1:7403"],
    }

    for (name, key_hex, port) in KEYS:
        state_path = (out_dir / f"{name}-state.json").resolve().as_posix()
        wal_path = (out_dir / f"{name}.wal.jsonl").resolve().as_posix()
        bind_addr = f"127.0.0.1:{port}"
        peers = ", ".join([f'"{p}"' for p in peer_map[name]])

        text = f'''chain_id = "axiom-1"
node_name = "{name}"
bind_addr = "{bind_addr}"
p2p_peers = [{peers}]
private_key_hex = "{key_hex}"
validator_power = 100
genesis_path = "{genesis_path.as_posix()}"
state_path = "{state_path}"
wal_path = "{wal_path}"
'''
        (out_dir / f"{name}.toml").write_text(text)

    print(f"wrote {genesis_path}")
    print("wrote node1.toml through node4.toml")

if __name__ == "__main__":
    main()
'''
)

# ---------- overwrite persistent devnet daemon ----------
(root / "node-rust" / "src" / "bin" / "axiom-devnet.rs").write_text(
'''use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use axiom_node::{
    config::NodeConfig,
    crypto::{public_key_hex, sign_bytes, signing_key_from_hex},
    devnet_runtime::DevnetRuntime,
    devnet_wire::DevnetMessage,
    finality::block_id,
    network_auth::{
        now_ms, sign_ack, sign_envelope, sign_hello, PeerAck, PeerHello, SignedEnvelope,
        WIRE_PROTOCOL_VERSION,
    },
    state::Validator,
    types::{Block, Transaction, Vote},
    validation::signing_bytes,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time::{sleep, timeout},
};

type SharedRuntime = Arc<Mutex<DevnetRuntime>>;
type SharedMempool = Arc<Mutex<VecDeque<Transaction>>>;
type SharedSessions = Arc<Mutex<HashMap<String, String>>>;
type SharedPending = Arc<Mutex<HashMap<String, PendingBlock>>>;

#[derive(Clone)]
struct AppState {
    runtime: SharedRuntime,
    mempool: SharedMempool,
    sessions: SharedSessions,
    node_name: String,
    checkpoint_interval: u64,
}

#[derive(Clone)]
struct PendingBlock {
    block: Block,
    votes: Vec<Vote>,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    config: String,
    #[arg(long)]
    data_dir: String,
}

#[derive(Serialize)]
struct StatusView {
    node_name: String,
    height: u64,
    tip_hash: String,
    connected_peer_count: usize,
    last_checkpoint_height: u64,
    mempool_size: usize,
}

#[derive(Deserialize)]
struct SubmitTxRequest {
    tx: Transaction,
}

#[derive(Serialize)]
struct SubmitTxResponse {
    accepted: bool,
    mempool_size: usize,
}

fn load_config(path: &str) -> Result<NodeConfig> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path))?;
    let cfg: NodeConfig = toml::from_str(&text)
        .with_context(|| format!("parsing config {}", path))?;
    Ok(cfg)
}

fn derive_status_addr(bind_addr: &str) -> Result<String> {
    let mut parts = bind_addr.rsplitn(2, ':');
    let port = parts
        .next()
        .ok_or_else(|| anyhow!("missing port in bind_addr"))?
        .parse::<u16>()?;
    let host = parts
        .next()
        .ok_or_else(|| anyhow!("missing host in bind_addr"))?;
    Ok(format!("{host}:{}", port + 1000))
}

fn current_proposer(validators: &std::collections::BTreeMap<String, Validator>, next_height: u64) -> Option<String> {
    let mut vals = validators.values().cloned().collect::<Vec<_>>();
    vals.sort_by(|a, b| a.name.cmp(&b.name));
    if vals.is_empty() {
        return None;
    }
    let idx = ((next_height.saturating_sub(1)) as usize) % vals.len();
    Some(vals[idx].name.clone())
}

fn validator_bind_addr(
    validators: &std::collections::BTreeMap<String, Validator>,
    name: &str,
) -> Option<String> {
    validators.get(name).map(|v| v.bind_addr.clone())
}

async fn status_handler(State(app): State<AppState>) -> impl IntoResponse {
    let rt = app.runtime.lock().await;
    let mempool = app.mempool.lock().await;
    let sessions = app.sessions.lock().await;
    let height = rt.state.height;
    let last_checkpoint_height = height - (height % app.checkpoint_interval.max(1));

    Json(StatusView {
        node_name: app.node_name.clone(),
        height,
        tip_hash: rt.state.tip_hash.clone(),
        connected_peer_count: sessions.len(),
        last_checkpoint_height,
        mempool_size: mempool.len(),
    })
}

async fn submit_tx_handler(
    State(app): State<AppState>,
    Json(req): Json<SubmitTxRequest>,
) -> Result<Json<SubmitTxResponse>, (StatusCode, String)> {
    {
        let rt = app.runtime.lock().await;
        axiom_node::validation::validate_transaction(&rt.state, &req.tx)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }

    {
        let mut mempool = app.mempool.lock().await;
        mempool.push_back(req.tx.clone());
    }

    let peers = {
        let rt = app.runtime.lock().await;
        rt.config.p2p_peers.clone()
    };

    let cfg = {
        let rt = app.runtime.lock().await;
        rt.config.clone()
    };

    for peer in peers {
        let _ = send_wire_message(
            peer,
            cfg.clone(),
            app.runtime.clone(),
            app.sessions.clone(),
            DevnetMessage::GossipTx { tx: req.tx.clone() },
        ).await;
    }

    let mempool_size = app.mempool.lock().await.len();
    Ok(Json(SubmitTxResponse {
        accepted: true,
        mempool_size,
    }))
}

async fn handshake_outbound(
    stream: TcpStream,
    runtime: SharedRuntime,
    sessions: SharedSessions,
    cfg: NodeConfig,
) -> Result<(tokio::net::tcp::OwnedWriteHalf, String)> {
    let (reader_half, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader_half).lines();

    let sk = signing_key_from_hex(&cfg.private_key_hex)?;
    let hello = sign_hello(
        PeerHello {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: cfg.chain_id.clone(),
            node_name: cfg.node_name.clone(),
            bind_addr: cfg.bind_addr.clone(),
            public_key: public_key_hex(&sk),
            challenge: format!("hello-{}-{}", cfg.node_name, now_ms()),
            timestamp_ms: now_ms(),
            signature: String::new(),
        },
        &cfg.private_key_hex,
    )?;

    writer.write_all(serde_json::to_string(&hello)?.as_bytes()).await?;
    writer.write_all(b"\\n").await?;

    let ack_line = timeout(Duration::from_secs(5), lines.next_line())
        .await
        .context("timed out waiting for ack")?? 
        .ok_or_else(|| anyhow!("peer closed before ack"))?;

    let ack: PeerAck = serde_json::from_str(&ack_line).context("decoding inbound ack")?;
    let session = {
        let mut rt = runtime.lock().await;
        rt.authenticate_peer(&hello, &ack)?
    };

    sessions.lock().await.insert(ack.node_name.clone(), session.session_id.clone());
    Ok((writer, session.session_id))
}

async fn send_wire_message(
    peer_addr: String,
    cfg: NodeConfig,
    runtime: SharedRuntime,
    sessions: SharedSessions,
    msg: DevnetMessage,
) -> Result<()> {
    let stream = TcpStream::connect(&peer_addr)
        .await
        .with_context(|| format!("connecting to {}", peer_addr))?;

    let (mut writer, session_id) = handshake_outbound(stream, runtime, sessions, cfg.clone()).await?;
    let payload = serde_json::to_value(msg)?;
    let env = sign_envelope(
        SignedEnvelope {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: cfg.chain_id.clone(),
            session_id,
            sender: cfg.node_name.clone(),
            msg_type: "devnet".to_string(),
            payload,
            timestamp_ms: now_ms(),
            signature: String::new(),
        },
        &cfg.private_key_hex,
    )?;

    writer.write_all(serde_json::to_string(&env)?.as_bytes()).await?;
    writer.write_all(b"\\n").await?;
    Ok(())
}

async fn maybe_commit_pending(
    runtime: SharedRuntime,
    pending: SharedPending,
    cfg: NodeConfig,
) -> Result<Option<(Block, Vec<Vote>)>> {
    let mut to_broadcast = None;

    let keys = {
        let map = pending.lock().await;
        map.keys().cloned().collect::<Vec<_>>()
    };

    for key in keys {
        let maybe = {
            let map = pending.lock().await;
            map.get(&key).cloned()
        };

        let Some(pb) = maybe else { continue; };

        let commit_result = {
            let mut rt = runtime.lock().await;
            rt.commit_block(&pb.block, &pb.votes)
        };

        if commit_result.is_ok() {
            pending.lock().await.remove(&key);
            to_broadcast = Some((pb.block, pb.votes));
            break;
        }
    }

    Ok(to_broadcast)
}

async fn handle_devnet_message(
    env: SignedEnvelope,
    runtime: SharedRuntime,
    mempool: SharedMempool,
    pending: SharedPending,
    sessions: SharedSessions,
    cfg: NodeConfig,
    session_id: String,
) -> Result<()> {
    {
        let rt = runtime.lock().await;
        rt.verify_peer_envelope(&session_id, &env)?;
    }

    let msg: DevnetMessage = serde_json::from_value(env.payload)?;

    match msg {
        DevnetMessage::GossipTx { tx } => {
            let valid = {
                let rt = runtime.lock().await;
                axiom_node::validation::validate_transaction(&rt.state, &tx).is_ok()
            };
            if valid {
                mempool.lock().await.push_back(tx);
            }
        }
        DevnetMessage::Proposal { block, proposer } => {
            let proposer_addr = {
                let rt = runtime.lock().await;
                validator_bind_addr(&rt.state.validators, &proposer)
            };

            let Some(proposer_addr) = proposer_addr else {
                return Ok(());
            };

            let vote = {
                let rt = runtime.lock().await;
                rt.sign_vote_for_block(&cfg.node_name, &cfg.private_key_hex, &block)?
            };

            let block_hash = block_id(&block);
            let vote_msg = DevnetMessage::Vote {
                block_hash,
                vote,
                proposer,
            };

            let _ = send_wire_message(
                proposer_addr,
                cfg.clone(),
                runtime.clone(),
                sessions.clone(),
                vote_msg,
            ).await;
        }
        DevnetMessage::Vote {
            block_hash,
            vote,
            proposer,
        } => {
            if proposer != cfg.node_name {
                return Ok(());
            }

            {
                let mut map = pending.lock().await;
                if let Some(entry) = map.get_mut(&block_hash) {
                    entry.votes.push(vote);
                }
            }

            if let Some((block, votes)) = maybe_commit_pending(runtime.clone(), pending.clone(), cfg.clone()).await? {
                let peers = {
                    let rt = runtime.lock().await;
                    rt.config.p2p_peers.clone()
                };
                for peer in peers {
                    let _ = send_wire_message(
                        peer,
                        cfg.clone(),
                        runtime.clone(),
                        sessions.clone(),
                        DevnetMessage::Commit {
                            block: block.clone(),
                            votes: votes.clone(),
                        },
                    ).await;
                }
            }
        }
        DevnetMessage::Commit { block, votes } => {
            let mut rt = runtime.lock().await;
            if rt.state.height < block.height {
                let _ = rt.commit_block(&block, &votes)?;
            }
        }
        DevnetMessage::Ping => {}
    }

    Ok(())
}

async fn handle_inbound(
    socket: TcpStream,
    runtime: SharedRuntime,
    mempool: SharedMempool,
    pending: SharedPending,
    sessions: SharedSessions,
    cfg: NodeConfig,
) -> Result<()> {
    let peer_addr = socket.peer_addr().ok();
    let (reader_half, mut writer) = socket.into_split();
    let mut lines = BufReader::new(reader_half).lines();

    let first = lines
        .next_line()
        .await?
        .ok_or_else(|| anyhow!("peer closed before hello"))?;

    let hello: PeerHello = serde_json::from_str(&first).context("decoding inbound hello")?;

    let sk = signing_key_from_hex(&cfg.private_key_hex)?;
    let ack = sign_ack(
        PeerAck {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: cfg.chain_id.clone(),
            node_name: cfg.node_name.clone(),
            bind_addr: cfg.bind_addr.clone(),
            public_key: public_key_hex(&sk),
            peer_challenge: hello.challenge.clone(),
            own_challenge: format!("ack-{}-{}", cfg.node_name, now_ms()),
            timestamp_ms: now_ms(),
            signature: String::new(),
        },
        &cfg.private_key_hex,
    )?;

    writer.write_all(serde_json::to_string(&ack)?.as_bytes()).await?;
    writer.write_all(b"\\n").await?;

    let session = {
        let mut rt = runtime.lock().await;
        rt.authenticate_peer(&hello, &ack)?
    };
    sessions.lock().await.insert(hello.node_name.clone(), session.session_id.clone());

    println!(
        "[{}] authenticated inbound peer {} ({:?}) session={}",
        cfg.node_name, hello.node_name, peer_addr, session.session_id
    );

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let env: SignedEnvelope = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("[{}] bad envelope: {err:#}", cfg.node_name);
                continue;
            }
        };

        if let Err(err) = handle_devnet_message(
            env,
            runtime.clone(),
            mempool.clone(),
            pending.clone(),
            sessions.clone(),
            cfg.clone(),
            session.session_id.clone(),
        ).await {
            eprintln!("[{}] devnet message error: {err:#}", cfg.node_name);
        }
    }

    Ok(())
}

async fn producer_loop(
    runtime: SharedRuntime,
    mempool: SharedMempool,
    pending: SharedPending,
    sessions: SharedSessions,
    cfg: NodeConfig,
) -> Result<()> {
    loop {
        sleep(Duration::from_secs(2)).await;

        let maybe_block = {
            let proposer_ok;
            {
                let rt = runtime.lock().await;
                proposer_ok = current_proposer(&rt.state.validators, rt.state.height + 1)
                    .as_deref()
                    == Some(&cfg.node_name);
            }

            if !proposer_ok {
                None
            } else {
                let txs = {
                    let mut mp = mempool.lock().await;
                    let n = mp.len().min(100);
                    mp.drain(..n).collect::<Vec<_>>()
                };

                if txs.is_empty() {
                    None
                } else {
                    let mut rt = runtime.lock().await;
                    match rt.propose_block(txs, now_ms()) {
                        Ok(block) => Some(block),
                        Err(err) => {
                            eprintln!("[{}] propose_block failed: {err:#}", cfg.node_name);
                            None
                        }
                    }
                }
            }
        };

        let Some(block) = maybe_block else { continue; };

        let local_vote = {
            let rt = runtime.lock().await;
            rt.sign_vote_for_block(&cfg.node_name, &cfg.private_key_hex, &block)?
        };

        let hash = block_id(&block);
        pending.lock().await.insert(
            hash.clone(),
            PendingBlock {
                block: block.clone(),
                votes: vec![local_vote],
            },
        );

        println!("[{}] proposed block height={} hash={}", cfg.node_name, block.height, hash);

        let peers = {
            let rt = runtime.lock().await;
            rt.config.p2p_peers.clone()
        };

        for peer in peers {
            let _ = send_wire_message(
                peer,
                cfg.clone(),
                runtime.clone(),
                sessions.clone(),
                DevnetMessage::Proposal {
                    block: block.clone(),
                    proposer: cfg.node_name.clone(),
                },
            ).await;
        }

        if let Some((block, votes)) = maybe_commit_pending(runtime.clone(), pending.clone(), cfg.clone()).await? {
            let peers = {
                let rt = runtime.lock().await;
                rt.config.p2p_peers.clone()
            };
            for peer in peers {
                let _ = send_wire_message(
                    peer,
                    cfg.clone(),
                    runtime.clone(),
                    sessions.clone(),
                    DevnetMessage::Commit {
                        block: block.clone(),
                        votes: votes.clone(),
                    },
                ).await;
            }
        }
    }
}

fn funded_dev_tx(
    sender_key_hex: &str,
    sender: &str,
    recipient: &str,
    nonce: u64,
    value: u64,
) -> Result<Transaction> {
    let sk = signing_key_from_hex(sender_key_hex)?;
    let mut tx = Transaction {
        chain_id: "axiom-1".to_string(),
        kind: "transfer".to_string(),
        sender: sender.to_string(),
        sender_pubkey: public_key_hex(&sk),
        nonce,
        gas_limit: 10,
        max_fee_per_gas: 3,
        value,
        recipient: Some(recipient.to_string()),
        data: None,
        timestamp_ms: now_ms(),
        signature: String::new(),
    };
    tx.signature = sign_bytes(&sk, &signing_bytes(&tx)?) ;
    Ok(tx)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = load_config(&args.config)?;
    let data_dir = PathBuf::from(&args.data_dir);

    let runtime = Arc::new(Mutex::new(
        DevnetRuntime::bootstrap(cfg.clone(), data_dir).context("bootstrap runtime")?,
    ));
    let mempool: SharedMempool = Arc::new(Mutex::new(VecDeque::new()));
    let sessions: SharedSessions = Arc::new(Mutex::new(HashMap::new()));
    let pending: SharedPending = Arc::new(Mutex::new(HashMap::new()));
    let checkpoint_interval = 10_u64;

    let status_addr = derive_status_addr(&cfg.bind_addr)?;
    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/tx", post(submit_tx_handler))
        .with_state(AppState {
            runtime: runtime.clone(),
            mempool: mempool.clone(),
            sessions: sessions.clone(),
            node_name: cfg.node_name.clone(),
            checkpoint_interval,
        });

    let control_listener = TcpListener::bind(&status_addr)
        .await
        .with_context(|| format!("binding status {}", status_addr))?;
    let control_task = tokio::spawn(async move {
        if let Err(err) = axum::serve(control_listener, app).await {
            eprintln!("control plane failed: {err:#}");
        }
    });

    let listener = TcpListener::bind(&cfg.bind_addr)
        .await
        .with_context(|| format!("binding {}", cfg.bind_addr))?;

    println!("[{}] axiom-devnet listening on {}", cfg.node_name, cfg.bind_addr);
    println!("[{}] status/tx server on {}", cfg.node_name, status_addr);

    let accept_runtime = runtime.clone();
    let accept_mempool = mempool.clone();
    let accept_pending = pending.clone();
    let accept_sessions = sessions.clone();
    let accept_cfg = cfg.clone();

    let accept_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _addr)) => {
                    let runtime = accept_runtime.clone();
                    let mempool = accept_mempool.clone();
                    let pending = accept_pending.clone();
                    let sessions = accept_sessions.clone();
                    let cfg = accept_cfg.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_inbound(
                            socket,
                            runtime,
                            mempool,
                            pending,
                            sessions,
                            cfg,
                        ).await {
                            eprintln!("inbound session error: {err:#}");
                        }
                    });
                }
                Err(err) => {
                    eprintln!("accept error: {err:#}");
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    let producer_task = tokio::spawn(producer_loop(
        runtime.clone(),
        mempool.clone(),
        pending.clone(),
        sessions.clone(),
        cfg.clone(),
    ));

    let heartbeat_runtime = runtime.clone();
    let heartbeat_sessions = sessions.clone();
    let heartbeat_mempool = mempool.clone();
    let heartbeat_name = cfg.node_name.clone();
    let heartbeat_task = tokio::spawn(async move {
        loop {
            let (height, tip) = {
                let rt = heartbeat_runtime.lock().await;
                (rt.state.height, rt.state.tip_hash.clone())
            };
            let peers = heartbeat_sessions.lock().await.len();
            let mempool_size = heartbeat_mempool.lock().await.len();
            println!(
                "[{}] heartbeat height={} tip={} peers={} mempool={}",
                heartbeat_name, height, tip, peers, mempool_size
            );
            sleep(Duration::from_secs(10)).await;
        }
    });

    tokio::signal::ctrl_c().await?;
    println!("[{}] shutting down", cfg.node_name);

    accept_task.abort();
    producer_task.abort();
    heartbeat_task.abort();
    control_task.abort();

    Ok(())
}
'''
)

# ---------- helper startup scripts ----------
for i in range(1, 5):
    (root / "testnet" / "devnet" / f"start-node{i}.sh").write_text(
f'''#!/usr/bin/env bash
set -euo pipefail
cd /Users/derekwardlaw/Downloads/axiom_public_repo
cargo run --manifest-path node-rust/Cargo.toml --bin axiom-devnet -- \\
  --config testnet/devnet/node{i}.toml \\
  --data-dir testnet/devnet/node{i}-data
'''
    )
    (root / "testnet" / "devnet" / f"start-node{i}.sh").chmod(0o755)

(root / "testnet" / "devnet" / "reset-devnet.sh").write_text(
'''#!/usr/bin/env bash
set -euo pipefail
cd /Users/derekwardlaw/Downloads/axiom_public_repo
rm -rf testnet/devnet/node1-data testnet/devnet/node2-data testnet/devnet/node3-data testnet/devnet/node4-data
rm -f testnet/devnet/node1-state.json testnet/devnet/node2-state.json testnet/devnet/node3-state.json testnet/devnet/node4-state.json
rm -f testnet/devnet/node1.wal.jsonl testnet/devnet/node2.wal.jsonl testnet/devnet/node3.wal.jsonl testnet/devnet/node4.wal.jsonl
python3 scripts/generate_axiom1_genesis.py --out testnet/devnet
echo "devnet reset complete"
'''
)
(root / "testnet" / "devnet" / "reset-devnet.sh").chmod(0o755)

# ---------- docs ----------
(root / "docs" / "startup-guide.md").write_text(
'''# AXIOM Devnet Startup Guide

1. Reset the devnet:
   `testnet/devnet/reset-devnet.sh`

2. Start node1 through node4 in separate terminals:
   `testnet/devnet/start-node1.sh`
   `testnet/devnet/start-node2.sh`
   `testnet/devnet/start-node3.sh`
   `testnet/devnet/start-node4.sh`

3. Check live listeners:
   `lsof -nP -iTCP:7401 -iTCP:7402 -iTCP:7403 -iTCP:7404 | grep LISTEN`

4. Check node status:
   `curl http://127.0.0.1:8401/status`
'''
)

(root / "docs" / "operator-guide.md").write_text(
'''# AXIOM Operator Guide

- Node ports: 7401-7404
- Status ports: 8401-8404
- Configs: `testnet/devnet/node1.toml` through `node4.toml`
- State and WAL paths are generated as absolute paths
- Use `reset-devnet.sh` to wipe local runtime data and regenerate genesis
'''
)

(root / "docs" / "tx-injection-guide.md").write_text(
'''# AXIOM Transaction Injection Guide

Submit a signed transaction to a node:

```bash
curl -X POST http://127.0.0.1:8401/tx \\
  -H 'content-type: application/json' \\
  -d '{"tx": { ... signed transaction json ... }}'
