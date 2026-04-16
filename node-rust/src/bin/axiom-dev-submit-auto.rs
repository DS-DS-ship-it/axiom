use std::{
    env,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use axiom_node::{
    crypto::{address_from_vk, public_key_hex, sign_bytes, signing_key_from_hex},
    types::Transaction,
    validation::signing_bytes,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AccountView {
    found: bool,
    nonce: u64,
}

fn curl_text(args: &[&str]) -> String {
    let out = Command::new("curl")
        .args(args)
        .output()
        .expect("failed to run curl");

    if !out.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
        std::process::exit(1);
    }

    String::from_utf8(out.stdout).expect("curl output was not utf8")
}

fn main() {
    let mut submit = false;
    let mut positional = Vec::new();

    for arg in env::args().skip(1) {
        if arg == "--submit" {
            submit = true;
        } else {
            positional.push(arg);
        }
    }

    let value: u64 = if let Some(v) = positional.first() {
        v.parse().expect("value must be a u64")
    } else {
        1000
    };

    let recipient = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| "axm_f7426c6c4cd4c0e171778352ea7315e4ea089ca6".to_string());

    let base_url = positional
        .get(2)
        .cloned()
        .unwrap_or_else(|| "http://127.0.0.1:8401".to_string());

    let base_url = base_url.trim_end_matches('/').to_string();

    let key_hex = "5555555555555555555555555555555555555555555555555555555555555555";
    let sk = signing_key_from_hex(key_hex).unwrap();
    let sender = address_from_vk(&sk.verifying_key());

    let account_url = format!("{}/account/{}", base_url, sender);
    let account_json = curl_text(&["-sS", &account_url]);
    let account: AccountView =
        serde_json::from_str(&account_json).expect("failed to parse /account response");

    if !account.found {
        eprintln!("sender account not found on node");
        std::process::exit(1);
    }

    let nonce = account.nonce;

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut tx = Transaction {
        chain_id: "axiom-1".to_string(),
        kind: "transfer".to_string(),
        sender,
        sender_pubkey: public_key_hex(&sk),
        nonce,
        gas_limit: 10,
        max_fee_per_gas: 3,
        value,
        recipient: Some(recipient),
        data: None,
        timestamp_ms,
        signature: String::new(),
    };

    tx.signature = sign_bytes(&sk, &signing_bytes(&tx).unwrap());

    let body = serde_json::json!({ "tx": tx });
    let body_string = serde_json::to_string_pretty(&body).unwrap();

    if !submit {
        println!("{}", body_string);
        return;
    }

    let submit_url = format!("{}/tx", base_url);
    let out = Command::new("curl")
        .args([
            "-sS",
            "-X",
            "POST",
            &submit_url,
            "-H",
            "content-type: application/json",
            "--data-binary",
            &body_string,
        ])
        .output()
        .expect("failed to run curl for submit");

    if !out.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
        std::process::exit(1);
    }

    println!("{}", String::from_utf8_lossy(&out.stdout));
}
