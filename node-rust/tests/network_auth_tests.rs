use axiom_node::{
    crypto::{generate_key, public_key_hex},
    network_auth::{
        authenticate_pair, decode_line, encode_line, sign_ack, sign_envelope, sign_hello,
        verify_envelope, PeerAck, PeerHello, SignedEnvelope, WIRE_PROTOCOL_VERSION,
    },
    state::{ChainState, Validator},
};
use serde_json::json;

#[test]
fn authenticated_handshake_and_envelope_verify() {
    let remote_key = generate_key();
    let local_key = generate_key();

    let hello = sign_hello(
        PeerHello {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: "axiom-local".to_string(),
            node_name: "node2".to_string(),
            bind_addr: "127.0.0.1:9002".to_string(),
            public_key: public_key_hex(&remote_key),
            challenge: "hello-challenge".to_string(),
            timestamp_ms: axiom_node::network_auth::now_ms(),
            signature: String::new(),
        },
        &hex::encode(remote_key.to_bytes()),
    )
    .unwrap();

    let ack = sign_ack(
        PeerAck {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: "axiom-local".to_string(),
            node_name: "node1".to_string(),
            bind_addr: "127.0.0.1:9001".to_string(),
            public_key: public_key_hex(&local_key),
            peer_challenge: "hello-challenge".to_string(),
            own_challenge: "ack-challenge".to_string(),
            timestamp_ms: axiom_node::network_auth::now_ms(),
            signature: String::new(),
        },
        &hex::encode(local_key.to_bytes()),
    )
    .unwrap();

    let session = authenticate_pair(&hello, &ack, "axiom-local").unwrap();

    let mut state = ChainState::default();
    state.validators.insert(
        "node2".to_string(),
        Validator {
            public_key: public_key_hex(&remote_key),
            power: 100,
            stake: 1000,
        },
    );

    let env = sign_envelope(
        SignedEnvelope {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: "axiom-local".to_string(),
            session_id: session.session_id.clone(),
            sender: "node2".to_string(),
            msg_type: "vote".to_string(),
            payload: json!({"height":1,"round":0}),
            timestamp_ms: axiom_node::network_auth::now_ms(),
            signature: String::new(),
        },
        &hex::encode(remote_key.to_bytes()),
    )
    .unwrap();

    let line = encode_line(&env).unwrap();
    let decoded: SignedEnvelope = decode_line(&line).unwrap();
    verify_envelope(&decoded, &session, &state, "axiom-local").unwrap();

    assert_eq!(session.remote_node_name, "node2");
    assert_eq!(session.remote_public_key, public_key_hex(&remote_key));
}

#[test]
fn forged_envelope_is_rejected() {
    let remote_key = generate_key();
    let wrong_key = generate_key();
    let local_key = generate_key();

    let hello = sign_hello(
        PeerHello {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: "axiom-local".to_string(),
            node_name: "node2".to_string(),
            bind_addr: "127.0.0.1:9002".to_string(),
            public_key: public_key_hex(&remote_key),
            challenge: "hello-challenge".to_string(),
            timestamp_ms: axiom_node::network_auth::now_ms(),
            signature: String::new(),
        },
        &hex::encode(remote_key.to_bytes()),
    )
    .unwrap();

    let ack = sign_ack(
        PeerAck {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: "axiom-local".to_string(),
            node_name: "node1".to_string(),
            bind_addr: "127.0.0.1:9001".to_string(),
            public_key: public_key_hex(&local_key),
            peer_challenge: "hello-challenge".to_string(),
            own_challenge: "ack-challenge".to_string(),
            timestamp_ms: axiom_node::network_auth::now_ms(),
            signature: String::new(),
        },
        &hex::encode(local_key.to_bytes()),
    )
    .unwrap();

    let session = authenticate_pair(&hello, &ack, "axiom-local").unwrap();

    let mut state = ChainState::default();
    state.validators.insert(
        "node2".to_string(),
        Validator {
            public_key: public_key_hex(&remote_key),
            power: 100,
            stake: 1000,
        },
    );

    let env = sign_envelope(
        SignedEnvelope {
            version: WIRE_PROTOCOL_VERSION,
            chain_id: "axiom-local".to_string(),
            session_id: session.session_id.clone(),
            sender: "node2".to_string(),
            msg_type: "vote".to_string(),
            payload: json!({"height":1,"round":0}),
            timestamp_ms: axiom_node::network_auth::now_ms(),
            signature: String::new(),
        },
        &hex::encode(wrong_key.to_bytes()),
    )
    .unwrap();

    assert!(verify_envelope(&env, &session, &state, "axiom-local").is_err());
}
