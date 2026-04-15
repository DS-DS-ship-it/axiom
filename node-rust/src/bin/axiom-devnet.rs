use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use axiom_node::{
    config::NodeConfig,
    crypto::{public_key_hex, signing_key_from_hex},
    devnet_runtime::DevnetRuntime,
    network_auth::{
        now_ms, sign_ack, sign_envelope, sign_hello, PeerAck, PeerHello, SignedEnvelope,
        WIRE_PROTOCOL_VERSION,
    },
    types::Transaction,
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
    time::sleep,
};

type SharedRuntime = Arc<Mutex<DevnetRuntime>>;
type SharedMempool = Arc<Mutex<VecDeque<Transaction>>>;
type SharedSessions = Arc<Mutex<HashMap<String, String>>>;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    config: String,
    #[arg(long)]
    data_dir: String,
}

#[derive(Clone)]
struct AppState {
    runtime: SharedRuntime,
    mempool: SharedMempool,
    sessions: SharedSessions,
    node_name: String,
    checkpoint_interval: u64,
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

async fn status_handler(State(app): State<AppState>) -> impl IntoResponse {
    let rt = app.runtime.lock().await;
    let mempool = app.mempool.lock().await;
    let sessions = app.sessions.lock().await;
    let height = rt.state.height;
    let last_checkpoint_height = if app.checkpoint_interval == 0 {
        0
    } else {
        height - (height % app.checkpoint_interval)
    };

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
        axiom_node::validation::validate_transaction(&req.tx, &rt.state, &rt.config.chain_id)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }

    let mut mempool = app.mempool.lock().await;
    mempool.push_back(req.tx);

    Ok(Json(SubmitTxResponse {
        accepted: true,
        mempool_size: mempool.len(),
    }))
}

async fn handle_inbound(
    socket: TcpStream,
    runtime: SharedRuntime,
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

    let hello: PeerHello = serde_json::from_str(&first)
        .context("decoding inbound hello")?;

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

    writer
        .write_all(serde_json::to_string(&ack)?.as_bytes())
        .await?;
    writer.write_all(b"\n").await?;

    let session_id = {
        let mut rt = runtime.lock().await;
        let session = rt.authenticate_peer(&hello, &ack)?;
        session.session_id
    };

    sessions
        .lock()
        .await
        .insert(hello.node_name.clone(), session_id.clone());

    println!(
        "[{}] authenticated inbound peer {} ({:?}) session={}",
        cfg.node_name, hello.node_name, peer_addr, session_id
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

        let verify = {
            let rt = runtime.lock().await;
            rt.verify_peer_envelope(&session_id, &env)
        };

        if let Err(err) = verify {
            eprintln!("[{}] verify_peer_envelope failed: {err:#}", cfg.node_name);
            continue;
        }

        println!(
            "[{}] inbound msg_type={} from {}",
            cfg.node_name, env.msg_type, env.sender
        );
    }

    Ok(())
}

async fn dial_peer(
    peer_addr: String,
    runtime: SharedRuntime,
    sessions: SharedSessions,
    cfg: NodeConfig,
) -> Result<()> {
    let stream = TcpStream::connect(&peer_addr)
        .await
        .with_context(|| format!("connecting to {}", peer_addr))?;

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

    writer
        .write_all(serde_json::to_string(&hello)?.as_bytes())
        .await?;
    writer.write_all(b"\n").await?;

    let ack_line = lines
        .next_line()
        .await?
        .ok_or_else(|| anyhow!("peer closed before ack"))?;

    let ack: PeerAck = serde_json::from_str(&ack_line)
        .context("decoding inbound ack")?;

    let session_id = {
        let mut rt = runtime.lock().await;
        let session = rt.authenticate_peer(&hello, &ack)?;
        session.session_id
    };

    sessions
        .lock()
        .await
        .insert(ack.node_name.clone(), session_id.clone());

    let ping_env = sign_envelope(
        SignedEnvelope {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: cfg.chain_id.clone(),
            session_id,
            sender: cfg.node_name.clone(),
            msg_type: "ping".to_string(),
            payload: serde_json::json!({"ok": true}),
            timestamp_ms: now_ms(),
            signature: String::new(),
        },
        &cfg.private_key_hex,
    )?;

    writer
        .write_all(serde_json::to_string(&ping_env)?.as_bytes())
        .await?;
    writer.write_all(b"\n").await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = load_config(&args.config)?;
    let data_dir = PathBuf::from(&args.data_dir);

    let runtime = Arc::new(Mutex::new(
        DevnetRuntime::bootstrap(cfg.clone(), data_dir)
            .context("bootstrap runtime")?,
    ));
    let mempool: SharedMempool = Arc::new(Mutex::new(VecDeque::new()));
    let sessions: SharedSessions = Arc::new(Mutex::new(HashMap::new()));

    let status_addr = derive_status_addr(&cfg.bind_addr)?;
    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/tx", post(submit_tx_handler))
        .with_state(AppState {
            runtime: runtime.clone(),
            mempool: mempool.clone(),
            sessions: sessions.clone(),
            node_name: cfg.node_name.clone(),
            checkpoint_interval: 10,
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
    let accept_sessions = sessions.clone();
    let accept_cfg = cfg.clone();

    let accept_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _addr)) => {
                    let runtime = accept_runtime.clone();
                    let sessions = accept_sessions.clone();
                    let cfg = accept_cfg.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_inbound(socket, runtime, sessions, cfg).await {
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

    let dial_runtime = runtime.clone();
    let dial_sessions = sessions.clone();
    let dial_cfg = cfg.clone();

    let dial_task = tokio::spawn(async move {
        loop {
            for peer in dial_cfg.p2p_peers.clone() {
                if let Err(err) = dial_peer(
                    peer.clone(),
                    dial_runtime.clone(),
                    dial_sessions.clone(),
                    dial_cfg.clone(),
                )
                .await
                {
                    eprintln!("[{}] dial {} failed: {err:#}", dial_cfg.node_name, peer);
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

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
    dial_task.abort();
    heartbeat_task.abort();
    control_task.abort();

    Ok(())
}
