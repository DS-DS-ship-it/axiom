use std::{
    collections::BTreeMap,
    io::Write,
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use axiom_node::{
    crypto::{public_key_hex, sign_bytes, signing_key_from_hex},
    finality::{block_id, block_signing_bytes, vote_signing_bytes},
    network::{decode_message_line, encode_message, verify_message, NetworkMessage, MAX_MESSAGE_BYTES},
    state::{ChainState, Validator},
    types::{Block, BlockHeader, Vote},
};
use ed25519_dalek::SigningKey;

const BIN: &str = env!("CARGO_BIN_EXE_axiom-node");
const ROOT: &str = env!("CARGO_MANIFEST_DIR");

struct ChildGuard {
    name: String,
    child: Child,
}

impl ChildGuard {
    fn spawn(name: &str, config: &str) -> Self {
        let child = Command::new(BIN)
            .arg("--config")
            .arg(config)
            .current_dir(ROOT)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {name}: {e}"));

        Self {
            name: name.to_string(),
            child,
        }
    }

    fn assert_running(&mut self) {
        match self.child.try_wait().expect("try_wait failed") {
            None => {}
            Some(status) => panic!("{} exited unexpectedly: {}", self.name, status),
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assert_ports_available(ports: &[u16]) {
    let mut listeners = Vec::new();
    for port in ports {
        let listener = TcpListener::bind(("127.0.0.1", *port))
            .unwrap_or_else(|e| panic!("port {} is already in use before test start: {}", port, e));
        listeners.push(listener);
    }
    drop(listeners);
}

fn wait_for_port(port: u16, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("port {} did not become reachable in time", port);
}

fn send_line(addr: &str, line: &str) {
    let mut stream = TcpStream::connect(addr)
        .unwrap_or_else(|e| panic!("failed to connect to {}: {}", addr, e));
    stream
        .write_all(line.as_bytes())
        .unwrap_or_else(|e| panic!("failed to write line to {}: {}", addr, e));
    stream.write_all(b"\n").expect("failed to write newline");
    stream.flush().expect("flush failed");
}

fn temp_config_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("axiom-abuse-{}-{}", std::process::id(), nanos));
    std::fs::create_dir_all(&dir).expect("failed to create temp config dir");
    dir
}

fn write_test_configs(base_port: u16) -> Vec<String> {
    let dir = temp_config_dir();
    let mut out = Vec::new();
    let chain_id = "axiom-abuse-local";
    let genesis_path = format!("{}/../testnet/genesis/genesis.json", ROOT);

    for i in 0..4u16 {
        let port = base_port + i;
        let peers: Vec<String> = (0..4u16)
            .filter(|j| *j != i)
            .map(|j| format!("127.0.0.1:{}", base_port + j))
            .collect();

        let private_key_hex = match i {
            0 => "1111111111111111111111111111111111111111111111111111111111111111",
            1 => "2222222222222222222222222222222222222222222222222222222222222222",
            2 => "3333333333333333333333333333333333333333333333333333333333333333",
            3 => "4444444444444444444444444444444444444444444444444444444444444444",
            _ => unreachable!(),
        };

        let text = format!(
            r#"chain_id = "{}"
node_name = "node{}"
bind_addr = "127.0.0.1:{}"
p2p_peers = [{}]
private_key_hex = "{}"
validator_power = 100
genesis_path = "{}"
"#,
            chain_id,
            i + 1,
            port,
            peers
                .iter()
                .map(|p| format!(r#""{}""#, p))
                .collect::<Vec<_>>()
                .join(", "),
            private_key_hex,
            genesis_path,
        );

        let path = dir.join(format!("node{}.toml", i + 1));
        std::fs::write(&path, text).expect("failed to write temp config");
        out.push(path.to_string_lossy().to_string());
    }

    out
}

fn build_validator_state() -> (ChainState, BTreeMap<String, SigningKey>, String) {
    let mut state = ChainState {
        tip_hash: "GENESIS".to_string(),
        ..Default::default()
    };
    let mut keys = BTreeMap::new();

    let hexes = [
        ("node1", "1111111111111111111111111111111111111111111111111111111111111111"),
        ("node2", "2222222222222222222222222222222222222222222222222222222222222222"),
        ("node3", "3333333333333333333333333333333333333333333333333333333333333333"),
        ("node4", "4444444444444444444444444444444444444444444444444444444444444444"),
    ];

    for (name, hex) in hexes {
        let sk = signing_key_from_hex(hex).unwrap();
        state.validators.insert(
            name.to_string(),
            Validator {
                public_key: public_key_hex(&sk),
                power: 100,
                stake: 1000,
            },
        );
        keys.insert(name.to_string(), sk);
    }

    (state, keys, "axiom-abuse-local".to_string())
}

fn signed_block(chain_id: &str, proposer: &str, key: &SigningKey) -> Block {
    let mut block = Block {
        header: BlockHeader {
            chain_id: chain_id.to_string(),
            height: 1,
            parent_hash: "GENESIS".to_string(),
            proposer: proposer.to_string(),
            slot: 1,
            round: 0,
            timestamp_ms: 1_700_000_000_000,
            state_root: "state_root_1".to_string(),
            tx_root: "tx_root_1".to_string(),
            base_fee: 2,
            gas_used: 0,
        },
        txs: vec![],
        signature: String::new(),
    };

    block.signature = sign_bytes(key, &block_signing_bytes(&block).unwrap());
    block
}

fn signed_vote(chain_id: &str, block_hash: &str, validator: &str, key: &SigningKey) -> Vote {
    let mut vote = Vote {
        chain_id: chain_id.to_string(),
        block_hash: block_hash.to_string(),
        height: 1,
        round: 0,
        validator: validator.to_string(),
        signature: String::new(),
    };

    vote.signature = sign_bytes(key, &vote_signing_bytes(&vote).unwrap());
    vote
}

#[test]
fn abuse_limit_harness_survives_malformed_and_oversized_message_storm() {
    let base_port = 7201u16;
    let ports = [7201u16, 7202, 7203, 7204];
    assert_ports_available(&ports);

    let configs = write_test_configs(base_port);

    let mut node1 = ChildGuard::spawn("node1", &configs[0]);
    let mut node2 = ChildGuard::spawn("node2", &configs[1]);
    let mut node3 = ChildGuard::spawn("node3", &configs[2]);
    let mut node4 = ChildGuard::spawn("node4", &configs[3]);

    wait_for_port(7201, Duration::from_secs(10));
    wait_for_port(7202, Duration::from_secs(10));
    wait_for_port(7203, Duration::from_secs(10));
    wait_for_port(7204, Duration::from_secs(10));

    let (state, keys, chain_id) = build_validator_state();
    let block = signed_block(&chain_id, "node1", keys.get("node1").unwrap());
    let block_hash = block_id(&block).unwrap();

    let valid_messages = vec![
        NetworkMessage::Block(block.clone()),
        NetworkMessage::Vote(signed_vote(&chain_id, &block_hash, "node2", keys.get("node2").unwrap())),
        NetworkMessage::Vote(signed_vote(&chain_id, &block_hash, "node3", keys.get("node3").unwrap())),
        NetworkMessage::Vote(signed_vote(&chain_id, &block_hash, "node4", keys.get("node4").unwrap())),
    ];

    for msg in &valid_messages {
        let line = encode_message(msg).unwrap();
        let decoded = decode_message_line(&line).unwrap();
        verify_message(&decoded, &state).unwrap();
        send_line("127.0.0.1:7201", &line);
    }

    let malformed = r#"{"type":"Vote","payload":"not-a-real-vote"}"#;
    assert!(decode_message_line(malformed).is_err());
    for _ in 0..100 {
        send_line("127.0.0.1:7201", malformed);
    }

    let oversized = "x".repeat(MAX_MESSAGE_BYTES + 1);
    assert!(decode_message_line(&oversized).is_err());
    for _ in 0..20 {
        send_line("127.0.0.1:7201", &oversized);
    }

    let forged = NetworkMessage::Vote(signed_vote(&chain_id, &block_hash, "node2", keys.get("node4").unwrap()));
    let forged_line = encode_message(&forged).unwrap();
    let decoded_forged = decode_message_line(&forged_line).unwrap();
    assert!(verify_message(&decoded_forged, &state).is_err());
    for _ in 0..20 {
        send_line("127.0.0.1:7201", &forged_line);
    }

    thread::sleep(Duration::from_millis(500));

    node1.assert_running();
    node2.assert_running();
    node3.assert_running();
    node4.assert_running();
}
