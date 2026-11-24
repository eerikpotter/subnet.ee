"""Pinned, data-only translation inputs used by recent Craft browser builds."""
import importlib.util
from pathlib import Path
import re
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / ".local/craft-inputs"


def translation_input(repo):
    pin = repo / "craft-i18n.rev"
    if not pin.is_file():
        return None
    revision = pin.read_text().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise RuntimeError(f"Invalid translation revision in {pin}")
    return revision, CACHE / "craft-i18n" / revision


def configure(repo, env):
    # An inherited directory must not silently replace the application's pin.
    env.pop("CRAFT_I18N_DIR", None)
    pinned = translation_input(repo)
    if pinned:
        revision, directory = pinned
        if not directory.is_dir():
            raise RuntimeError(f"Missing craft-i18n {revision}; run python3 scripts/prepare-craft-inputs.py {repo.name}")
        env["CRAFT_I18N_DIR"] = str(directory)


def prepare(repo):
    pinned = translation_input(repo)
    if not pinned:
        return
    revision, directory = pinned
    if directory.is_dir():
        return
    # Reuse the update workflow's restricted public-Git transport and config check.
    spec = importlib.util.spec_from_file_location("craft_updater", ROOT / "scripts/update-crafts.py")
    updater = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(updater)
    cache = CACHE / "craft-i18n.git"
    CACHE.mkdir(parents=True, exist_ok=True)
    if not cache.exists():
        updater.git(CACHE, "init", "--bare", "--template=", str(cache))
    updater.safe_config(cache)
    updater.git(cache, "fetch", "--no-tags", "--no-recurse-submodules",
                "https://github.com/storytold/craft-i18n.git", revision)
    directory.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=directory.parent) as temp:
        temp = Path(temp)
        archive = temp / "input.tar"
        updater.git(cache, "archive", "--format=tar", "--output=" + str(archive), revision)
        extracted = temp / "input"
        with tarfile.open(archive) as source:
            source.extractall(extracted, filter="data")
        extracted.rename(directory)
    print(f"Prepared craft-i18n {revision}", flush=True)
