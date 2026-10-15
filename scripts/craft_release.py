"""Describe the one deployed editor package; old packages live only in browsers."""
import hashlib
import json
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen


def official_notes(app, version, response):
    """Accept only the public upstream release for this exact app version."""
    url = f"https://github.com/storytold/{app}/releases/tag/v{version}"
    if (response.get("draft") or response.get("tag_name") != f"v{version}"
            or response.get("html_url") != url or not response.get("published_at")):
        raise ValueError(f"Not a published {app} v{version} release")
    return {"version": version, "url": url, "publishedAt": response["published_at"][:10],
            "body": response.get("body") or ""}


def refresh_notes(root):
    """Explicit network step; normal assembly is deterministic and offline."""
    apps = json.loads((root / "dist/apps.json").read_text())["apps"]
    entries = {}
    for app in apps:
        name, version = app["id"], app["appVersion"]
        request = Request(f"https://api.github.com/repos/storytold/{name}/releases/tags/v{version}",
                          headers={"User-Agent": "subnet-craft-releases", "Accept": "application/vnd.github+json"})
        try:
            with urlopen(request, timeout=30) as response:
                entries[name] = official_notes(name, version, json.load(response))
            print(f"{name}: ArtCraft v{version} release notes")
        except HTTPError as error:
            if error.code != 404:
                raise
            print(f"{name}: no published v{version} release; notes unavailable")
    # A network failure leaves the previous snapshot untouched.
    (root / "registry/craft-releases.json").write_text(json.dumps({"apps": entries}, ensure_ascii=False, indent=2) + "\n")


def write_release(folder, app, notes):
    assets = {}
    for file in sorted(folder.rglob("*")):
        if not file.is_file() or "licenses" in file.relative_to(folder).parts:
            continue
        path = file.relative_to(folder).as_posix()
        if path in ("index.html", "release.json"):
            continue
        data = file.read_bytes()
        assets[path] = {"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}
    identity = json.dumps({"version": app['appVersion'], "assets": assets}, sort_keys=True, separators=(",", ":"))
    entry = notes.get("apps", {}).get(app['id'])
    if entry and entry["version"] != app['appVersion']:
        entry = None  # Never attach a newer/older release's notes to this version.
    release = {
        "schema": 1, "app": app['id'], "version": app['appVersion'],
        "build": hashlib.sha256(identity.encode()).hexdigest(),
        "releaseNotes": entry,
        "bytes": sum(asset['bytes'] for asset in assets.values()), "assets": assets,
    }
    (folder / "release.json").write_text(json.dumps(release, ensure_ascii=False, indent=2) + "\n")
    return release


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Snapshot ArtCraft notes for the currently packaged app versions")
    parser.add_argument("--refresh", required=True, action="store_true")
    parser.parse_args()
    refresh_notes(Path(__file__).resolve().parent.parent)
