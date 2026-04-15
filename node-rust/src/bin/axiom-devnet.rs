use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use axiom_node::{
    config::NodeConfig,
    crypto::{public_key_hex, signing_key_from_hex},
    devnet_runtime::DevnetRuntime,
    network_auth::{
        now_ms, sign_ack, sign_hello, PeerAck, PeerHello, WIRE_PROTOCOL_VERSION,
    },
};
use clap::Parser;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time::{sleep, timeout},
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    config: String,
    #[arg(long)]
    data_dir: String,
}

type SharedRuntime = Arc<Mutex<DevnetRuntime>>;

fn load_config(path: &str) -> Result<NodeConfig> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path))?;
    let cfg: NodeConfig = toml::from_str(&text)
        .with_context(|| format!("parsing config {}", path))?;
    Ok(cfg)
}

async fn handle_inbound(
    socket: TcpStream,
    runtime: SharedRuntime,
    cfg: NodeConfig,
    key_hex: String,
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

    let sk = signing_key_from_hex(&key_hex)?;
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
        &key_hex,
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

    println!(
        "[{}] authenticated inbound peer {} ({:?}) session={}",
        cfg.node_name, hello.node_name, peer_addr, session_id
    );

    while let Some(line) = lines.next_line().await? {
        if !line.trim().is_empty() {
            println!(
                "[{}] inbound payload from {}: {}",
                cfg.node_name, hello.node_name, line
            );
        }
    }

    Ok(())
}

async fn dial_peer(
    peer_addr: String,
    runtime: SharedRuntime,
    cfg: NodeConfig,
    key_hex: String,
) -> Result<()> {
    let stream = TcpStream::connect(&peer_addr)
        .await
        .with_context(|| format!("connecting to {}", peer_addr))?;

    let (reader_half, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader_half).lines();

    let sk = signing_key_from_hex(&key_hex)?;
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
        &key_hex,
    )?;

    writer
        .write_all(serde_json::to_string(&hello)?.as_bytes())
        .await?;
    writer.write_all(b"\n").await?;

    let ack_line = timeout(Duration::from_secs(5), lines.next_line())
        .await
        .context("timed out waiting for ack")?? 
        .ok_or_else(|| anyhow!("peer closed before ack"))?;

    let ack: PeerAck = serde_json::from_str(&ack_line)
        .context("decoding inbound ack")?;

    let session_id = {
        let mut rt = runtime.lock().await;
        let session = rt.authenticate_peer(&hello, &ack)?;
        session.session_id
    };

    println!(
        "[{}] authenticated outbound peer {} session={}",
        cfg.node_name, peer_addr, session_id
    );

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

    let listener = TcpListener::bind(&cfg.bind_addr)
        .await
        .with_context(|| format!("binding {}", cfg.bind_addr))?;

    println!(
        "[{}] axiom-devnet listening on {}",
        cfg.node_name, cfg.bind_addr
    );

    let accept_runtime = runtime.clone();
    let accept_cfg = cfg.clone();
    let accept_key = cfg.private_key_hex.clone();

    let accept_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _addr)) => {
                    let runtime = accept_runtime.clone();
                    let cfg = accept_cfg.clone();
                    let key_hex = accept_key.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_inbound(socket, runtime, cfg, key_hex).await {
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
    let dial_cfg = cfg.clone();
    let dial_key = cfg.private_key_hex.clone();

    let dial_task = tokio::spawn(async move {
        loop {
            for peer in dial_cfg.p2p_peers.clone() {
                if let Err(err) = dial_peer(
                    peer.clone(),
                    dial_runtime.clone(),
                    dial_cfg.clone(),
                    dial_key.clone(),
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
    let heartbeat_name = cfg.node_name.clone();
    let heartbeat_task = tokio::spawn(async move {
        loop {
            let (height, tip) = {
                let rt = heartbeat_runtime.lock().await;
                (rt.state.height, rt.state.tip_hash.clone())
            };
            println!(
                "[{}] heartbeat height={} tip={}",
                heartbeat_name, height, tip
            );
            sleep(Duration::from_secs(10)).await;
        }
    });

    tokio::signal::ctrl_c().await?;
    println!("[{}] shutting down", cfg.node_name);

    accept_task.abort();
    dial_task.abort();
    heartbeat_task.abort();

    Ok(())
}
