from __future__ import annotations

from dataclasses import dataclass

from .crypto import hash_hex, canonical_dumps
from .models import Account, Block, Contract, Transaction
from .vm import TinyVM, VMError


class ValidationError(Exception):
    pass


@dataclass
class Genesis:
    chain_id: str
    validators: dict[str, int]
    accounts: dict[str, int]


class AxiomChain:
    def __init__(self, genesis: Genesis) -> None:
        self.chain_id = genesis.chain_id
        self.validators = dict(genesis.validators)
        self.accounts = {name: Account(balance=bal) for name, bal in genesis.accounts.items()}
        self.contracts: dict[str, Contract] = {}
        self.height = 0
        self.tip_hash = "GENESIS"
        self.base_fee = 1
        self.total_burned = 0
        self.vm = TinyVM()
        self.mempool: list[Transaction] = []

    def ensure_account(self, address: str) -> Account:
        return self.accounts.setdefault(address, Account())

    def proposer_for_height(self, height: int) -> str:
        ordered = sorted(self.validators)
        return ordered[(height - 1) % len(ordered)]

    def validate_transaction(self, tx: Transaction) -> None:
        if tx.chain_id != self.chain_id:
            raise ValidationError("wrong chain id")
        if not tx.verify():
            raise ValidationError("bad signature")
        if tx.max_fee_per_gas < self.base_fee:
            raise ValidationError("fee below base fee")
        sender = self.ensure_account(tx.sender)
        if sender.nonce != tx.nonce:
            raise ValidationError(f"bad nonce: expected {sender.nonce}, got {tx.nonce}")
        required = tx.value + tx.gas_limit * tx.max_fee_per_gas
        if sender.balance < required:
            raise ValidationError("insufficient balance")
        if tx.kind not in {"transfer", "deploy", "call"}:
            raise ValidationError("unsupported kind")
        if tx.kind == "transfer" and not tx.recipient:
            raise ValidationError("missing recipient")
        if tx.kind == "deploy" and not tx.data:
            raise ValidationError("missing contract code")

    def submit_tx(self, tx: Transaction) -> None:
        self.validate_transaction(tx)
        self.mempool.append(tx)

    def apply_transaction(self, tx: Transaction, proposer: str) -> int:
        self.validate_transaction(tx)
        sender = self.ensure_account(tx.sender)
        sender.balance -= tx.gas_limit * tx.max_fee_per_gas
        sender.nonce += 1

        gas_used = 21_000
        if tx.kind == "transfer":
            sender.balance -= tx.value
            self.ensure_account(tx.recipient).balance += tx.value
        elif tx.kind == "deploy":
            sender.balance -= tx.value
            address = self.derive_contract_address(tx.sender, tx.nonce)
            self.contracts[address] = Contract(address=address, owner=tx.sender, code=tx.data or [], balance=tx.value)
        elif tx.kind == "call":
            if tx.recipient not in self.contracts:
                raise ValidationError("unknown contract")
            contract = self.contracts[tx.recipient]
            sender.balance -= tx.value
            contract.balance += tx.value
            result = self.vm.execute(contract.code, contract.storage, max(tx.gas_limit - gas_used, 1))
            contract.storage = result.storage
            gas_used += result.gas_used

        burn = gas_used * self.base_fee
        effective = gas_used * tx.max_fee_per_gas
        tip = max(0, effective - burn)
        self.total_burned += burn
        self.ensure_account(proposer).balance += tip
        refund = tx.gas_limit * tx.max_fee_per_gas - effective
        sender.balance += refund
        return gas_used

    def produce_block(self) -> Block:
        proposer = self.proposer_for_height(self.height + 1)
        selected = list(self.mempool)
        self.mempool.clear()
        for tx in selected:
            self.apply_transaction(tx, proposer)
        block = Block(
            height=self.height + 1,
            parent_hash=self.tip_hash,
            proposer=proposer,
            base_fee=self.base_fee,
            txs=selected,
            state_root=self.state_root(),
        )
        self.height = block.height
        self.tip_hash = block.block_hash()
        return block

    def state_root(self) -> str:
        payload = {
            "accounts": {k: {"balance": v.balance, "nonce": v.nonce, "staked": v.staked} for k, v in sorted(self.accounts.items())},
            "contracts": {k: {"owner": v.owner, "balance": v.balance, "storage": dict(sorted(v.storage.items()))} for k, v in sorted(self.contracts.items())},
            "height": self.height,
        }
        return hash_hex(canonical_dumps(payload))

    def derive_contract_address(self, sender: str, nonce: int) -> str:
        return "axc_" + hash_hex(f"{sender}:{nonce}".encode(), digest_size=20)
