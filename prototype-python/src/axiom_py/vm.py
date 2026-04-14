from __future__ import annotations

from dataclasses import dataclass


class VMError(Exception):
    pass


@dataclass
class VMResult:
    gas_used: int
    storage: dict[str, int]


class TinyVM:
    """Very small deterministic VM for the Python prototype.

    Supported opcodes:
    - PUSH {value}
    - ADD
    - SUB
    - STORE {key}
    - LOAD {key}
    - HALT
    """

    def execute(self, code: list[dict], storage: dict[str, int], gas_limit: int) -> VMResult:
        stack: list[int] = []
        pc = 0
        gas_used = 0
        current = dict(storage)

        while pc < len(code):
            op = code[pc].get("op")
            gas_used += 1
            if gas_used > gas_limit:
                raise VMError("out of gas")

            if op == "PUSH":
                stack.append(int(code[pc]["value"]))
            elif op == "ADD":
                if len(stack) < 2:
                    raise VMError("stack underflow")
                b = stack.pop()
                a = stack.pop()
                stack.append(a + b)
            elif op == "SUB":
                if len(stack) < 2:
                    raise VMError("stack underflow")
                b = stack.pop()
                a = stack.pop()
                stack.append(a - b)
            elif op == "STORE":
                if not stack:
                    raise VMError("stack underflow")
                current[str(code[pc]["key"])] = stack.pop()
            elif op == "LOAD":
                current_value = int(current.get(str(code[pc]["key"]), 0))
                stack.append(current_value)
            elif op == "HALT":
                break
            else:
                raise VMError(f"unknown opcode: {op}")
            pc += 1

        return VMResult(gas_used=gas_used, storage=current)
