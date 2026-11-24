#!/usr/bin/env python3
"""Fetch only the exact translation revisions pinned by the selected applications."""
import argparse
import json
from craft_inputs import ROOT, prepare

apps = tuple(app["id"] for app in json.loads((ROOT / "registry/apps.json").read_text()))
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("apps", nargs="+", choices=["all", *apps])
args = parser.parse_args()
for app in apps if "all" in args.apps else args.apps:
    prepare(ROOT / "apps" / app)
