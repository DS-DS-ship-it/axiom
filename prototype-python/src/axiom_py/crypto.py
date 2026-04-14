from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)


def hash_hex(data: bytes, digest_size: int = 32) -> str:
    return hashlib.blake2b(data, digest_size=digest_size).hexdigest()


def canonical_dumps(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


@dataclass
class Keypair:
    private_key: Ed25519PrivateKey

    @property
    def public_key(self) -> Ed25519PublicKey:
        return self.private_key.public_key()

    @property
    def public_key_hex(self) -> str:
        return self.public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        ).hex()

    @property
    def address(self) -> str:
        return "axm_" + hash_hex(bytes.fromhex(self.public_key_hex), digest_size=20)

    def sign_payload(self, payload: dict) -> str:
        return self.private_key.sign(canonical_dumps(payload)).hex()


def generate_keypair() -> Keypair:
    return Keypair(Ed25519PrivateKey.generate())


def verify_signature(public_key_hex: str, payload: dict, signature_hex: str) -> bool:
    try:
        pub = Ed25519PublicKey.from_public_bytes(bytes.fromhex(public_key_hex))
        pub.verify(bytes.fromhex(signature_hex), canonical_dumps(payload))
        return True
    except Exception:
        return False
