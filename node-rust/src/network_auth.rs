use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    crypto::{address_from_vk, sign_bytes, signing_key_from_hex, verify_bytes},
    state::ChainState,
};

pub const WIRE_PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 1_000_000;
pub const MAX_CLOCK_SKEW_MS: u64 = 120_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerHello {
    pub version: u16,
    pub chain_id: String,
    pub node_name: String,
    pub bind_addr: String,
    pub public_key: String,
    pub challenge: String,
    pub timestamp_ms: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAck {
    pub version: u16,
    pub chain_id: String,
    pub node_name: String,
    pub bind_addr: String,
    pub public_key: String,
    pub peer_challenge: String,
    pub own_challenge: String,
    pub timestamp_ms: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfo {
    pub remote_node_name: String,
    pub remote_address: String,
    pub remote_public_key: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedEnvelope {
    pub version: u16,
    pub chain_id: String,
    pub session_id: String,
    pub sender: String,
    pub msg_type: String,
    pub payload: serde_json::Value,
    pub timestamp_ms: u64,
    pub signature: String,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn bounded_size(bytes: &[u8]) -> Result<()> {
    ensure!(bytes.len() <= MAX_FRAME_BYTES, "oversized frame");
    Ok(())
}

pub fn hello_signing_bytes(hello: &PeerHello) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(&json!({
        "version": hello.version,
        "chain_id": hello.chain_id,
        "node_name": hello.node_name,
        "bind_addr": hello.bind_addr,
        "public_key": hello.public_key,
        "challenge": hello.challenge,
        "timestamp_ms": hello.timestamp_ms,
    }))?;
    bounded_size(&bytes)?;
    Ok(bytes)
}

pub fn ack_signing_bytes(ack: &PeerAck) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(&json!({
        "version": ack.version,
        "chain_id": ack.chain_id,
        "node_name": ack.node_name,
        "bind_addr": ack.bind_addr,
        "public_key": ack.public_key,
        "peer_challenge": ack.peer_challenge,
        "own_challenge": ack.own_challenge,
        "timestamp_ms": ack.timestamp_ms,
    }))?;
    bounded_size(&bytes)?;
    Ok(bytes)
}

pub fn envelope_signing_bytes(env: &SignedEnvelope) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(&json!({
        "version": env.version,
        "chain_id": env.chain_id,
        "session_id": env.session_id,
        "sender": env.sender,
        "msg_type": env.msg_type,
        "payload": env.payload,
        "timestamp_ms": env.timestamp_ms,
    }))?;
    bounded_size(&bytes)?;
    Ok(bytes)
}

pub fn sign_hello(mut hello: PeerHello, private_key_hex: &str) -> Result<PeerHello> {
    let sk = signing_key_from_hex(private_key_hex)?;
    hello.signature = sign_bytes(&sk, &hello_signing_bytes(&hello)?);
    Ok(hello)
}

pub fn sign_ack(mut ack: PeerAck, private_key_hex: &str) -> Result<PeerAck> {
    let sk = signing_key_from_hex(private_key_hex)?;
    ack.signature = sign_bytes(&sk, &ack_signing_bytes(&ack)?);
    Ok(ack)
}

pub fn sign_envelope(mut env: SignedEnvelope, private_key_hex: &str) -> Result<SignedEnvelope> {
    let sk = signing_key_from_hex(private_key_hex)?;
    env.signature = sign_bytes(&sk, &envelope_signing_bytes(&env)?);
    Ok(env)
}

fn ensure_timestamp_fresh(ts: u64) -> Result<()> {
    let now = now_ms();
    let delta = now.abs_diff(ts);
    ensure!(delta <= MAX_CLOCK_SKEW_MS, "timestamp outside allowed skew");
    Ok(())
}

pub fn verify_hello(hello: &PeerHello, expected_chain_id: &str) -> Result<()> {
    ensure!(
        hello.version == WIRE_PROTOCOL_VERSION,
        "unsupported protocol version"
    );
    ensure!(hello.chain_id == expected_chain_id, "wrong chain id");
    ensure!(!hello.node_name.is_empty(), "missing node name");
    ensure!(!hello.challenge.is_empty(), "missing challenge");
    ensure_timestamp_fresh(hello.timestamp_ms)?;
    verify_bytes(
        &hello.public_key,
        &hello_signing_bytes(hello)?,
        &hello.signature,
    )?;
    Ok(())
}

pub fn verify_ack(ack: &PeerAck, expected_chain_id: &str, expected_challenge: &str) -> Result<()> {
    ensure!(
        ack.version == WIRE_PROTOCOL_VERSION,
        "unsupported protocol version"
    );
    ensure!(ack.chain_id == expected_chain_id, "wrong chain id");
    ensure!(
        ack.peer_challenge == expected_challenge,
        "challenge mismatch"
    );
    ensure!(!ack.own_challenge.is_empty(), "missing own challenge");
    ensure_timestamp_fresh(ack.timestamp_ms)?;
    verify_bytes(&ack.public_key, &ack_signing_bytes(ack)?, &ack.signature)?;
    Ok(())
}

pub fn authenticate_pair(
    hello: &PeerHello,
    ack: &PeerAck,
    expected_chain_id: &str,
) -> Result<SessionInfo> {
    verify_hello(hello, expected_chain_id)?;
    verify_ack(ack, expected_chain_id, &hello.challenge)?;

    let remote_address = address_from_vk(&ed25519_dalek::VerifyingKey::from_bytes(
        &hex::decode(&hello.public_key)?
            .try_into()
            .map_err(|_| anyhow!("bad public key len"))?,
    )?);

    let session_bytes = serde_json::to_vec(&json!({
        "hello_pk": hello.public_key,
        "hello_challenge": hello.challenge,
        "ack_pk": ack.public_key,
        "ack_challenge": ack.own_challenge,
        "chain_id": expected_chain_id,
    }))?;

    Ok(SessionInfo {
        remote_node_name: hello.node_name.clone(),
        remote_address,
        remote_public_key: hello.public_key.clone(),
        session_id: crate::crypto::hash_hex(&session_bytes),
    })
}

pub fn verify_envelope(
    env: &SignedEnvelope,
    session: &SessionInfo,
    state: &ChainState,
    expected_chain_id: &str,
) -> Result<()> {
    ensure!(
        env.version == WIRE_PROTOCOL_VERSION,
        "unsupported protocol version"
    );
    ensure!(env.chain_id == expected_chain_id, "wrong chain id");
    ensure!(env.session_id == session.session_id, "wrong session id");
    ensure!(env.sender == session.remote_node_name, "wrong sender name");
    ensure_timestamp_fresh(env.timestamp_ms)?;

    let validator = state
        .validators
        .get(&env.sender)
        .ok_or_else(|| anyhow!("unknown validator sender"))?;

    verify_bytes(
        &validator.public_key,
        &envelope_signing_bytes(env)?,
        &env.signature,
    )?;
    Ok(())
}

pub fn encode_line<T: Serialize>(value: &T) -> Result<String> {
    let line = serde_json::to_string(value)?;
    bounded_size(line.as_bytes())?;
    Ok(line)
}

pub fn decode_line<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T> {
    bounded_size(line.as_bytes())?;
    Ok(serde_json::from_str(line)?)
}
