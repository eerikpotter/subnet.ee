#!/usr/bin/env python3
"""Keep the standalone PhotoCraft backend a checked snapshot of the root service."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'services/sovereign-cloud'
DEST = ROOT / 'apps/photocraft/icp/backend'

def sync(check=False):
    files = {str(p.relative_to(SOURCE)): p.read_bytes() for p in sorted((SOURCE / 'src').rglob('*.rs'))}
    files['cloud.did'] = (SOURCE / 'cloud.did').read_bytes()
    files['api.md'] = (SOURCE / 'api.md').read_bytes()
    # Keep dependencies part of the same snapshot; only deployment packaging
    # differs in PhotoCraft's independent ICP workspace.
    cargo = (SOURCE / 'Cargo.toml').read_text()
    cargo = cargo.replace('name = "sovereign-cloud-service"', 'name = "photocraft-cloud"')
    cargo = cargo.replace('[workspace]\n\n', '')
    cargo = cargo.replace('../../packages/sovereign-cloud', '../../vendor/sovereign-cloud')
    cargo = cargo.split('[profile.release]')[0].rstrip() + '\n'
    files['Cargo.toml'] = cargo.encode()
    manifest = {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}
    for name, data in files.items():
        path = DEST / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit(f'Cloud service snapshot differs: {name}; run scripts/sync-service.py')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    if check and {str(p.relative_to(DEST)) for p in (DEST / 'src').rglob('*.rs')} != {name for name in files if name.startswith('src/')} :
        raise SystemExit('Cloud service snapshot contains unexpected source files')
    path = DEST / 'snapshot.json'
    if check:
        if not path.exists() or json.loads(path.read_text()) != manifest:
            raise SystemExit('Cloud service snapshot manifest differs')
    else:
        path.write_text(json.dumps(manifest, indent=2) + '\n')
    print('Cloud service snapshot verified' if check else 'Cloud service snapshot synchronized')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    sync(parser.parse_args().check)
