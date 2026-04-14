#!/usr/bin/env python3
import argparse
import json
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "debug" / "axiom-node"

KEY_HEXES = [
    "1111111111111111111111111111111111111111111111111111111111111111",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "3333333333333333333333333333333333333333333333333333333333333333",
    "4444444444444444444444444444444444444444444444444444444444444444",
]

def write_configs(cfg_dir: Path, base_port: int):
    chain_id = "axiom-soak-local"
    genesis_path = (ROOT / ".." / "testnet" / "genesis" / "genesis.json").resolve()
    cfgs = []
    for i in range(4):
        port = base_port + i
        peers = [f'"127.0.0.1:{base_port + j}"' for j in range(4) if j != i]
        state_path = cfg_dir / f"node{i+1}-state.json"
        wal_path = cfg_dir / f"node{i+1}.wal.jsonl"
        text = f'''chain_id = "{chain_id}"
node_name = "node{i+1}"
bind_addr = "127.0.0.1:{port}"
p2p_peers = [{", ".join(peers)}]
private_key_hex = "{KEY_HEXES[i]}"
validator_power = 100
genesis_path = "{genesis_path}"
state_path = "{state_path}"
wal_path = "{wal_path}"
'''
        path = cfg_dir / f"node{i+1}.toml"
        path.write_text(text)
        cfgs.append(path)
    return cfgs

def wait_port(port: int, timeout: float = 10.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return
        except OSError:
            time.sleep(0.1)
    raise RuntimeError(f"port {port} did not come up")

def send_line(addr: str, payload: str):
    host, port = addr.split(":")
    with socket.create_connection((host, int(port)), timeout=1.0) as s:
        s.sendall(payload.encode("utf-8") + b"\n")

def ps_sample(pid: int):
    out = subprocess.check_output(
        ["ps", "-o", "pid=,rss=,%cpu=", "-p", str(pid)],
        text=True,
    ).strip()
    if not out:
        return {"pid": pid, "rss_kb": None, "cpu_percent": None}
    parts = out.split()
    return {
        "pid": int(parts[0]),
        "rss_kb": int(parts[1]),
        "cpu_percent": float(parts[2]),
    }

def restart_proc(proc, cfg_path: Path):
    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
    return subprocess.Popen(
        [str(BIN), "--config", str(cfg_path)],
        cwd=str(ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--hours", type=float, default=24.0)
    ap.add_argument("--base-port", type=int, default=7301)
    ap.add_argument("--restart-every-sec", type=int, default=300)
    ap.add_argument("--sample-every-sec", type=int, default=15)
    ap.add_argument("--malformed-per-cycle", type=int, default=50)
    ap.add_argument("--oversized-per-cycle", type=int, default=5)
    ap.add_argument("--out-dir", default=None)
    args = ap.parse_args()

    if not BIN.exists():
        print(f"missing binary: {BIN}. Run cargo build first.", file=sys.stderr)
        sys.exit(1)

    out_dir = Path(args.out_dir) if args.out_dir else Path(tempfile.mkdtemp(prefix="axiom-soak-"))
    out_dir.mkdir(parents=True, exist_ok=True)
    cfgs = write_configs(out_dir, args.base_port)
    report_path = out_dir / "soak_report.jsonl"
    summary_path = out_dir / "soak_summary.json"

    procs = []
    start = time.time()
    next_restart = start + args.restart_every_sec
    next_sample = start
    restart_index = 0
    samples = 0
    restarts = 0
    failures = 0

    try:
        for cfg in cfgs:
            procs.append(
                subprocess.Popen(
                    [str(BIN), "--config", str(cfg)],
                    cwd=str(ROOT),
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            )

        for i in range(4):
            wait_port(args.base_port + i)

        with report_path.open("a", encoding="utf-8") as rep:
            deadline = start + args.hours * 3600.0
            while time.time() < deadline:
                addr = f"127.0.0.1:{args.base_port}"

                for _ in range(args.malformed_per_cycle):
                    try:
                        send_line(addr, '{"type":"Vote","payload":"not-a-real-vote"}')
                    except OSError:
                        failures += 1

                oversized = "x" * 1_000_001
                for _ in range(args.oversized_per_cycle):
                    try:
                        send_line(addr, oversized)
                    except OSError:
                        failures += 1

                if time.time() >= next_restart:
                    idx = restart_index % 4
                    procs[idx] = restart_proc(procs[idx], cfgs[idx])
                    wait_port(args.base_port + idx)
                    restarts += 1
                    restart_index += 1
                    next_restart = time.time() + args.restart_every_sec

                if time.time() >= next_sample:
                    row = {
                        "ts": time.time(),
                        "alive": [],
                        "samples": [],
                        "failures": failures,
                        "restarts": restarts,
                    }
                    for i, proc in enumerate(procs):
                        alive = proc.poll() is None
                        row["alive"].append(alive)
                        if not alive:
                            failures += 1
                            procs[i] = restart_proc(proc, cfgs[i])
                            wait_port(args.base_port + i)
                            row["alive"][-1] = True
                            restarts += 1
                        row["samples"].append(ps_sample(procs[i].pid))
                    rep.write(json.dumps(row) + "\n")
                    rep.flush()
                    samples += 1
                    next_sample = time.time() + args.sample_every_sec

                time.sleep(1.0)
    finally:
        for proc in procs:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)

        summary = {
            "out_dir": str(out_dir),
            "report_path": str(report_path),
            "samples": samples,
            "restarts": restarts,
            "failures": failures,
            "duration_hours": args.hours,
            "base_port": args.base_port,
        }
        summary_path.write_text(json.dumps(summary, indent=2))
        print(json.dumps(summary, indent=2))

if __name__ == "__main__":
    main()
