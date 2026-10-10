#!/usr/bin/env python3
"""Reproducible SDK vendoring keeps each fork buildable without its parent checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "packages/sovereign-cloud"
DEST = ROOT / "apps/photocraft/vendor/sovereign-cloud"
FILES = ["Cargo.toml", "LICENSE-MIT", "LICENSE-APACHE", "README.md", "NOTICE", "rustfmt.toml", "clippy.toml"]

def snapshot():
    paths = [SOURCE / p for p in FILES] + sorted((SOURCE / "src").rglob("*.rs"))
    return {str(p.relative_to(SOURCE)): p.read_bytes() for p in paths}

def sync(check=False):
    files = snapshot()
    manifest = {name: hashlib.sha256(content).hexdigest() for name, content in files.items()}
    if check:
        for name, content in files.items():
            target = DEST / name
            if not target.is_file() or target.read_bytes() != content:
                raise SystemExit(f"SDK snapshot differs: {name}; run python3 scripts/sync-sdk.py")
        if json.loads((DEST / "snapshot.json").read_text()) != manifest:
            raise SystemExit("SDK snapshot manifest differs")
        actual = {str(p.relative_to(DEST)) for p in DEST.rglob("*") if p.is_file()}
        if actual != set(files) | {"snapshot.json"}:
            raise SystemExit("SDK snapshot contains unexpected files")
    else:
        if DEST.exists():
            shutil.rmtree(DEST)
        for name, content in files.items():
            target = DEST / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)
        (DEST / "snapshot.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print("SDK snapshot verified" if check else "SDK snapshot synchronized")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    sync(parser.parse_args().check)
