use axiom_node::crypto::{
    address_from_vk, generate_key, public_key_hex, sign_bytes, signing_key_from_hex, verify_bytes,
};

#[test]
fn sign_and_verify_round_trip() {
    let sk = generate_key();
    let pk_hex = public_key_hex(&sk);
    let msg = b"axiom-test-message";
    let sig_hex = sign_bytes(&sk, msg);

    verify_bytes(&pk_hex, msg, &sig_hex).unwrap();
}

#[test]
fn bad_signature_is_rejected() {
    let sk = generate_key();
    let pk_hex = public_key_hex(&sk);
    let sig_hex = sign_bytes(&sk, b"message-a");

    assert!(verify_bytes(&pk_hex, b"message-b", &sig_hex).is_err());
}

#[test]
fn private_key_round_trip_and_address_format() {
    let sk = generate_key();
    let sk_hex = hex::encode(sk.to_bytes());
    let loaded = signing_key_from_hex(&sk_hex).unwrap();

    assert_eq!(public_key_hex(&sk), public_key_hex(&loaded));

    let addr = address_from_vk(&loaded.verifying_key());
    assert!(addr.starts_with("axm_"));
    assert_eq!(addr.len(), 44);
}
