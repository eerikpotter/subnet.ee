#!/usr/bin/env python3
"""Build independently versioned editors, then assemble one static-site artifact."""
import argparse
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
def build_portal(dev=False):
    env = os.environ.copy()
    env["PATH"] = str(ROOT / ".tools/bin") + os.pathsep + env["PATH"]
    env["RUSTUP_TOOLCHAIN"] = env.get("SUBNET_RUST_TOOLCHAIN", "1.95.0")
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/portal"))
    env.pop("NO_COLOR", None)
    command = ["trunk", "build", "--locked", "--public-url", "./", "--color", "never"]
    if not dev: command.append("--release")
    subprocess.run(command, cwd=ROOT / "packages/sovereign-cloud-web", env=env, check=True)

APPS = tuple(app["id"] for app in json.loads((ROOT / "registry/apps.json").read_text()))

class CloudFeatures(HTMLParser):
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag != "link" or attrs.get("rel") != "rust" or attrs.get("data-type") == "worker":
            return
        features = attrs.get("data-cargo-features")
        if features is not None and "icp-cloud" not in features.replace(",", " ").split():
            raise RuntimeError("Trunk HTML overrides --features: retain icp-cloud alongside upstream data-cargo-features")

def build(app, dev=False):
    repo = ROOT / "apps" / app
    if not (repo / "Cargo.toml").is_file():
        raise RuntimeError(f"Missing {app}; run git submodule update --init --recursive")
    CloudFeatures().feed((repo / "apps" / f"{app}-web" / "index.html").read_text())
    env = os.environ.copy()
    env["PATH"] = str(ROOT / ".tools/bin") + os.pathsep + env["PATH"]
    env["RUSTUP_TOOLCHAIN"] = env.get("SUBNET_RUST_TOOLCHAIN", "1.95.0")
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/craft"))
    env.pop("NO_COLOR", None)
    if not shutil.which("trunk", path=env["PATH"]):
        raise RuntimeError("Install Trunk 0.21.14 or put it in .tools/bin/trunk")
    command = ["trunk", "build", "--locked", "--public-url", "./", "--color", "never"]
    if not dev:
        command.append("--release")
    subprocess.run([sys.executable, str(ROOT / "scripts/sync-sdk.py"), app, "--check-pinned"], check=True)
    command.extend(["--features", "icp-cloud"])
    print(f"Building {app}", flush=True)
    subprocess.run(command, cwd=repo / "apps" / f"{app}-web", env=env, check=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apps", nargs="+", choices=["all", "portal", *APPS])
    parser.add_argument("--dev", action="store_true")
    parser.add_argument("--assemble", action="store_true")
    options = parser.parse_args()
    if "all" in options.apps or "portal" in options.apps:
        build_portal(options.dev)
    selected = APPS if "all" in options.apps else [app for app in options.apps if app != "portal"]
    for app in selected:
        build(app, options.dev)
    if options.assemble:
        subprocess.run([sys.executable, str(ROOT / "scripts/assemble.py"), *options.apps], check=True)
