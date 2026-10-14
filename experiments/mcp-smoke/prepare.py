#!/usr/bin/env python3
"""Add this test to a disposable, exact-revision DFINITY imcp2 checkout only."""
from pathlib import Path
import shutil
import subprocess
import sys

REVISION = "355d8deb181c5cf0486de5c3ad8be93c5f2dfa76"

if len(sys.argv) != 2:
    raise SystemExit("usage: prepare.py /path/to/disposable/imcp2-checkout")
checkout = Path(sys.argv[1]).resolve()
actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=checkout, text=True).strip()
if actual != REVISION:
    raise SystemExit(f"Expected DFINITY imcp2 {REVISION}, got {actual}")
source = checkout / "crates/imcp2-core/src"
library = source / "lib.rs"
declaration = '\n#[cfg(all(test, feature = "e2e"))]\nmod subnet_mcp_smoke;\n'
contents = library.read_text()
original = subprocess.check_output(["git", "show", "HEAD:crates/imcp2-core/src/lib.rs"], cwd=checkout, text=True)
if contents not in (original, original + declaration):
    raise SystemExit("lib.rs has unrelated changes; use a disposable checkout")
changed = subprocess.check_output(["git", "diff", "HEAD", "--name-only"], cwd=checkout, text=True).splitlines()
if set(changed) - {"crates/imcp2-core/src/lib.rs"}:
    raise SystemExit("Upstream tracked sources or dependencies have changed; use a clean disposable checkout")
if declaration not in contents:
    library.write_text(contents + declaration)
shutil.copyfile(Path(__file__).with_name("subnet_mcp_smoke.rs"), source / "subnet_mcp_smoke.rs")
print(f"Prepared test-only module in {checkout}; production imcp2 code/dependencies unchanged")
