use axiom_node::{
    crypto::{address_from_vk, public_key_hex, sign_bytes, signing_key_from_hex},
    types::Transaction,
    validation::signing_bytes,
};

fn main() {
    let key_hex = "5555555555555555555555555555555555555555555555555555555555555555";
    let recipient = "axm_f7426c6c4cd4c0e171778352ea7315e4ea089ca6";
    let nonce = 1u64;
    let value = 1000u64;

    let sk = signing_key_from_hex(key_hex).unwrap();
    let sender = address_from_vk(&sk.verifying_key());

    let mut tx = Transaction {
        chain_id: "axiom-1".to_string(),
        kind: "transfer".to_string(),
        sender,
        sender_pubkey: public_key_hex(&sk),
        nonce,
        gas_limit: 10,
        max_fee_per_gas: 3,
        value,
        recipient: Some(recipient.to_string()),
        data: None,
        timestamp_ms: 1_700_000_123_000,
        signature: String::new(),
    };

    tx.signature = sign_bytes(&sk, &signing_bytes(&tx).unwrap());

    let body = serde_json::json!({ "tx": tx });
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
}
