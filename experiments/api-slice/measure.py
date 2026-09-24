#!/usr/bin/env python3
"""Measure API feedback loops in a disposable copy, never editing the checkout.

Requires fetched Cargo dependencies and web/node_modules. Reports wall time,
not server throughput. A fresh target is primed once, followed by warm samples.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--samples", type=int, default=3)
parser.add_argument("--output", type=Path, default=ROOT / "target/api-loop-results.json")
args = parser.parse_args()
if args.samples < 1:
    parser.error("samples must be positive")
args.output.parent.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
for key in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_TARGET_DIR", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"]:
    env.pop(key, None)
env.update(CARGO_BUILD_JOBS="4", CARGO_INCREMENTAL="1", CARGO_NET_OFFLINE="true")
result = {"recorded_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
          "platform": platform.platform(), "available_cpus": len(os.sched_getaffinity(0)),
          "settings": "4 jobs; incremental; default debug; rustc default linker; no wrappers; warm downloads and OS cache; one fresh target",
          "versions": {tool: subprocess.check_output([tool, "--version"], text=True).strip()
                       for tool in ["rustc", "cargo", "node", "npm"]},
          "measurements": []}

with tempfile.TemporaryDirectory(prefix="iris-api-loop-") as temp:
    work = Path(temp)
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]:
        shutil.copy2(ROOT / name, work / name)
    shutil.copytree(ROOT / "experiments", work / "experiments",
                    ignore=shutil.ignore_patterns("node_modules", "dist", "__pycache__", ".contract-probe-*"))
    result["source_sha256"] = {str(p.relative_to(work)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(work.rglob("*")) if p.is_file() and p.suffix in [".rs", ".toml", ".lock", ".sql", ".ts", ".tsx", ".mjs", ".json", ".py"]}
    web = work / "experiments/api-slice/web"
    (web / "node_modules").symlink_to(ROOT / "experiments/api-slice/web/node_modules", target_is_directory=True)

    def measure(label, commands):
        start = time.perf_counter()
        for command in commands:
            subprocess.run(command, cwd=work, env=env, check=True, stdout=subprocess.DEVNULL, timeout=600)
        elapsed = round(time.perf_counter() - start, 4)
        result["measurements"].append({"label": label, "seconds": elapsed, "commands": commands})
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        print(f"{label}: {elapsed}s", flush=True)

    test = ["cargo", "test", "--locked", "-p", "iris-api-spike", "--features", "dev-identity", "--test", "contract", "demo::issue_authorize_then_accept_through_both_contracts", "--", "--exact"]
    generate = ["npm", "--prefix", "experiments/api-slice/web", "run", "generate"]
    types = ["npm", "--prefix", "experiments/api-slice/web", "run", "typecheck"]
    measure("fresh_target_test", [test])
    measure("prime_contract_export_and_types", [generate, types])
    body = work / "experiments/api-slice/server/src/lib.rs"
    contract = work / "experiments/api-slice/server/src/invitations.rs"
    original_body, original_contract = body.read_text(), contract.read_text()
    for sample in range(1, args.samples + 1):
        measure(f"warm_test_{sample}", [test])
        needle = "Only a project owner can issue invitations."
        assert original_body.count(needle) == 1
        body.write_text(original_body.replace(needle, f"{needle} Probe {sample}."))
        measure(f"body_edit_to_test_{sample}", [test])
        field = "pub expires_at: String,"
        value = "expires_at: expires_at.to_string(),"
        assert original_contract.count(field) == original_contract.count(value) == 1
        contract.write_text(original_contract
            .replace(field, f"{field}\n    pub contract_probe_{sample}: Option<String>,")
            .replace(value, f"{value}\n                contract_probe_{sample}: None,"))
        measure(f"contract_shape_edit_to_react_types_{sample}", [generate, types])
        # Re-prime the feature-enabled test after the contract edit, so the next
        # no-change sample really has no pending Rust compilation work.
        subprocess.run(test, cwd=work, env=env, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
print(f"Results: {args.output}")
