#!/usr/bin/env python3
import json
import os
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from urllib.request import urlopen, Request

HOST = "127.0.0.1"
PORT = 8500

SCRIPT_DIR = Path(__file__).resolve().parent
NODE_RUST_DIR = Path(os.environ.get("AXIOM_NODE_RUST_DIR", SCRIPT_DIR / "node-rust"))

def pick_submit_command(amount: int, address: str, base_url: str):
    built = NODE_RUST_DIR / "target" / "debug" / "axiom-dev-submit-auto"
    if built.exists():
        return [str(built), "--submit", str(amount), address, base_url]
    return [
        "cargo", "run", "--bin", "axiom-dev-submit-auto", "--",
        "--submit", str(amount), address, base_url
    ]

def fetch_json(url: str):
    req = Request(url, headers={"accept": "application/json"})
    with urlopen(req, timeout=10) as resp:
        return json.loads(resp.read().decode("utf-8"))

class Handler(BaseHTTPRequestHandler):
    def _cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")

    def _json(self, code: int, payload: dict):
        body = json.dumps(payload, indent=2).encode("utf-8")
        self.send_response(code)
        self._cors()
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self):
        self.send_response(204)
        self._cors()
        self.end_headers()

    def do_GET(self):
        if self.path == "/" or self.path == "/health":
            self._json(200, {
                "ok": True,
                "service": "fortiq-devnet-faucet",
                "host": HOST,
                "port": PORT,
                "node_rust_dir": str(NODE_RUST_DIR),
            })
            return
        self._json(404, {"ok": False, "error": "not found"})

    def do_POST(self):
        if self.path != "/faucet":
            self._json(404, {"ok": False, "error": "not found"})
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
            raw = self.rfile.read(length).decode("utf-8")
            data = json.loads(raw or "{}")

            address = str(data.get("address", "")).strip()
            if not address.startswith("axm_"):
                raise ValueError("address must start with axm_")

            amount = int(data.get("amount", 1000))
            if amount <= 0:
                raise ValueError("amount must be > 0")
            if amount > 100000:
                raise ValueError("amount too large for demo faucet")

            base_url = str(data.get("base_url", "http://127.0.0.1:8401")).strip().rstrip("/")

            cmd = pick_submit_command(amount, address, base_url)
            proc = subprocess.run(
                cmd,
                cwd=NODE_RUST_DIR,
                text=True,
                capture_output=True,
                timeout=120,
                check=False,
            )

            if proc.returncode != 0:
                self._json(500, {
                    "ok": False,
                    "error": "submitter failed",
                    "command": cmd,
                    "stdout": proc.stdout,
                    "stderr": proc.stderr,
                })
                return

            try:
                submit_response = json.loads(proc.stdout)
            except Exception:
                submit_response = {"raw": proc.stdout.strip()}

            account = fetch_json(f"{base_url}/account/{address}")
            status = fetch_json(f"{base_url}/status")

            self._json(200, {
                "ok": True,
                "submitted_to": base_url,
                "address": address,
                "amount": amount,
                "submit_response": submit_response,
                "account_after": account,
                "status_after": status,
            })
        except Exception as exc:
            self._json(400, {"ok": False, "error": str(exc)})

if __name__ == "__main__":
    print(f"fortiq faucet helper listening on http://{HOST}:{PORT}")
    print(f"using node-rust dir: {NODE_RUST_DIR}")
    HTTPServer((HOST, PORT), Handler).serve_forever()
