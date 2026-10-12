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

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apps", nargs="*", default=["portal"], help="Targets to replace (default: portal); all is explicit")
    parser.add_argument("--build", action="store_true", help="Build only the selected targets first")
    parser.add_argument("--bootstrap", action="store_true", help="First-time deployment of both frontend and cloud backend")
    options = parser.parse_args(argv)
    registered = {app['id'] for app in json.loads((ROOT / "registry/apps.json").read_text())}
    if set(options.apps) - registered - {"portal", "all"}:
        parser.error("Unknown application")
    mappings = ROOT / ".icp/cache/mappings/local.ids.json"
    if not options.bootstrap and (not mappings.exists() or not {'frontend', 'cloud'} <= json.loads(mappings.read_text()).keys()):
        parser.error("Initial setup: python3 scripts/dev.py all --build --bootstrap")
    if options.build:
        run(sys.executable, "scripts/build.py", *options.apps)
    run(sys.executable, "scripts/assemble.py", *options.apps)
    result = subprocess.run(["icp", "network", "status", "local", "--json"], cwd=ROOT, capture_output=True, text=True)
    if result.returncode:
        run("icp", "network", "start", "local", "-d")
    env = os.environ.copy()
    env["RUSTUP_TOOLCHAIN"] = env.get("SUBNET_RUST_TOOLCHAIN", "1.95.0")
    env.setdefault("ICP_CLI_PLUGIN_COMPUTE_LIMIT_SECS", "900")
    if options.bootstrap:
        run("icp", "deploy", "cloud", "frontend", "-e", "local", env=env)
    else:
        # Asset sync never rebuilds/reinstalls the cloud backend or editor source.
        run("icp", "sync", "frontend", "-e", "local", env=env)
    network = json.loads(subprocess.check_output(["icp", "network", "status", "local", "--json"], cwd=ROOT))
    port = urlparse(network['gateway_url']).port
    print(f"\nLocal: http://frontend.local.localhost:{port}/")
    print(f"Verify: python3 scripts/verify.py {' '.join(options.apps)}")

if __name__ == "__main__":
    main()
