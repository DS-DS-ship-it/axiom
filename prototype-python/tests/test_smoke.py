from axiom_py.chain import AxiomChain, Genesis
from axiom_py.crypto import generate_keypair
from axiom_py.models import Transaction


def test_signed_transfer_smoke() -> None:
    alice = generate_keypair()
    bob = generate_keypair()
    val = generate_keypair()
    chain = AxiomChain(
        Genesis(
            chain_id="axiom-testnet-1",
            validators={val.address: 1000},
            accounts={alice.address: 100_000, bob.address: 0, val.address: 0},
        )
    )
    tx = Transaction(
        kind="transfer",
        sender=alice.address,
        sender_pubkey=alice.public_key_hex,
        recipient=bob.address,
        value=10_000,
        nonce=0,
        gas_limit=25_000,
        max_fee_per_gas=2,
    )
    tx.sign(alice.sign_payload)
    chain.submit_tx(tx)
    block = chain.produce_block()
    assert block.height == 1
    assert chain.accounts[bob.address].balance == 10_000
    assert chain.total_burned > 0
