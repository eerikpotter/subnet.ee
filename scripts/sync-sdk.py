#!/usr/bin/env python3
"""Reproducible SDK vendoring keeps each fork buildable without its parent checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]
APPS = [app["id"] for app in json.loads((ROOT / "registry/apps.json").read_text())]
PACKAGES = ["sovereign-cloud", "sovereign-cloud-ui"]
FILES = ["Cargo.toml", "LICENSE-MIT", "LICENSE-APACHE", "README.md", "NOTICE", "rustfmt.toml", "clippy.toml"]

def snapshot(source):
    paths = [source / p for p in FILES] + sorted((source / "src").rglob("*.rs")) + sorted(p for p in (source / "assets").rglob("*") if p.is_file())
    return {str(p.relative_to(source)): p.read_bytes() for p in paths}

def sync_package(app, package, check=False, pinned=False):
    source = ROOT / "packages" / package
    dest = ROOT / "apps" / app / "vendor" / package
    if pinned:
        # A fork owns its SDK version; an unrelated canonical SDK edit must not block it.
        manifest = json.loads((dest / "snapshot.json").read_text())
        actual = {str(p.relative_to(dest)) for p in dest.rglob("*") if p.is_file()} - {"snapshot.json"}
        if actual != set(manifest) or any(hashlib.sha256((dest / name).read_bytes()).hexdigest() != digest for name, digest in manifest.items()):
            raise SystemExit(f"Pinned SDK snapshot differs: {app}/{package}")
        print(f"{app}/{package}: pinned snapshot verified")
        return
    files = snapshot(source)
    manifest = {name: hashlib.sha256(content).hexdigest() for name, content in files.items()}
    if check:
        for name, content in files.items():
            target = dest / name
            if not target.is_file() or target.read_bytes() != content:
                raise SystemExit(f"SDK snapshot differs: {app}/{package}/{name}; run python3 scripts/sync-sdk.py {app}")
        if json.loads((dest / "snapshot.json").read_text()) != manifest:
            raise SystemExit("SDK snapshot manifest differs")
        actual = {str(p.relative_to(dest)) for p in dest.rglob("*") if p.is_file()}
        if actual != set(files) | {"snapshot.json"}:
            raise SystemExit("SDK snapshot contains unexpected files")
    else:
        if dest.exists():
            shutil.rmtree(dest)
        for name, content in files.items():
            target = dest / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)
        (dest / "snapshot.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"{app}/{package}: verified" if check else f"{app}/{package}: synchronized")

def sync(apps, check=False, pinned=False):
    for app in APPS if "all" in apps else apps:
        for package in PACKAGES:
            sync_package(app, package, check, pinned)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apps", nargs="+", choices=["all", *APPS])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="Compare selected forks with the canonical SDK")
    mode.add_argument("--check-pinned", action="store_true", help="Verify each fork's own pinned SDK without requiring the latest canonical version")
    options = parser.parse_args()
    sync(options.apps, options.check, options.check_pinned)
