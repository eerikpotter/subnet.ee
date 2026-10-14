#!/usr/bin/env python3
"""Generate MCP discovery at sync time, using the target environment's canisters.

Operator opt-in approved on 2026-10-14; see docs/collaboration-progress.md.
Only the cloud API is declared. Ordinary builds do not generate an opt-in.
"""
import base64
import binascii
import json
import os
from pathlib import Path
import zlib

ROOT = Path(__file__).resolve().parents[1]


def canister_id(env, name):
    value = env.get(name, "")
    compact = value.replace("-", "")
    try:
        raw = base64.b32decode(compact.upper() + "=" * (-len(compact) % 8))
        principal = raw[4:]
        canonical = base64.b32encode(raw).decode().lower().rstrip("=")
        canonical = "-".join(canonical[i:i + 5] for i in range(0, len(canonical), 5))
        if (not principal or principal == b"\x04" or len(principal) > 29
                or raw[:4] != zlib.crc32(principal).to_bytes(4, "big")
                or value != canonical):
            raise ValueError()
    except (ValueError, binascii.Error):
        raise ValueError(f"{name} must contain the deployed canister's canonical principal") from None
    return value


def generate(destination, env):
    cloud = canister_id(env, "ICP_CLI_CID_CLOUD")
    frontend = canister_id(env, "ICP_CLI_CID_FRONTEND")
    if cloud == frontend:
        raise ValueError("The cloud and frontend must be different canisters")
    network = env.get("ICP_CLI_NETWORK")
    if network not in ("local", "ic"):
        raise ValueError("MCP discovery supports the configured local and ic networks only")
    if not destination.is_dir():
        raise ValueError("Assemble the frontend before generating MCP discovery")
    directory = destination / ".well-known"
    directory.mkdir(exist_ok=True)
    manifest = {"version": "1.0.0", "canisters": [{
        "id": cloud,
        "name": "cloud",
        "role": "subnet.ee file and folder storage API",
        "description": "Account-authorized files, folders, sharing and revisions. API semantics are available from get_api_doc.",
    }]}
    (directory / "ic-architecture").write_text(json.dumps(manifest, indent=2) + "\n")
    identity = directory / "ii-derivation-origin"
    if network == "ic":
        # Same identity contract as icp.yaml and the browser's certified ic_env.
        # verify.py checks both against the actual frontend ID after deployment.
        identity.write_text(f"https://{frontend}.icp.net\n")
    else:
        # Local browser login uses its visible origin. Do not retain a mainnet
        # identity pointer when the same build directory is synced locally.
        identity.unlink(missing_ok=True)


if __name__ == "__main__":
    try:
        generate(ROOT / "dist", os.environ)
    except ValueError as error:
        raise SystemExit(str(error)) from None
