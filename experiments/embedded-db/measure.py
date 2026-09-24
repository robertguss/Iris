#!/usr/bin/env python3
"""Measure disposable source copies; never edit the working tree or clear its target.

Run after cargo fetch --locked. Results describe test-binary build/relink latency,
not request throughput. Downloads and OS-cache eviction are intentionally excluded.
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
import tomllib


ROOT = Path(__file__).resolve().parents[2]
EXPERIMENT = Path("experiments/embedded-db")


def output(command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--output", type=Path, default=ROOT / "target/embedded-db-results.json")
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    logs = ROOT / "target" / f"db-measure-{time.time_ns()}"
    logs.mkdir(parents=True)
    inputs = [ROOT / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")]
    for directory in ("sqlite", "turso", "shared"):
        inputs.extend(p for p in (ROOT / EXPERIMENT / directory).rglob("*") if p.is_file())
    # Cargo resolves every workspace member even for -p. Copy other members so
    # adding a later experiment does not break this isolated measurement runner.
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    for member in workspace["workspace"]["members"]:
        for path in (ROOT / member).rglob("*"):
            if path.is_file() and path not in inputs:
                inputs.append(path)
    env = os.environ.copy()
    for key in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_TARGET_DIR",
                "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    env["CARGO_BUILD_JOBS"] = "4"
    env["CARGO_INCREMENTAL"] = "1"
    result = {
        "recorded_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "rustc": output(["rustc", "-Vv"]),
        "cargo": output(["cargo", "-V"]),
        "platform": platform.platform(),
        "cpu": output(["sh", "-c", "grep -m1 'model name' /proc/cpuinfo"]),
        "available_cpus": len(os.sched_getaffinity(0)),
        "memory": output(["sh", "-c", "grep MemTotal /proc/meminfo"]),
        "cc": output(["cc", "--version"]).splitlines()[0],
        "system_ld": output(["ld", "--version"]).splitlines()[0],
        "settings": {"jobs": 4, "incremental": True, "profile": "default test (debug)",
                     "cache": "fresh target per sample; warm downloaded crates and OS cache",
                     "wrappers": "disabled", "rustflags": "unset", "linker": "rustc default"},
        "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted(inputs)},
        "samples": [],
    }

    def save():
        args.output.write_text(json.dumps(result, indent=2) + "\n")

    for sample in range(1, args.samples + 1):
        # Alternate order to reduce systematic first/second-candidate bias.
        for backend in (("sqlite", "turso") if sample % 2 else ("turso", "sqlite")):
            with tempfile.TemporaryDirectory(prefix="iris-db-measure-") as tmp:
                work = Path(tmp)
                for source in inputs:
                    dest = work / source.relative_to(ROOT)
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(source, dest)
                package = f"iris-{backend}-spike"
                base = ["cargo", "test", "--offline", "--locked", "-p", package, "--test", "scenarios"]
                build = [*base, "--no-run"]
                entry = {"backend": backend, "sample": sample, "measurements": []}
                result["samples"].append(entry)

                def measure(label, command):
                    logfile = logs / f"{sample}-{backend}-{label}.log"
                    print(f"{backend} sample {sample}: {label}", flush=True)
                    start = time.perf_counter()
                    with logfile.open("w") as log:
                        completed = subprocess.run(command, cwd=work, env=env, stdout=log,
                                                   stderr=subprocess.STDOUT, timeout=600)
                    duration = time.perf_counter() - start
                    entry["measurements"].append({"label": label, "command": command,
                                                  "seconds": round(duration, 4),
                                                  "exit_code": completed.returncode})
                    save()
                    if completed.returncode:
                        raise SystemExit(f"Failed; inspect {logfile}")
                    print(f"  {duration:.3f}s", flush=True)

                measure("clean_test_build", build)
                measure("warm_no_change", build)
                source = work / EXPERIMENT / backend / "src/lib.rs"
                text = source.read_text()
                needle = "invitation claim lost under write lock"
                assert text.count(needle) == 1
                source.write_text(text.replace(needle, needle + " (measured body edit)"))
                measure("function_body_edit", build)
                domain = work / EXPERIMENT / "shared/domain.rs"
                text = domain.read_text()
                assert text.count("pub enum Outcome {") == 1
                domain.write_text(text.replace("pub enum Outcome {", "pub enum Outcome {\n    BenchmarkProbe,"))
                measure("shared_type_edit", build)
                measure("targeted_test", [*base, "scenarios::expiration_boundary", "--", "--exact"])
                measure("all_scenarios", base)
    print(f"Results: {args.output}\nLogs: {logs}")


if __name__ == "__main__":
    main()
