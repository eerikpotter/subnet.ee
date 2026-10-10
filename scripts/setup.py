#!/usr/bin/env python3
"""Initialize pinned submodules and their upstream remotes; never advances a pin."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
if __name__ == "__main__":
    subprocess.run(["git", "submodule", "update", "--init", "--recursive"], cwd=ROOT, check=True)
    for app in json.loads((ROOT / "registry/apps.json").read_text()):
        repo = ROOT / "apps" / app['id']
        remotes = subprocess.check_output(["git", "remote"], cwd=repo, text=True).splitlines()
        if "upstream" not in remotes:
            subprocess.run(["git", "remote", "add", "upstream", f"https://github.com/storytold/{app['id']}.git"], cwd=repo, check=True)
    print("Pinned apps ready. Select/create a branch inside an app before editing it.")
