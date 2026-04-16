#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

GENESIS = {
  "chain_id":"axiom-1",
  "protocol_version":1,
  "genesis_time_ms":1700000000000,
  "base_fee":2,
  "checkpoint_interval":10,
  "validators":[
    {"name":"node1","public_key":"d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c9778737","bind_addr":"127.0.0.1:7401","power":100,"stake":1000000},
    {"name":"node2","public_key":"a09aa5f47a6759802ff955f8dc2d2a14a5c99d23be97f864127ff9383455a4f0","bind_addr":"127.0.0.1:7402","power":100,"stake":1000000},
    {"name":"node3","public_key":"17cb79fb2b4120f2b1ec65e4198d6e08b28e813feb01e4a400839b85e18080ce","bind_addr":"127.0.0.1:7403","power":100,"stake":1000000},
    {"name":"node4","public_key":"d759793bbc13a2819a827c76adb6fba8a49aee007f49f2d0992d99b825ad2c48","bind_addr":"127.0.0.1:7404","power":100,"stake":1000000}
  ],
  "accounts":[
    {"address":"axm_devnet_faucet","balance":1000000000,"nonce":0,"staked":0},
    {"address":"axm_ops_reserve","balance":250000000,"nonce":0,"staked":0}
  ],
  "metadata":{"network":"devnet","asset":"AXM","purpose":"axiom-1 bootstrap"}
}

KEYS = [
    "1111111111111111111111111111111111111111111111111111111111111111",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "3333333333333333333333333333333333333333333333333333333333333333",
    "4444444444444444444444444444444444444444444444444444444444444444",
]

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="testnet/devnet")
    args = ap.parse_args()

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    (out / "axiom-1-genesis.json").write_text(json.dumps(GENESIS, indent=2))

    for i, key in enumerate(KEYS, start=1):
        peers = [f'"127.0.0.1:{7400+j}"' for j in range(1, 5) if j != i]
        text = f"""chain_id = "axiom-1"
node_name = "node{i}"
bind_addr = "127.0.0.1:{7400+i}"
p2p_peers = [{", ".join(peers)}]
private_key_hex = "{key}"
validator_power = 100
genesis_path = "testnet/devnet/axiom-1-genesis.json"
state_path = "testnet/devnet/node{i}-state.json"
wal_path = "testnet/devnet/node{i}.wal.jsonl"
"""
        (out / f"node{i}.toml").write_text(text)

    print(f"wrote {out / 'axiom-1-genesis.json'}")
    print("wrote node1.toml through node4.toml")

if __name__ == "__main__":
    main()
