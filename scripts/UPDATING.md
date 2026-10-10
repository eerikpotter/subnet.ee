# Updating the Craft forks

Run these commands from the `subnet-cloud` root with Python 3 and a maintained Git installation.
The updater uses only the Python standard library and Git. It never invokes Cargo, Trunk,
package managers, repository scripts, deployment tools or push. Keep this script itself under review.

## 1. Download and inspect (safe default)

```sh
python3 scripts/update-crafts.py
# Or one application:
python3 scripts/update-crafts.py check photocraft
```

This fetches **public** `https://github.com/storytold/<app>.git`, branch `main`, into
`.local/craft-updates/cache/`. It does not change the app branches, files, root submodule
pins or the running site. Reports and exact base/upstream commit IDs are written to
`.local/craft-updates/plans/<app>.{json,txt}`. Local uncommitted changes are permitted
for this inspection; commit them before preparing a merge. Review the complete diff,
not just the highlighted build/dependency/configuration paths. The report is not a malware scan.

The allowlist is in the updater, separate from the website registry. Network fetches
use HTTPS only, without credential helpers, redirects, hooks, automatic maintenance,
submodule recursion or inherited Git configuration. Local object transfers allow the
file protocol only for known workspace paths. Executable filters, custom merge drivers
and config includes in a repository cause the script to stop. Keep Git patched: parsing
untrusted Git objects still uses Git and cannot be made risk-free by a Python wrapper.

## 2. Prepare a merge without changing the working app

```sh
python3 scripts/update-crafts.py prepare photocraft
```

The command requires a clean application checkout and unchanged HEAD since inspection.
It creates a detached review worktree in `.local/craft-updates/review/`, merges the two
**pinned** commits with `--no-commit`, and stops. Conflicts remain in that review copy.
Your normal app checkout and its branch stay unchanged. A worktree is an organizational
boundary, **not an operating-system security sandbox**.

Review dependencies (`Cargo.toml`, `Cargo.lock`), `build.rs`, proc macros, `.cargo/`,
Trunk hooks, shell scripts, workflows, new executables and submodules before executing
anything. Do not blindly accept either side of a conflict or regenerate dependency locks
to the newest versions. Preserve the reviewed lockfile versions wherever compatible.
Our integration is primarily in `apps/<app>-web/src/cloud*` plus small host/menu hooks.
The canonical cloud packages live in `packages/`; their vendored snapshots are generated
by `scripts/sync-sdk.py`, not maintained independently in five apps.

If code or dependencies are not yet trusted, build and test in a disposable VM/container
with no host home/SSH/cloud credentials, Docker socket or unrelated workspace mounts,
and no network during the build after fetching dependencies. A container with sensitive
host mounts does not provide that protection. The updater intentionally does not pretend
that a normal local Cargo build is sandboxed. `--locked` prevents dependency resolution
drift; it does not prevent build scripts or procedural macros from executing.

After review, run the repository's documented checks and the root shared SDK/service
checks. Check the browser cloud save/open and unsaved-work navigation in the local deployment.
Then commit the resolved merge **inside the printed review directory**:

```sh
git -c core.hooksPath=/dev/null -C /absolute/review/path add <resolved-files>
git -c core.hooksPath=/dev/null -C /absolute/review/path commit -m 'Merge reviewed upstream main'
git -C /absolute/review/path rev-parse HEAD
```

If no update is needed, stop after the report. Existing review directories are never
overwritten. Keep them until the merge has been promoted and backed up; use Git's normal
worktree commands for deliberate cleanup. `prepare` does not initialize upstream submodules.

## 3. Promote the exact reviewed result

```sh
python3 scripts/update-crafts.py promote photocraft --commit FULL_40_CHARACTER_SHA
```

This requires clean checkouts, the unchanged original app HEAD, and the exact committed
review HEAD containing both the original fork commit and the selected upstream commit.
It preserves `backup/craft-update-<old-sha>` and fast-forwards the current app branch.
A detached app must be put on a fork branch first. The script cannot determine whether
you actually reviewed or tested a commit; `--commit` is that explicit handoff.

After building and validating the result, commit the updated `apps/photocraft` pointer
in `subnet-cloud`. Push the fork branch before the parent pointer. Push/deploy remain
explicit separate actions. The root pins commits, never moving upstream branches.
Rollback is an explicit checkout/pointer change to the backup commit; the updater never
uses destructive reset, automatic stash, forced push or silent conflict resolution.

## Tests

```sh
python3 -m unittest discover -s scripts -p test_update_crafts.py
```

The offline tests use temporary repositories. They cover conflict isolation, unchanged
working copies, clean-tree and stale-HEAD protection, exact-SHA promotion, backup refs,
preservation of fork changes, hook suppression and refusal of executable Git filters.

References: [Git fetch](https://git-scm.com/docs/git-fetch),
[Git hooks](https://git-scm.com/docs/githooks),
[Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html).

## Keep future merges small

Reports list paths changed by both upstream and our fork, so likely integration hotspots
are visible before a merge. Overlap predicts review work, not the number of actual conflicts.
Keep provider UI/transport in the canonical packages, app serialization in a small host adapter,
and generic menu hooks in a separate module. Avoid reformatting upstream files or copying
upstream menu rendering into our integration. PhotoCraft's merged menu currently needs only
10 added lines in the upstream `menus.rs`; recursive menu signatures remain unchanged.
Its native service initializer and layer checker need no fork changes. A future upstream
extension API could remove more of these hooks; contributing that API upstream would be a
separate, deliberate PR. Updating regularly limits the size of each review but does not prove
that an update is trustworthy or eliminate manifest/lockfile conflicts.

Do not fix unrelated Craft behavior in these integration forks. Keep local save/export,
rendering and editing behavior upstream-owned; report problems upstream separately.
The earlier DeckCraft local-save fix was removed for this reason. Necessary cloud format
adapters should live in separate modules and touch existing code only through small hooks.
Also check the Trunk HTML entry point: an explicit `data-cargo-features` overrides the CLI
feature selection. Its feature list must retain `icp-cloud` alongside upstream features
(PhotoCraft currently also needs `heif`). Verify the cloud menu in the resulting browser build.
