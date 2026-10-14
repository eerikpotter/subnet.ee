#!/usr/bin/env python3
"""Assemble registered Trunk outputs into one atomic static-site directory."""
import argparse
import base64
import hashlib
import html
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from urllib.parse import urlsplit

from localize import Page, dictionary

ROOT = Path(__file__).resolve().parents[1]

class InlineScripts(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.current = None
        self.scripts = []
        self.wasm_preloads = []
    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "link" and attributes.get("rel") == "preload" and attributes.get("as") == "fetch" and attributes.get("href", "").endswith(".wasm"):
            self.wasm_preloads.append(self.get_starttag_text())
        if tag == "script" and "src" not in dict(attrs):
            self.current = []
    def handle_data(self, data):
        if self.current is not None:
            self.current.append(data)
    def handle_endtag(self, tag):
        if tag == "script" and self.current is not None:
            self.scripts.append("".join(self.current))
            self.current = None

def instrument_editor(folder):
    """Adapt only Trunk's bootstrap; editor JS/Wasm and their URLs stay intact."""
    page = folder / "editor.html"
    source = page.read_text()
    parser = InlineScripts()
    parser.feed(source)
    wasms = list(folder.glob("*.wasm"))
    if len(wasms) != 1:
        raise RuntimeError(f"Expected one Wasm package in {folder}")
    wasm = wasms[0]
    data = wasm.read_bytes()
    integrity = "sha384-" + base64.b64encode(hashlib.sha384(data).digest()).decode()
    generated = ('// subnet-wasm-progress:start\n'
                 'import { loadCraftWasm } from "/craft-wasm.js";\n'
                 'const wasm = await init({ module_or_path: await loadCraftWasm('
                 f'new URL({json.dumps("./" + wasm.name)}, window.location.href), {len(data)}, {json.dumps(integrity)})'
                 ' });\n// subnet-wasm-progress:end')
    previous = re.compile(r'// subnet-wasm-progress:start\n.*?// subnet-wasm-progress:end', re.S)
    original = re.compile(r"const wasm = await init\(\{\s*module_or_path:\s*(['\"])([^'\"]+\.wasm)\1\s*\}\);")
    matches = 0
    for script in parser.scripts:
        pattern = previous if previous.search(script) else original
        if pattern.search(script):
            replacement, count = pattern.subn(lambda _: generated, script)
            matches += count
            source = source.replace(script, replacement, 1)
    if matches != 1:
        raise RuntimeError(f"Unrecognized Trunk bootstrap in {page}; expected one Wasm init")
    # A preload with SRI buffers the response before the reader can observe it.
    # The shared streaming loader enforces the same SHA-384 before execution.
    for tag in parser.wasm_preloads:
        source = source.replace(tag, "", 1)
    page.write_text(source)

def art(app):
    if app == "photocraft":
        return '<div class="art-work"><div class="photo-sky"><span class="photo-sun"></span><span class="photo-hill"></span><span class="photo-hill second"></span></div></div><div class="photo-tools">+<br>◌<br>⌁</div>'
    if app == "pdfcraft":
        lines = '<div class="pdf-line title"></div>' + '<div class="pdf-line"></div>' * 5
        return f'<div class="pdf-sheet back">{lines}</div><div class="pdf-sheet">{lines}</div><span class="pdf-stamp">PDF</span>'
    if app == "wordcraft":
        return '<div class="word-page"><div class="word-title">Alguses<br>oli sõna</div><div class="word-line"></div><div class="word-line short"></div><div class="word-selection"></div><div class="word-line"></div><div class="word-line short"></div></div>'
    if app == "deckcraft":
        return '<div class="deck-slide back"></div><div class="deck-slide"><div class="deck-title">Ideest<br>esitluseni.</div><div class="deck-chart"><span></span><span></span><span></span></div><div class="deck-dots">● ○ ○</div></div>'
    if app == "cadcraft":
        return '<div class="cad-plan"><svg viewBox="0 0 240 150"><path class="cad-wall" d="M35 30h160v90H35Zm100 0v52h60M35 82h52m25 0h23v38"/><path class="cad-door" d="M87 82v25a25 25 0 0 0 25-25"/><path class="cad-measure" d="M35 15h160M35 10v10m160-10v10M210 30v90m-5-90h10m-10 90h10"/><text x="106" y="12">8 000</text><text x="204" y="144">4 500</text></svg></div>'
    if app == "gridcraft":
        return '<div class="grid-sheet"><div class="grid-columns"><span>A</span><span>B</span><span>C</span></div><div class="grid-cells"><span>Ideed</span><span>Tehtud</span><span>Järgmisena</span><span>12</span><span>8</span><span>4</span><span class="grid-selected">=SUM(B2:C2)</span><span></span><span></span></div><div class="grid-bars"><i></i><i></i><i></i><i></i></div></div>'
    # Deterministic original waveform illustration; no external media or scripts.
    heights = [4, 8, 13, 7, 18, 22, 14, 9, 17, 23, 12, 5, 9, 16, 21, 12, 18, 6, 4, 13, 20, 9, 15, 5]
    waves = "".join(f'<span class="wave" style="height:{h}px"></span>' for h in heights)
    return '<div class="sound-tracks">' + f'<div class="track">{waves}</div>' * 3 + '</div>'

def cards(apps):
    colors = {"photocraft":"photo", "pdfcraft":"pdf", "wordcraft":"word", "deckcraft":"deck", "soundcraft":"sound", "cadcraft":"cad", "gridcraft":"grid"}
    result = []
    for app in apps:
        e = {key: html.escape(str(value), quote=True) for key, value in app.items()}
        result.append(f'''<a class="app-card" href="/apps/{e['id']}/" aria-label="Ava {e['name']}">
          <div class="art art-{colors[app['id']]}" aria-hidden="true"><span class="app-symbol">{e['symbol']}</span>{art(app['id'])}</div>
          <div class="card-body"><div class="card-title"><h3>{e['name']}</h3><span class="category">{e['kind']}</span></div><p>{e['description']}</p>
          <div class="card-bottom"><span class="formats">{e['formats']}</span><span class="open-app">Ava rakendus <span aria-hidden="true">→</span></span></div></div></a>''')
    return "\n".join(result)

def landing_cards(apps):
    """Show every registered app, including future apps without custom artwork."""
    captions = {"photocraft": "Pildid ja visuaalid", "pdfcraft": "PDF-id ja dokumendid",
                "wordcraft": "Tekstid ja märkmed", "deckcraft": "Esitlused ja ideed",
                "soundcraft": "Heli ja muusika", "cadcraft": "Joonised ja kavandid",
                "gridcraft": "Tabelid ja andmed"}
    result = []
    for app in apps:
        name = html.escape(app['name'], quote=True)
        app_id = html.escape(app['id'], quote=True)
        caption = html.escape(captions.get(app['id'], app['kind']))
        icon = app_id if app['id'] in captions else "application"
        result.append(f'<a class="tool-card tool-{app_id}" href="/apps/{app_id}/" aria-label="Ava {name}">'
                      f'<svg class="tool-icon" viewBox="0 0 48 48" aria-hidden="true"><use href="#icon-{icon}"/></svg>'
                      f'<h3>{name}</h3><p>{caption}</p><span class="tool-arrow" aria-hidden="true">↗</span></a>')
    return "\n".join(result)

def assemble(root=ROOT, selected=("portal",)):
    apps = json.loads((root / "registry/apps.json").read_text())
    english = dictionary(root / "portal/translations.js")
    origin = os.environ.get("SUBNET_PUBLIC_ORIGIN", "https://www.subnet.ee").rstrip("/")
    parsed_origin = urlsplit(origin)
    if parsed_origin.scheme not in ("http", "https") or not parsed_origin.netloc or parsed_origin.path or parsed_origin.query or parsed_origin.fragment:
        raise RuntimeError("SUBNET_PUBLIC_ORIGIN must be an absolute HTTP(S) origin without a path")
    app_ids = {app['id'] for app in apps}
    if set(selected) - app_ids - {"all", "portal"}:
        raise RuntimeError("Unknown assembly target")
    replace = app_ids if "all" in selected else set(selected) - {"portal"}
    previous = root / "dist"
    manifest_path = previous / "apps.json"
    cached = {app['id']: app for app in json.loads(manifest_path.read_text())['apps']} if manifest_path.exists() else {}
    runtime = root / ".local/portal-runtime"
    if manifest_path.exists() and (not runtime.exists() or ("portal" not in selected and "all" not in selected)):
        runtime = previous / "cloud"
    bindings = list(runtime.glob("sovereign-cloud-web-*.js"))
    wasm = list(runtime.glob("sovereign-cloud-web-*_bg.wasm"))
    if len(bindings) != 1 or len(wasm) != 1:
        raise RuntimeError("Missing portal runtime. Run python3 scripts/build.py portal")
    # Never replace a working bundle with a portal containing missing editor links.
    for app in apps:
        if app['id'] not in replace:
            source = previous / "apps" / app['id']
            if app['id'] not in cached or not (source / "editor.html").is_file() or any(not (source / name).is_file() for name in cached[app['id']]['assets']):
                raise RuntimeError(f"No packaged {app['id']}; run python3 scripts/build.py {app['id']} --assemble (or build.py all --assemble for initial setup)")
        else:
            source = root / "apps" / app['id'] / "dist/web"
        if not (source / "index.html").is_file() or not list(source.glob("*.wasm")) or not list(source.glob("*.js")):
            raise RuntimeError(f"Missing complete {app['id']} build. Run python3 scripts/build.py {app['id']}")
    (root / ".local").mkdir(exist_ok=True)
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
        for page in [stage / "index.html", stage / "pilveruum/index.html"]:
            if page.is_file():
                rendered = (page.read_text()
                            .replace("<!-- APP_CARDS -->", cards(apps))
                            .replace("<!-- LANDING_CARDS -->", landing_cards(apps))
                            .replace("__APP_COUNT_PADDED__", f"{len(apps):02d}")
                            .replace("__APP_COUNT__", str(len(apps))))
                path = "/pilveruum/" if page.parent.name == "pilveruum" else "/"
                template = Page(rendered)
                page.write_text(template.render(english, "et", path, origin))
                english_page = stage / "en" / page.relative_to(stage)
                english_page.parent.mkdir(parents=True, exist_ok=True)
                english_page.write_text(template.render(english, "en", path, origin))
        legacy_en = stage / "en/tooruum/index.html"
        legacy_en.parent.mkdir(parents=True, exist_ok=True)
        legacy_en.write_text((stage / "tooruum/index.html").read_text().replace('lang="et"', 'lang="en"').replace('/pilveruum/', '/en/pilveruum/'))
        hashes = []
        manifest = {"apps": [], "portalAssets": {"cloud/" + p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [bindings[0], wasm[0]]},
                    "shellAssets": {name: hashlib.sha256((stage / name).read_bytes()).hexdigest() for name in ["tokens.css", "craft-shell.css", "craft-shell.js", "craft-loading.js", "craft-wasm.js", "i18n.js", "translations.js"]}}
        for app in apps:
            repo = root / "apps" / app['id']
            destination = stage / "apps" / app['id']
            app_shell = Page(shell.replace("__APP_NAME__", html.escape(app['name'], quote=True))).render(english, "et")
            if app['id'] not in replace:
                # Reuse the last assembled artifact, even if the source checkout is absent/dirty.
                shutil.copytree(previous / "apps" / app['id'], destination)
                (destination / "index.html").write_text(app_shell)
                manifest['apps'].append({**cached[app['id']], **app})
                continue
            shutil.copytree(repo / "dist/web", destination)
            # Rules and II metadata belong to the enclosing single origin.
            for name in ["_headers", "_redirects", ".well-known"]:
                nested = destination / name
                if nested.is_dir(): shutil.rmtree(nested)
                elif nested.exists(): nested.unlink()
            # Preserve relative asset URLs and give every editor its own viewport.
            # Only the public entry point is replaced by the common portal shell.
            (destination / "index.html").rename(destination / "editor.html")
            (destination / "index.html").write_text(app_shell)
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
        for app in apps:
            folder = stage / "apps" / app['id']
            instrument_editor(folder)
            parser = InlineScripts()
            parser.feed((folder / "editor.html").read_text())
            hashes.extend("'sha256-" + base64.b64encode(hashlib.sha256(s.encode()).digest()).decode() + "'" for s in parser.scripts)
        policy = "; ".join(["default-src 'self'", "script-src 'self' 'wasm-unsafe-eval' " + " ".join(sorted(set(hashes))),
            "style-src 'self' 'unsafe-inline'", "img-src 'self' data: blob:", "font-src 'self' data:", "connect-src 'self' https://icp-api.io", "worker-src 'self' blob:", "media-src 'self' blob:", "object-src 'none'", "base-uri 'self'", "frame-ancestors 'self'"])
        headers = f"/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: no-referrer\n  Content-Security-Policy: {policy}\n\n/*.html\n  Cache-Control: no-cache\n\n/*.css\n  Cache-Control: no-cache\n\n/*.json\n  Cache-Control: no-cache\n\n/i18n.js\n  Cache-Control: no-cache\n\n/translations.js\n  Cache-Control: no-cache\n\n/landing.js\n  Cache-Control: no-cache\n\n/cloud.js\n  Cache-Control: no-cache\n\n/craft-shell.js\n  Cache-Control: no-cache\n\n/assets/*\n  Cache-Control: no-cache\n\n/apps/*.wasm\n  Content-Type: application/wasm\n  Cache-Control: public, max-age=31536000, immutable\n\n/apps/*.js\n  Cache-Control: public, max-age=31536000, immutable\n\n/cloud/*.wasm\n  Content-Type: application/wasm\n  Cache-Control: public, max-age=31536000, immutable\n\n/cloud/*.js\n  Cache-Control: public, max-age=31536000, immutable\n\n/.well-known/ii-app-metadata\n  Content-Type: application/json\n  Access-Control-Allow-Origin: *\n"
        headers += "\n/craft-loading.js\n  Cache-Control: no-cache\n\n/craft-wasm.js\n  Cache-Control: no-cache\n\n/agent-workflow.js\n  Cache-Control: no-cache\n"
        headers += "\n/.well-known/ic-domains\n  Content-Type: text/plain\n  Cache-Control: no-cache\n\n/.well-known/ii-alternative-origins\n  Content-Type: application/json\n  Access-Control-Allow-Origin: *\n  Cache-Control: no-cache\n"
        headers += "\n/.well-known/ic-architecture\n  Content-Type: application/json\n  Access-Control-Allow-Origin: *\n  Cache-Control: no-cache\n\n/.well-known/ii-derivation-origin\n  Content-Type: text/plain\n  Access-Control-Allow-Origin: *\n  Cache-Control: no-cache\n"
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
        print(f"Assembled portal; replaced editors: {', '.join(sorted(replace)) or 'none'}; reused {len(apps) - len(replace)} packaged editors")
    finally:
        if stage.exists(): shutil.rmtree(stage)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apps", nargs="*", default=["portal"], help="Editors to replace; default: portal only, all: initial/full assembly")
    options = parser.parse_args()
    try: assemble(selected=options.apps)
    except (RuntimeError, subprocess.CalledProcessError) as error: raise SystemExit(str(error))
