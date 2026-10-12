#!/usr/bin/env python3
"""Inspect, prepare and explicitly promote Craft upstream updates without running project code."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
APPS = ("photocraft", "pdfcraft", "wordcraft", "deckcraft", "soundcraft", "cadcraft", "gridcraft")
SHA = re.compile(r"[0-9a-f]{40}\Z")


def git(repo, *args, check=True, local=False):
    # No inherited Git injection, user/system aliases, hooks, filters or credential helpers.
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_TERMINAL_PROMPT="0", GIT_ASKPASS="false")
    config = ["core.hooksPath=" + os.devnull, "core.fsmonitor=false",
              "core.attributesFile=" + os.devnull, "core.pager=cat",
              "protocol.allow=never", "protocol.https.allow=always",
              "credential.helper=", "http.followRedirects=false",
              "fetch.recurseSubmodules=false", "submodule.recurse=false",
              "fetch.fsckObjects=true", "transfer.fsckObjects=true",
              "maintenance.auto=false", "gc.auto=0", "rerere.enabled=false",
              "commit.gpgSign=false", "merge.gpgSign=false"]
    if local:
        config.append("protocol.file.allow=always")
    command = ["git", "--no-pager"]
    for value in config:
        command.extend(["-c", value])
    result = subprocess.run(command + list(args), cwd=repo, env=env,
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if check and result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or "Git command failed")
    return result


def out(repo, *args, **kwargs):
    return git(repo, *args, **kwargs).stdout.strip()


def safe_config(repo):
    # Local repository config is not cloned from upstream, but may contain user customizations.
    # Refuse executable drivers rather than silently running them during checkout/merge.
    config = out(repo, "config", "--local", "--no-includes", "--name-only", "--list")
    for key in config.splitlines():
        if (key.startswith(("include.", "includeif.", "filter."))
                or key == "extensions.worktreeconfig" or re.match(r"merge\..*\.driver$", key)):
            raise RuntimeError(f"Executable/included Git configuration requires manual review: {key}")


def clean(repo):
    safe_config(repo)
    if out(repo, "status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError(f"Commit or move local changes first: {repo}")
    if git(repo, "rev-parse", "--verify", "MERGE_HEAD", check=False).returncode == 0:
        raise RuntimeError(f"Finish the existing merge first: {repo}")


def paths(root, app):
    if app not in APPS:
        raise RuntimeError("Unknown application")
    state = root / ".local/craft-updates"
    return root / "apps" / app, state, state / "cache" / (app + ".git")


def load_plan(root, app):
    repo, state, cache = paths(root, app)
    plan = json.loads((state / "plans" / (app + ".json")).read_text())
    if plan.get("app") != app or not all(SHA.fullmatch(plan.get(k, "")) for k in ("base", "upstream")):
        raise RuntimeError("Invalid plan; run check again")
    review = state / "review" / f"{app}-{plan['base'][:12]}-{plan['upstream'][:12]}"
    return repo, cache, review, plan


def check_app(root, app, source=None):
    repo, state, cache = paths(root, app)
    safe_config(repo)
    base = out(repo, "rev-parse", "HEAD")
    if not cache.exists():
        cache.parent.mkdir(parents=True, exist_ok=True)
        git(cache.parent, "init", "--bare", "--template=", str(cache))
    safe_config(cache)
    url = source or f"https://github.com/storytold/{app}.git"
    # source is only used by offline tests; the CLI never accepts arbitrary remote URLs.
    # Seed from the existing checkout first; avoid downloading its history twice.
    git(cache, "fetch", "--no-tags", "--no-recurse-submodules", str(repo), base, local=True)
    git(cache, "fetch", "--no-tags", "--no-recurse-submodules", url,
        "+refs/heads/main:refs/remotes/upstream/main", local=source is not None)
    target = out(cache, "rev-parse", "refs/remotes/upstream/main")
    common = out(cache, "merge-base", base, target)
    changes = out(cache, "diff", "--no-ext-diff", "--no-textconv", "--name-status", common, target)
    incoming_paths = set(out(cache, "diff", "--name-only", "-z", common, target).split("\0")) - {""}
    fork_paths = set(out(cache, "diff", "--name-only", "-z", common, base).split("\0")) - {""}
    overlap = sorted(incoming_paths & fork_paths)
    sensitive = []
    for row in changes.splitlines():
        path = row.split("\t")[-1]
        name = Path(path).name
        if (name in {"Cargo.lock", "Cargo.toml", "build.rs", ".gitmodules", ".gitattributes"}
                or path.startswith((".cargo/", ".github/", "scripts/", "xtask/", "packaging/"))
                or name.startswith(("rust-toolchain", "Trunk", "Dockerfile"))):
            sensitive.append(row)
    plan = dict(app=app, base=base, upstream=target, merge_base=common, source=url,
                checked_at=datetime.now(timezone.utc).isoformat(),
                incoming_commits=int(out(cache, "rev-list", "--count", f"{base}..{target}")),
                fork_commits=int(out(cache, "rev-list", "--count", f"{target}..{base}")),
                overlapping_paths=overlap, fork_changed_files=len(fork_paths),
                review_paths=sensitive)
    folder = state / "plans"
    folder.mkdir(parents=True, exist_ok=True)
    destination = folder / (app + ".json")
    destination.write_text(json.dumps(plan, indent=2) + "\n")
    report = f"{app}: {base} -> upstream {target}\n"
    report += f"Incoming commits: {plan['incoming_commits']}; fork commits: {plan['fork_commits']}\n\n"
    report += out(cache, "diff", "--no-ext-diff", "--no-textconv", "--stat", common, target)
    report += "\n\nBuild/dependency/configuration paths to review (not a security audit):\n"
    report += "\n".join(sensitive) or "(none)"
    report += "\n\nFiles changed by both the fork and upstream (review these first; not necessarily conflicts):\n"
    report += "\n".join(overlap) or "(none)"
    report += "\n\nAll changed paths:\n" + changes + "\n"
    destination.with_suffix(".txt").write_text(report)
    print(f"{app}: {plan['incoming_commits']} upstream commits; report {destination.with_suffix('.txt')}", flush=True)
    return plan


def prepare(root, app):
    repo, cache, review, plan = load_plan(root, app)
    clean(repo)
    if out(repo, "rev-parse", "HEAD") != plan["base"]:
        raise RuntimeError("App HEAD changed since check; run check again")
    safe_config(cache)
    if review.exists():
        raise RuntimeError(f"Review copy already exists; continue there: {review}")
    review.parent.mkdir(parents=True, exist_ok=True)
    git(cache, "worktree", "add", "--detach", str(review), plan["base"])
    # Configure identity only; never copy executable config from the developer's repository.
    for key in ("user.name", "user.email"):
        value = subprocess.run(["git", "config", "--get", key], cwd=repo, text=True,
                               stdout=subprocess.PIPE, check=False).stdout.strip()
        if value:
            git(cache, "config", key, value)
    result = git(review, "merge", "--no-commit", "--no-ff", plan["upstream"], check=False)
    print(f"Review copy: {review}\n{result.stdout}{result.stderr}", flush=True)
    if result.returncode and not out(review, "diff", "--name-only", "--diff-filter=U"):
        raise RuntimeError("Merge failed without file conflicts; inspect the review copy")
    print("Resolve/review, run trusted checks in an appropriate sandbox, then commit in this copy.\n"
          "No build, dependency installation, push or deployment was run.")
    return review


def promote(root, app, commit):
    if not SHA.fullmatch(commit):
        raise RuntimeError("Use the full reviewed 40-character commit SHA")
    repo, cache, review, plan = load_plan(root, app)
    clean(repo)
    clean(review)
    safe_config(cache)
    if out(repo, "rev-parse", "HEAD") != plan["base"]:
        raise RuntimeError("App HEAD changed since preparation; do not overwrite it")
    if not out(repo, "branch", "--show-current"):
        raise RuntimeError("Select a fork branch in the app before promotion")
    if out(review, "rev-parse", "HEAD") != commit:
        raise RuntimeError("Review HEAD differs from the explicitly approved SHA")
    for ancestor in (plan["base"], plan["upstream"]):
        git(review, "merge-base", "--is-ancestor", ancestor, commit)
    git(repo, "fetch", "--no-tags", "--no-recurse-submodules", str(cache), commit, local=True)
    backup = "refs/heads/backup/craft-update-" + plan["base"][:12]
    existing = git(repo, "rev-parse", "--verify", backup, check=False)
    if existing.returncode:
        git(repo, "update-ref", backup, plan["base"], "0" * 40)
    elif existing.stdout.strip() != plan["base"]:
        raise RuntimeError("Backup ref already exists with a different commit")
    git(repo, "merge", "--ff-only", "--no-edit", commit)
    print(f"{app}: promoted {commit}; previous version saved as {backup}\n"
          f"Local build/sync: python3 scripts/dev.py {app} --build\n"
          f"Scoped HTTP check: python3 scripts/verify.py {app}\n"
          "Commit the app pointer in subnet-cloud after validation. Nothing was pushed or deployed.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command")
    inspect = sub.add_parser("check", help="download public upstream objects and write reports only (default)")
    inspect.add_argument("apps", nargs="*", choices=["all", *APPS])
    prep = sub.add_parser("prepare", help="merge pinned commits in an isolated review copy; never build")
    prep.add_argument("app", choices=APPS)
    accept = sub.add_parser("promote", help="fast-forward a clean app to an explicitly reviewed commit")
    accept.add_argument("app", choices=APPS)
    accept.add_argument("--commit", required=True)
    args = parser.parse_args()
    try:
        if args.command in (None, "check"):
            selected = getattr(args, "apps", None) or APPS
            if "all" in selected:
                selected = APPS
            failures = []
            for app in selected:
                try:
                    check_app(ROOT, app)
                except (RuntimeError, OSError, ValueError) as error:
                    failures.append(app)
                    print(f"{app}: {error}", file=sys.stderr, flush=True)
            return bool(failures)
        if args.command == "prepare":
            prepare(ROOT, args.app)
        else:
            promote(ROOT, args.app, args.commit)
    except (RuntimeError, OSError, ValueError) as error:
        print(f"Stopped: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
