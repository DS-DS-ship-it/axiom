from __future__ import annotations

from dataclasses import dataclass, field, asdict
from time import time
from typing import Any

from .crypto import canonical_dumps, hash_hex, verify_signature


@dataclass
class Account:
    balance: int = 0
    nonce: int = 0
    staked: int = 0


@dataclass
class Contract:
    address: str
    owner: str
    code: list[dict[str, Any]]
    storage: dict[str, int] = field(default_factory=dict)
    balance: int = 0


@dataclass
class Transaction:
    kind: str
    sender: str
    sender_pubkey: str
    nonce: int
    gas_limit: int
    max_fee_per_gas: int
    value: int = 0
    recipient: str = ""
    data: list[dict[str, Any]] | None = None
    chain_id: str = "axiom-testnet-1"
    timestamp_ms: int = field(default_factory=lambda: int(time() * 1000))
    signature: str = ""

    def signing_payload(self) -> dict[str, Any]:
        payload = asdict(self)
        payload.pop("signature", None)
        return payload

    def sign(self, signer) -> None:
        self.signature = signer(self.signing_payload())

    def verify(self) -> bool:
        return verify_signature(self.sender_pubkey, self.signing_payload(), self.signature)

    def txid(self) -> str:
        return hash_hex(canonical_dumps(self.signing_payload()))


@dataclass
class Block:
    height: int
    parent_hash: str
    proposer: str
    base_fee: int
    txs: list[Transaction]
    state_root: str = ""
    timestamp_ms: int = field(default_factory=lambda: int(time() * 1000))

    def block_hash(self) -> str:
        body = [tx.txid() for tx in self.txs]
        return hash_hex(canonical_dumps({
            "height": self.height,
            "parent_hash": self.parent_hash,
            "proposer": self.proposer,
            "base_fee": self.base_fee,
            "txs": body,
            "state_root": self.state_root,
            "timestamp_ms": self.timestamp_ms,
        }))
