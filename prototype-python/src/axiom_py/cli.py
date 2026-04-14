from __future__ import annotations

import argparse
import json

from .chain import AxiomChain, Genesis
from .crypto import generate_keypair
from .models import Transaction


def cmd_demo(_: argparse.Namespace) -> None:
    alice = generate_keypair()
    bob = generate_keypair()
    val = generate_keypair()
    chain = AxiomChain(
        Genesis(
            chain_id="axiom-testnet-1",
            validators={val.address: 1000},
            accounts={alice.address: 1_000_000, bob.address: 0, val.address: 0},
        )
    )
    tx = Transaction(
        kind="transfer",
        sender=alice.address,
        sender_pubkey=alice.public_key_hex,
        recipient=bob.address,
        value=50_000,
        nonce=0,
        gas_limit=25_000,
        max_fee_per_gas=2,
    )
    tx.sign(alice.sign_payload)
    chain.submit_tx(tx)
    block = chain.produce_block()
    print(json.dumps({
        "height": block.height,
        "block_hash": block.block_hash(),
        "alice_balance": chain.accounts[alice.address].balance,
        "bob_balance": chain.accounts[bob.address].balance,
        "validator_tip_balance": chain.accounts[val.address].balance,
        "burned": chain.total_burned,
    }, indent=2))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="AXIOM Python reference CLI")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("demo", help="run a signed transfer demo")
    p.set_defaults(func=cmd_demo)
    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
