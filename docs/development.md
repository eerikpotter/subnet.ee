# Development

[Project overview](../README.md) · [Deployment](deployment.md) · [History](history.md)

Run commands from the repository root. This guide describes the working workflow;
past test results and implementation milestones are recorded in the history.

Keep the root README focused on the project and documentation entry points. Maintain
working instructions in these guides and add dated validation results to the history.

## Start locally

Requirements: Rust 1.95.0 with `wasm32-unknown-unknown`, Trunk 0.21.14, Python 3,
Node.js/npm, ICP CLI 1.5+ and `ic-wasm`. Toolchain setup, if needed:

```sh
rustup toolchain install 1.95.0 --profile minimal --target wasm32-unknown-unknown
cargo +1.95.0 install trunk --version 0.21.14 --locked
npm install -g @icp-sdk/icp-cli @icp-sdk/ic-wasm
python3 scripts/setup.py
python3 scripts/dev.py all --build --bootstrap
python3 scripts/verify.py all
```

The dev command deploys **only to local**. It prints the actual URL; never assume a fixed
gateway port. The full build/deployment above is for first-time setup only. Later runs
reuse the packaged editors in `dist/`; ordinary updates never deploy the backend.

For portal HTML/CSS/JavaScript, app cards or the shared header:

```sh
python3 scripts/dev.py
python3 scripts/verify.py
```

This assembles the portal and syncs frontend assets, without Cargo, editor checkout
inspection or editor downloads during verification. To change just one app:

```sh
python3 scripts/dev.py photocraft --build
python3 scripts/verify.py photocraft
```

Only PhotoCraft is built and its own pinned SDK snapshot checked. Other apps retain
their existing binaries and source metadata. For a build without deployment, use
`python3 scripts/build.py photocraft --assemble`; plain `build.py photocraft` only builds.
After editing the portal Rust facade, use `python3 scripts/dev.py portal --build`.
Backend changes use the separate command `icp deploy cloud -e local` after focused service checks.
`all` must be explicit in build, assembly and verification commands. No command here
automatically runs the Craft repositories' test suites.
`icp network stop` stops this project's local network. `icp network start local -d`
restarts it. Keep `.icp/cache/` if you want the local data and network state to survive.

The workspace has its own local canisters and origin. Existing standalone
PhotoCraft mainnet/local deployments are not migrated. Their accounts, files and
canister IDs remain there. The local workflow does not deploy to mainnet or push Git changes.

## Code ownership

Edit portal HTML/CSS/JavaScript in `portal/`, editor metadata in
`registry/apps.json`, shared cloud behavior in `packages/`, and the backend in
`services/sovereign-cloud/`. `dist/` is assembled output.

Editor forks own their document formats and editing behavior. Cloud adapters
connect those formats to the shared SDK and UI; preserve the editors' local
save/export behavior when changing an integration. The registry records each
editor's cloud format and version.

Each app keeps its own Cargo workspace and lockfile.
The root is not a giant Cargo workspace. Build caches can be shared with
`CARGO_TARGET_DIR`; `SUBNET_RUST_TOOLCHAIN` overrides the compiler when deliberately testing.
Give each repository's `xtask` its own target directory, or run `cargo clean -p xtask`
before switching repositories: their identically named binaries embed the workspace path.

## Internal SDK

Edit the canonical packages, then deliberately adopt them in an affected fork with
`python3 scripts/sync-sdk.py photocraft`. Use `all` only for an intentional fleet-wide
SDK update. Each fork keeps its own checked-in snapshot; builds verify that snapshot's
integrity without requiring it to match the latest canonical version. `--check` compares
with canonical sources; `--check-pinned` checks the fork's existing snapshot. See
[the SDK README](../packages/sovereign-cloud/README.md).

The SDK owns identity adaptation, configured endpoint resolution, transport, upload
checkpoints, progress and verified downloads. The shared UI owns the menu actions and dialogs;
small editor adapters retain native encoding, document revisions and save state. The service preserves
its existing Candid methods, serde shapes, stable-memory IDs and quotas. Its canonical implementation
lives under `services/sovereign-cloud`; `python3 scripts/sync-service.py` updates a checked
snapshot in the standalone PhotoCraft backend. The root owns service changes and deployment;
the PhotoCraft fork remains independently buildable. Do not edit the snapshots directly.

Use the same hostname consistently during local testing. All app paths share one
browser origin. On mainnet, `www.subnet.ee` explicitly uses this frontend canister's
origin for Internet Identity, preserving accounts created on its gateway address.
Accounts belonging to the previously replaced subnet.ee canister are a separate
identity/data migration. `app_id` is not a credential.

## Languages

The landing page and workspace are prerendered in Estonian at `/` and `/tooruum/`,
and in English at `/en/` and `/en/tooruum/`. Each URL serves complete, indexable HTML
with a canonical URL and reciprocal `hreflang` links. Canonical URLs use
`https://www.subnet.ee` by default; set `SUBNET_PUBLIC_ORIGIN` to override it.

Both translations are embedded in HTML attributes. ET/EN links switch text and labels
in place without fetching, reloading, or resetting account/file-list state. Browser
back/forward and fragments work; without JavaScript the same links open static pages.
Legacy `?lang=en` / `?lang=et` links are normalized without a reload. Public paths select
the language; the stored `subnet.language` preference is only a fallback for app wrappers.
App URLs keep `?lang=` alongside cloud file references and renderer flags.

`portal/translations.js` maps authored Estonian UI text to English; dynamic messages use
`SubnetI18n.text(element, source, values)` or `bind(element, render)` so current states
repaint on language changes. `t()` formats strings and `locale` is a live getter. Add a
translation when changing UI copy; `scripts/localize.py` prerenders it during assembly. Translation only touches text and
label attributes; user file names/content and the already-English Craft editors stay intact.
Language changes never reload an open editor. Run `node --test scripts/i18n.test.mjs
scripts/cloud-i18n.test.mjs scripts/craft-shell.test.mjs scripts/craft-loading.test.mjs`
and `python3 -m unittest discover -s scripts -p test_localize.py` for the locale,
state-preservation, static HTML and shared-shell checks.

## Git and updates

Use `python3 scripts/update-crafts.py` to download upstream commits and produce reports
without changing or building the apps. `prepare` merges pinned commits in a separate
review copy; `promote --commit <sha>` accepts a reviewed result. See
[scripts/UPDATING.md](../scripts/UPDATING.md) for the workflow, rollback and security boundaries.

Each submodule has `origin = eerikpotter/<app>` and `upstream = storytold/<app>`.
The root commit pins the tested app commit. It does not track a moving upstream branch.
Create/select a branch in a submodule before editing: initialized submodules normally
have detached HEAD. Review and test upstream merges inside the fork, commit there,
then commit the changed submodule pointer in this root. Push app commits before
pushing root pointers when sharing; local-only commits are not available on GitHub.

## Checks

Choose checks for the changed boundary. Portal layout needs a visual check; navigation
logic and startup use `node --test scripts/craft-shell.test.mjs scripts/craft-loading.test.mjs scripts/craft-wasm.test.mjs`. One editor update needs its build,
relevant adapter checks and a save/open smoke check in that editor. SDK behavior changes
need the affected package tests; backend behavior changes need the affected service tests.
Run wider checks only for a shared contract change that can affect multiple consumers.

The common shell reports decoded Wasm package bytes and percentage, then verification
and startup, until the editor publishes its first `data-craft-dirty` state. Package
size and SHA-384 come from the assembled artifact; compressed Content-Length is not
used as the denominator. The bootstrap fetches the original versioned URL once,
preserves HTTP caching and compiles the stream while downloading. Its SHA-384 check
gates stream completion, and compilation returns a module without executing it;
only then does the original Wasm binding instantiate it. This replaces the Wasm
preload's buffered integrity check; JavaScript module preloads stay intact.
Slow startup offers a retry after 20 seconds without treating it as a failure;
startup errors offer recovery, and a running editor cannot be replaced by a late retry.

`verify.py` makes HTTP requests to the local site; it never compiles or runs Craft tests.
Its default checks the portal/shared files. Pass one app to check that editor's served
HTML/JS/Wasm, or explicit `all` for a full deployment audit. Shared metadata checks still
ensure all registered app links exist. Script isolation regressions run offline with
`python3 -m unittest discover -s scripts -p test_scoped_workflow.py`.

PhotoCraft's existing browser, layer, ownership and persistence tests remain applicable.
`sdk_migration` additionally takes `PHOTOCRAFT_BASELINE_WASM` and `PHOTOCRAFT_CLOUD_WASM`
to test old-backend data through the extracted SDK after upgrade, including a dropped
commit response. These tests create isolated disposable PocketIC canisters.

## Editor integration

Each `/apps/<id>/` entry point uses `portal/craft-shell.html`, `.css` and `.js`.
Assembly places the original Trunk page at `editor.html` alongside its assets;
the shared shell hosts it in a same-origin iframe and forwards launch parameters
and fragments. Editors load when opened.

Each browser host publishes `data-craft-dirty="true"` or `"false"` after state
changes. The shell uses that state for navigation and unload warnings; multi-document
editors must include all open documents. Shell-only changes need assembly; host
changes need a build of the affected editor.

Cloud Save updates the linked file; Save As creates a new file. An upload may mark
only its captured, unchanged document revision clean. Account changes must preserve
unsaved documents, discard stale UI responses, and require Save As to copy a document
to another account. All editors share one trusted browser origin.

The portal facade and editor clients share the Rust session adapter. A loaded session
is checked against persisted credentials before each SDK request. Logout prevents
new client requests but does not revoke already-issued IC delegations at the replica.

For browser QA, `scripts/prepare-browser-test.py` creates opt-in, loopback-only
fixtures with disposable signing identities. Use an isolated canister hostname.
Reassemble and sync without the fixtures before final verification; `verify.py`
rejects them. These checks exercise real local canister calls, but Internet Identity
issuance requires a separate interactive check.

For protocol and storage details, see the [SDK](../packages/sovereign-cloud/README.md),
[shared UI](../packages/sovereign-cloud-ui/README.md) and
[backend](../services/sovereign-cloud/README.md) documentation.
