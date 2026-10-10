#!/usr/bin/env python3
"""Start or update the shared LOCAL canister environment. Never deploys to mainnet."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]

def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, **kwargs)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", action="store_true", help="Rebuild all four editors first")
    options = parser.parse_args()
    if options.build:
        run(sys.executable, "scripts/build.py", "all")
    run(sys.executable, "scripts/assemble.py")
    result = subprocess.run(["icp", "network", "status", "local", "--json"], cwd=ROOT, capture_output=True, text=True)
    if result.returncode:
        run("icp", "network", "start", "local", "-d")
    env = os.environ.copy()
    env["RUSTUP_TOOLCHAIN"] = env.get("SUBNET_RUST_TOOLCHAIN", "1.95.0")
    env.setdefault("ICP_CLI_PLUGIN_COMPUTE_LIMIT_SECS", "900")
    run("icp", "deploy", "-e", "local", env=env)
    network = json.loads(subprocess.check_output(["icp", "network", "status", "local", "--json"], cwd=ROOT))
    port = urlparse(network['gateway_url']).port
    print(f"\nLocal: http://frontend.local.localhost:{port}/")
    print("Verify: python3 scripts/verify.py")
