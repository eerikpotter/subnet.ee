#!/usr/bin/env python3
"""Assemble registered Trunk outputs into one atomic static-site directory."""
import argparse
import base64
import hashlib
import html
from html.parser import HTMLParser
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

class InlineScripts(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.current = None
        self.scripts = []
    def handle_starttag(self, tag, attrs):
        if tag == "script" and "src" not in dict(attrs):
            self.current = []
    def handle_data(self, data):
        if self.current is not None:
            self.current.append(data)
    def handle_endtag(self, tag):
        if tag == "script" and self.current is not None:
            self.scripts.append("".join(self.current))
            self.current = None

def art(app):
    if app == "photocraft":
        return '<div class="art-work"><div class="photo-sky"><span class="photo-sun"></span><span class="photo-hill"></span><span class="photo-hill second"></span></div></div><div class="photo-tools">+<br>◌<br>⌁</div>'
    if app == "pdfcraft":
        lines = '<div class="pdf-line title"></div>' + '<div class="pdf-line"></div>' * 5
        return f'<div class="pdf-sheet back">{lines}</div><div class="pdf-sheet">{lines}</div><span class="pdf-stamp">PDF</span>'
    if app == "wordcraft":
        return '<div class="word-page"><div class="word-title">Algus on<br>üks mõte.</div><div class="word-line"></div><div class="word-line short"></div><div class="word-selection"></div><div class="word-line"></div><div class="word-line short"></div></div>'
    if app == "deckcraft":
        return '<div class="deck-slide back"></div><div class="deck-slide"><div class="deck-title">Ideest<br>esitluseni.</div><div class="deck-chart"><span></span><span></span><span></span></div><div class="deck-dots">● ○ ○</div></div>'
    # Deterministic original waveform illustration; no external media or scripts.
    heights = [4, 8, 13, 7, 18, 22, 14, 9, 17, 23, 12, 5, 9, 16, 21, 12, 18, 6, 4, 13, 20, 9, 15, 5]
    waves = "".join(f'<span class="wave" style="height:{h}px"></span>' for h in heights)
    return '<div class="sound-tracks">' + f'<div class="track">{waves}</div>' * 3 + '</div>'

def cards(apps):
    colors = {"photocraft":"photo", "pdfcraft":"pdf", "wordcraft":"word", "deckcraft":"deck", "soundcraft":"sound"}
    result = []
    for app in apps:
        e = {key: html.escape(str(value), quote=True) for key, value in app.items()}
        result.append(f'''<a class="app-card" href="/apps/{e['id']}/" aria-label="Ava {e['name']}">
          <div class="art art-{colors[app['id']]}" aria-hidden="true"><span class="app-symbol">{e['symbol']}</span>{art(app['id'])}</div>
          <div class="card-body"><div class="card-title"><h3>{e['name']}</h3><span class="category">{e['kind']}</span></div><p>{e['description']}</p>
          <div class="card-bottom"><span class="formats">{e['formats']}</span><span class="open-app">Ava rakendus <span aria-hidden="true">→</span></span></div></div></a>''')
    return "\n".join(result)

def assemble(root=ROOT):
    apps = json.loads((root / "registry/apps.json").read_text())
    subprocess.run(["python3", str(root / "scripts/sync-sdk.py"), "--check"], check=True)
    subprocess.run(["python3", str(root / "scripts/sync-service.py"), "--check"], check=True)
    runtime = root / ".local/portal-runtime"
    bindings = list(runtime.glob("sovereign-cloud-web-*.js"))
    wasm = list(runtime.glob("sovereign-cloud-web-*_bg.wasm"))
    if len(bindings) != 1 or len(wasm) != 1:
        raise RuntimeError("Missing portal runtime. Run python3 scripts/build.py portal")
    # Never replace a working bundle with a portal containing missing editor links.
    for app in apps:
        source = root / "apps" / app['id'] / "dist/web"
        if not (source / "index.html").is_file() or not list(source.glob("*.wasm")) or not list(source.glob("*.js")):
            raise RuntimeError(f"Missing complete {app['id']} build. Run python3 scripts/build.py {app['id']}")
    stage = Path(tempfile.mkdtemp(prefix="subnet-site-", dir=root / ".local"))
    try:
        shutil.copytree(root / "portal", stage, dirs_exist_ok=True)
        shell = (stage / "craft-shell.html").read_text()
        (stage / "craft-shell.html").unlink()
        cloud = stage / "cloud"
        cloud.mkdir()
        for path in [bindings[0], wasm[0]]:
            shutil.copy2(path, cloud / path.name)
        facade = (stage / "cloud.js").read_text().replace("__CLOUD_BINDINGS__", "/cloud/" + bindings[0].name).replace("__CLOUD_WASM__", "/cloud/" + wasm[0].name)
        (stage / "cloud.js").write_text(facade)
        index = ((stage / "index.html").read_text()
                 .replace("<!-- APP_CARDS -->", cards(apps))
                 .replace("__APP_COUNT_PADDED__", f"{len(apps):02d}")
                 .replace("__APP_COUNT__", str(len(apps))))
        (stage / "index.html").write_text(index)
        hashes = []
        manifest = {"apps": [], "portalAssets": {"cloud/" + p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [bindings[0], wasm[0]]},
                    "shellAssets": {name: hashlib.sha256((stage / name).read_bytes()).hexdigest() for name in ["craft-shell.css", "craft-shell.js"]}}
        for app in apps:
            repo = root / "apps" / app['id']
            destination = stage / "apps" / app['id']
            shutil.copytree(repo / "dist/web", destination)
            # Rules and II metadata belong to the enclosing single origin.
            for name in ["_headers", "_redirects", ".well-known"]:
                nested = destination / name
                if nested.is_dir(): shutil.rmtree(nested)
                elif nested.exists(): nested.unlink()
            parser = InlineScripts()
            parser.feed((destination / "index.html").read_text())
            hashes.extend("'sha256-" + base64.b64encode(hashlib.sha256(s.encode()).digest()).decode() + "'" for s in parser.scripts)
            # Preserve relative asset URLs and give every editor its own viewport.
            # Only the public entry point is replaced by the common portal shell.
            (destination / "index.html").rename(destination / "editor.html")
            (destination / "index.html").write_text(shell.replace("__APP_NAME__", html.escape(app['name'], quote=True)))
            licenses = destination / "licenses"
            licenses.mkdir(exist_ok=True)
            for name in ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md", "ATTRIBUTION.toml"]:
                if (repo / name).is_file(): shutil.copy2(repo / name, destination / name)
            for parent in [repo / "assets", repo / "vendor", repo / "apps" / f"{app['id']}-web" / "assets"]:
                if parent.is_dir():
                    for path in parent.rglob("*"):
                        if path.is_file() and (path.name.startswith(("LICENSE", "OFL")) or path.suffix == ".license"):
                            target = licenses / path.relative_to(repo)
                            target.parent.mkdir(parents=True, exist_ok=True)
                            shutil.copy2(path, target)
            manifest['apps'].append({**app, "path": f"/apps/{app['id']}/", "editorPath": f"/apps/{app['id']}/editor.html", "sourceCommit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
                "sourceDirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=repo, text=True).strip()),
                "assets": {f.name: hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(destination.glob("*")) if f.suffix in (".wasm", ".js")}})
        policy = "; ".join(["default-src 'self'", "script-src 'self' 'wasm-unsafe-eval' " + " ".join(sorted(set(hashes))),
            "style-src 'self' 'unsafe-inline'", "img-src 'self' data: blob:", "font-src 'self' data:", "connect-src 'self'", "worker-src 'self' blob:", "media-src 'self' blob:", "object-src 'none'", "base-uri 'self'", "frame-ancestors 'self'"])
        headers = f"/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: no-referrer\n  Content-Security-Policy: {policy}\n  Cache-Control: no-cache\n\n/apps/*.wasm\n  Content-Type: application/wasm\n  Cache-Control: public, max-age=31536000, immutable\n\n/apps/*.js\n  Cache-Control: public, max-age=31536000, immutable\n\n/cloud/*.wasm\n  Content-Type: application/wasm\n  Cache-Control: public, max-age=31536000, immutable\n\n/cloud/*.js\n  Cache-Control: public, max-age=31536000, immutable\n\n/.well-known/ii-app-metadata\n  Content-Type: application/json\n  Access-Control-Allow-Origin: *\n"
        (stage / "_headers").write_text(headers)
        well_known = stage / ".well-known"
        well_known.mkdir(exist_ok=True)
        (well_known / "ii-app-metadata").write_text(json.dumps({"name":"subnet.ee", "description":"A personal creative workspace with Sovereign Cloud files."}))
        (stage / "apps.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
        destination = root / "dist"
        backup = root / ".local/previous-dist"
        if backup.exists(): shutil.rmtree(backup)
        if destination.exists(): destination.rename(backup)
        try:
            stage.rename(destination)
        except Exception:
            if backup.exists(): backup.rename(destination)
            raise
        if backup.exists(): shutil.rmtree(backup)
        print(f"Assembled {len(apps)} apps in {destination}")
    finally:
        if stage.exists(): shutil.rmtree(stage)

if __name__ == "__main__":
    (ROOT / ".local").mkdir(exist_ok=True)
    try: assemble()
    except (RuntimeError, subprocess.CalledProcessError) as error: raise SystemExit(str(error))
