#!/usr/bin/env python3
"""Verify presync and certified discovery on disposable local ICP canisters.

Uses the existing tiny assembly-test fixture, not a Craft UI/browser fixture.
Never uses the main project's canisters or deploys to the ic environment.
"""
import http.client
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from test_scoped_workflow import ScopedWorkflowTest


def main():
    wasm = Path(os.environ.get("CLOUD_WASM", ROOT / ".icp/cache/artifacts/cloud"))
    if not wasm.is_file():
        raise SystemExit("Build cloud with the official ICP recipe first")
    fixture = ScopedWorkflowTest()
    fixture.setUp()
    root = fixture.root
    (root / "scripts").mkdir()
    shutil.copyfile(ROOT / "scripts/mcp-discovery.py", root / "scripts/mcp-discovery.py")
    shutil.copyfile(wasm, root / "cloud.wasm")
    (root / "icp.yaml").write_text("""canisters:
  - name: cloud
    recipe:
      type: "@dfinity/prebuilt@v2.1.0"
      configuration:
        path: cloud.wasm
  - name: frontend
    recipe:
      type: "@dfinity/static-site@v0.4.0"
      configuration:
        dir: dist
        presync:
          - python3 scripts/mcp-discovery.py
networks:
  - name: local
    mode: managed
    gateway:
      port: 0
""")

    def icp(*args, capture=False):
        result = subprocess.run(["icp", *args], cwd=root, check=True, text=True,
                                capture_output=capture)
        return result.stdout if capture else None

    started = False
    try:
        icp("network", "start", "local", "-d")
        started = True
        icp("deploy", "-e", "local", "--identity", "anonymous")
        network = json.loads(icp("network", "status", "local", "--json", capture=True))
        assert network['managed']
        gateway = urlparse(network['gateway_url'])
        ids = json.loads((root / ".icp/cache/mappings/local.ids.json").read_text())

        def get(path):
            connection = http.client.HTTPConnection("127.0.0.1", gateway.port, timeout=20)
            connection.request("GET", path, headers={"Host": f"{ids['frontend']}.localhost:{gateway.port}"})
            response = connection.getresponse()
            result = response.status, dict(response.getheaders()), response.read()
            connection.close()
            return result

        status, raw_headers, body = get("/.well-known/ic-architecture")
        headers = {key.lower(): value for key, value in raw_headers.items()}
        assert status == 200, (status, body)
        assert headers['content-type'].startswith("application/json"), headers
        assert headers['access-control-allow-origin'] == "*", headers
        assert 'no-cache' in headers['cache-control'], headers
        assert headers['ic-certificate'], headers
        manifest = json.loads(body)
        assert manifest['version'] == '1.0.0'
        assert [c['id'] for c in manifest['canisters']] == [ids['cloud']], manifest
        assert get("/.well-known/ii-derivation-origin")[0] == 404
        assert get("/.well-known/not-a-discovery-file")[0] == 404
        print("PASS actual ICP presync: injected cloud ID, certified JSON/CORS/cache headers, local identity fallback and missing-path 404", flush=True)
    finally:
        if started:
            icp("network", "stop", "local")
        fixture.doCleanups()


if __name__ == "__main__":
    main()
