#!/usr/bin/env python3
"""Verify the actual local canister routes, payloads, CSP, runtime trust and cache headers."""
import base64
import gzip
import hashlib
import http.client
import json
from pathlib import Path
import subprocess
from urllib.parse import unquote, urlparse

from assemble import InlineScripts

ROOT = Path(__file__).resolve().parents[1]

def main():
    network = json.loads(subprocess.check_output(["icp", "network", "status", "local", "--json"], cwd=ROOT))
    if not network.get("managed"):
        raise RuntimeError("Expected the managed local test network")
    port = urlparse(network['gateway_url']).port
    ids = json.loads((ROOT / ".icp/cache/mappings/local.ids.json").read_text())
    host = f"{ids['frontend']}.localhost:{port}"
    def get(path):
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=120)
        connection.request("GET", path, headers={"Host":host, "Accept-Encoding":"gzip"})
        response = connection.getresponse()
        headers = {k.lower():v for k, v in response.getheaders()}
        status, body = response.status, response.read()
        connection.close()
        if headers.get("content-encoding") == "gzip": body = gzip.decompress(body)
        return status, headers, body
    manifest = json.loads((ROOT / "dist/apps.json").read_text())
    checks = [("/", ROOT / "dist/index.html"), ("/style.css", ROOT / "dist/style.css"), ("/apps.json", ROOT / "dist/apps.json")]
    checks.append(("/cloud.js", ROOT / "dist/cloud.js"))
    checks.extend(("/" + name, ROOT / "dist" / name) for name in manifest["portalAssets"])
    if any((ROOT / "dist" / name).exists() for name in ["browser-session-test.html", "browser-session-test.js"]):
        raise RuntimeError("Remove temporary browser fixtures before final verification")
    for app in manifest['apps']:
        folder = ROOT / "dist/apps" / app['id']
        checks.append((app['path'], folder / "index.html"))
        checks.extend((app['path'] + name, folder / name) for name in app['assets'])
    for route, file in checks:
        status, headers, data = get(route)
        if status != 200 or data != file.read_bytes(): raise RuntimeError(f"Wrong payload/status: {route} ({status})")
        if not headers.get("ic-certificate"): raise RuntimeError(f"Missing certification header: {route}")
        if file.suffix == ".wasm" and headers.get("content-type") != "application/wasm": raise RuntimeError(f"Wrong Wasm MIME: {route}")
        if file.suffix in (".js", ".wasm") and file.name != "cloud.js" and "immutable" not in headers.get("cache-control", ""): raise RuntimeError(f"Missing versioned cache rule: {route}")
        if file.name == "index.html":
            env_cookie = headers.get("set-cookie", "").split(";", 1)[0]
            if not env_cookie.startswith("ic_env="): raise RuntimeError(f"Missing runtime cookie: {route}")
            env = dict(item.split("=", 1) for item in unquote(env_cookie.removeprefix("ic_env=")).split("&"))
            if env.get("PUBLIC_CANISTER_ID:cloud") != ids['cloud'] or env.get("ic_root_key") != network['root_key']:
                raise RuntimeError("Runtime endpoint/trust does not match managed network")
            policy = headers.get("content-security-policy", "")
            parser = InlineScripts()
            parser.feed(data.decode())
            for script in parser.scripts:
                expected = "'sha256-" + base64.b64encode(hashlib.sha256(script.encode()).digest()).decode() + "'"
                if expected not in policy: raise RuntimeError(f"Bootstrap blocked by CSP: {route}")
            if "no-cache" not in headers.get("cache-control", ""): raise RuntimeError(f"Stale HTML cache policy: {route}")
        print(f"PASS {route} ({len(data):,} bytes)")
    for route in ["/apps/photocraft/missing.wasm", "/apps/unknown/", "/missing.js", "/browser-session-test.html", "/browser-session-test.js"]:
        if get(route)[0] != 404: raise RuntimeError(f"Missing file must return 404: {route}")
    status, headers, body = get("/.well-known/ii-app-metadata")
    if status != 200 or json.loads(body)['name'] != "subnet.ee" or headers.get('access-control-allow-origin') != '*':
        raise RuntimeError("II app metadata unavailable")
    print(f"PASS portal, all four applications, trust configuration and removed-fixture/missing-asset handling\nLocal: http://frontend.local.localhost:{port}/")

if __name__ == "__main__": main()
