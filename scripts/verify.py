#!/usr/bin/env python3
"""Verify local or mainnet routes, payloads, CSP, runtime trust and cache headers."""
import argparse
import base64
import gzip
import hashlib
import http.client
import json
from pathlib import Path
import subprocess
import time
from urllib.parse import unquote, urlparse

from assemble import InlineScripts

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    registered = json.loads((ROOT / "registry/apps.json").read_text())
    parser.add_argument("apps", nargs="*", default=["portal"], help="Targets to verify (default: portal); all downloads every editor's assets")
    parser.add_argument("--environment", choices=("local", "ic"), default="local", help="Deployment to verify; defaults to local")
    parser.add_argument("--origin", help="Custom HTTPS origin to verify on mainnet")
    parser.add_argument("--timings", action="store_true", help="Report HTTP transfer sizes and timings (not browser startup times)")
    options = parser.parse_args()
    app_ids = {app['id'] for app in registered}
    if set(options.apps) - app_ids - {"portal", "all"}:
        parser.error("Unknown application")
    selected = app_ids if "all" in options.apps else set(options.apps) - {"portal"}
    network = json.loads(subprocess.check_output(["icp", "network", "status", options.environment, "--json"], cwd=ROOT))
    if options.environment == "local" and not network.get("managed"):
        raise RuntimeError("Expected the managed local test network")
    if options.environment == "ic" and network.get("root_key_source") != "mainnet":
        raise RuntimeError("Expected the pinned mainnet root key")
    gateway = urlparse(network['gateway_url'])
    port = gateway.port
    mapping_dir = "cache" if options.environment == "local" else "data"
    ids = json.loads((ROOT / f".icp/{mapping_dir}/mappings/{options.environment}.ids.json").read_text())
    host = (f"{ids['frontend']}.localhost:{port}" if options.environment == "local"
            else f"{ids['frontend']}.{gateway.hostname}")
    if options.origin:
        custom = urlparse(options.origin)
        if options.environment != "ic" or custom.scheme != "https" or not custom.hostname or custom.port or custom.username or custom.password or custom.path not in ("", "/") or custom.query or custom.fragment:
            parser.error("--origin requires a mainnet HTTPS origin without a port, credentials, path, query or fragment")
        host = custom.hostname
    timings = {}
    def get(path):
        connection = (http.client.HTTPConnection("127.0.0.1", port, timeout=120)
                      if options.environment == "local" else http.client.HTTPSConnection(host, timeout=120))
        started = time.perf_counter()
        connection.request("GET", path, headers={"Host":host, "Accept-Encoding":"gzip"})
        response = connection.getresponse()
        headers = {k.lower():v for k, v in response.getheaders()}
        status, body = response.status, response.read()
        timings[path] = (time.perf_counter() - started, len(body))
        connection.close()
        if headers.get("content-encoding") == "gzip": body = gzip.decompress(body)
        return status, headers, body
    manifest = json.loads((ROOT / "dist/apps.json").read_text())
    if [app['id'] for app in manifest['apps']] != [app['id'] for app in registered]:
        raise RuntimeError("Deployed bundle must include every registered app in order")
    portal = (ROOT / "dist/index.html").read_text()
    workspace = (ROOT / "dist/pilveruum/index.html").read_text()
    if '__APP_COUNT' in portal + workspace or f"{len(registered)} tööriista" not in workspace:
        raise RuntimeError("Portal application count does not match the registry")
    checks = [("/", ROOT / "dist/index.html"), ("/style.css", ROOT / "dist/style.css"), ("/apps.json", ROOT / "dist/apps.json")]
    checks.extend((route, ROOT / "dist" / path) for route, path in [
        ("/pilveruum/", "pilveruum/index.html"), ("/en/", "en/index.html"), ("/en/pilveruum/", "en/pilveruum/index.html")])
    checks.extend(("/" + name, ROOT / "dist" / name) for name in ["landing.css", "landing.js", "pilveruum/index.html", "assets/brand/subnet.png", "assets/landing/estonian-lakes.png", "assets/landing/cloud-of-estonia.png", "assets/landing/creative-workspace.jpg"])
    checks.extend(("/" + name, ROOT / "dist" / name) for name in ["agent-workflow.css", "agent-workflow.js", "assets/landing/sovereign-cloud.png"])
    image_assets = json.loads((ROOT / "dist/image-assets.json").read_text())
    checks.extend((url, ROOT / "dist" / url.lstrip('/')) for url in image_assets)
    checks.append(("/cloud.js", ROOT / "dist/cloud.js"))
    if "portal" in options.apps or "all" in options.apps:
        checks.extend(("/" + name, ROOT / "dist" / name) for name in manifest["portalAssets"])
    checks.extend(("/" + name, ROOT / "dist" / name) for name in manifest["shellAssets"])
    if any((ROOT / "dist" / name).exists() for name in ["browser-session-test.html", "browser-session-test.js"]):
        raise RuntimeError("Remove temporary browser fixtures before final verification")
    for app in manifest['apps']:
        if f'href="{app["path"]}"' not in workspace:
            raise RuntimeError(f"Missing portal link: {app['id']}")
        folder = ROOT / "dist/apps" / app['id']
        checks.append((app['path'], folder / "index.html"))
        shell = (folder / "index.html").read_text()
        if ('__APP_NAME__' in shell or 'id="craft-editor"' not in shell or 'data-leave' not in shell
                or 'href="/pilveruum/"' not in shell or 'href="/pilveruum/#failid"' not in shell):
            raise RuntimeError(f"Missing shared navigation: {app['id']}")
        checks.append((app['editorPath'], folder / "editor.html"))
        if app['id'] not in selected:
            continue
        checks.extend((app['path'] + name, folder / name) for name in app['assets'])
        release = json.loads((folder / "release.json").read_text())
        if release['app'] != app['id'] or release['version'] != app['appVersion']:
            raise RuntimeError(f"App release metadata mismatch: {app['id']}")
        checks.append((app['path'] + "release.json", folder / "release.json"))
        for name, asset in release['assets'].items():
            file = folder / name
            if len(file.read_bytes()) != asset['bytes'] or hashlib.sha256(file.read_bytes()).hexdigest() != asset['sha256']:
                raise RuntimeError(f"App release asset mismatch: {app['id']}/{name}")
            if name not in app['assets'] and name != "editor.html":
                checks.append((app['path'] + name, file))
    checks = list(dict.fromkeys(checks))
    for route, file in checks:
        status, headers, data = get(route)
        if status != 200 or data != file.read_bytes(): raise RuntimeError(f"Wrong payload/status: {route} ({status})")
        if not headers.get("ic-certificate"): raise RuntimeError(f"Missing certification header: {route}")
        if file.suffix == ".wasm" and headers.get("content-type") != "application/wasm": raise RuntimeError(f"Wrong Wasm MIME: {route}")
        if route in image_assets:
            if not headers.get("content-type", "").startswith("image/webp"):
                raise RuntimeError(f"Wrong image MIME: {route}")
            directives = {value.strip() for value in headers.get("cache-control", "").split(",")}
            if not {"immutable", "max-age=31536000"} <= directives or directives & {"no-cache", "no-store", "max-age=0"}:
                raise RuntimeError(f"Wrong image cache policy: {route}")
        if file.suffix in (".js", ".wasm") and file.parent != ROOT / "dist" and file.name not in ("craft-wasm.js", "craft-cache-sw.js"):
            directives = {value.strip().lower() for value in headers.get("cache-control", "").split(",")}
            if not {"immutable", "max-age=31536000"} <= directives or directives & {"no-cache", "no-store", "max-age=0"}:
                raise RuntimeError(f"Missing or conflicting versioned cache rule: {route}")
        if file.name in ("index.html", "editor.html"):
            env_cookie = headers.get("set-cookie", "").split(";", 1)[0]
            if not env_cookie.startswith("ic_env="): raise RuntimeError(f"Missing runtime cookie: {route}")
            env = dict(item.split("=", 1) for item in unquote(env_cookie.removeprefix("ic_env=")).split("&"))
            if env.get("PUBLIC_CANISTER_ID:cloud") != ids['cloud'] or env.get("ic_root_key") != network['root_key']:
                raise RuntimeError("Runtime endpoint/trust does not match deployment network")
            if options.environment == "ic" and (env.get("PUBLIC_SOVEREIGN_CLOUD_GATEWAY") != "https://icp-api.io" or env.get("PUBLIC_SOVEREIGN_CLOUD_DERIVATION_ORIGIN") != f"https://{ids['frontend']}.icp.net"):
                raise RuntimeError("Missing custom-domain API or Internet Identity configuration")
            policy = headers.get("content-security-policy", "")
            if "connect-src 'self' https://icp-api.io" not in policy:
                raise RuntimeError("CSP blocks the mainnet API")
            parser = InlineScripts()
            parser.feed(data.decode())
            for script in parser.scripts:
                expected = "'sha256-" + base64.b64encode(hashlib.sha256(script.encode()).digest()).decode() + "'"
                if expected not in policy: raise RuntimeError(f"Bootstrap blocked by CSP: {route}")
            if "no-cache" not in headers.get("cache-control", ""): raise RuntimeError(f"Stale HTML cache policy: {route}")
        print(f"PASS {route} ({len(data):,} bytes)")
        if options.timings:
            seconds, transferred = timings[route]
            print(f"  HTTP {seconds:.3f}s; {transferred:,} transferred bytes; encoding={headers.get('content-encoding', 'identity')}; cache={headers.get('cache-control', 'none')}", flush=True)
    for route in ["/apps/photocraft/missing.wasm", "/apps/unknown/", "/missing.js", "/browser-session-test.html", "/browser-session-test.js", "/apps/wordcraft/_cached/" + "0" * 64 + "/editor.html"]:
        if get(route)[0] != 404: raise RuntimeError(f"Missing file must return 404: {route}")
    status, headers, body = get("/.well-known/ii-app-metadata")
    if status != 200 or json.loads(body)['name'] != "subnet.ee" or headers.get('access-control-allow-origin') != '*':
        raise RuntimeError("II app metadata unavailable")
    status, headers, body = get("/.well-known/ic-domains")
    if status != 200 or body.decode().splitlines() != ["www.subnet.ee"] or not headers.get("ic-certificate"):
        raise RuntimeError("Custom domain ownership file unavailable")
    status, headers, body = get("/.well-known/ii-alternative-origins")
    if status != 200 or json.loads(body) != {"alternativeOrigins": ["https://www.subnet.ee"]} or not headers.get("content-type", "").startswith("application/json") or headers.get("access-control-allow-origin") != "*" or not headers.get("ic-certificate"):
        raise RuntimeError("Internet Identity alternative origin is not available with JSON, CORS and certification")
    status, headers, body = get("/.well-known/ic-architecture")
    if (status != 200 or not headers.get("content-type", "").startswith("application/json")
            or headers.get("access-control-allow-origin") != "*" or not headers.get("ic-certificate")
            or "no-cache" not in headers.get("cache-control", "")):
        raise RuntimeError("MCP manifest is not available with JSON, CORS, revalidation and certification")
    discovery = json.loads(body)
    if (discovery.get("version") != "1.0.0"
            or [entry.get("id") for entry in discovery.get("canisters", [])] != [ids['cloud']]):
        raise RuntimeError("MCP manifest must declare exactly this environment's cloud API")
    status, headers, body = get("/.well-known/ii-derivation-origin")
    if options.environment == "ic":
        if (status != 200 or body.decode().strip() != f"https://{ids['frontend']}.icp.net"
                or not headers.get("content-type", "").startswith("text/plain")
                or not headers.get("ic-certificate") or "no-cache" not in headers.get("cache-control", "")):
            raise RuntimeError("MCP and browser Internet Identity origins differ")
    elif status != 404:
        raise RuntimeError("Local MCP identity must default to the visible origin")
    url = f"http://frontend.local.localhost:{port}/" if options.environment == "local" else f"https://{host}/"
    print(f"PASS shared site, {len(selected)} selected editors ({', '.join(sorted(selected)) or 'none'}), trust and missing-asset handling\n{options.environment}: {url}")

if __name__ == "__main__": main()
