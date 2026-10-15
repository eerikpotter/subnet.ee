# Development

[Project overview](../README.md) · [Deployment](deployment.md) · [History](history.md)

Run commands from the repository root. This guide describes the working workflow;
past test results and implementation milestones are recorded in the history.

Keep the root README focused on the project and documentation entry points. Maintain
working instructions in these guides and add dated validation results to the history.

Read [AGENTS.md](../AGENTS.md) before designing or implementing changes. ICP skills
are required at the architecture stage as well as during coding. Record why a
supported ICP integration does or does not meet the requirements before adding
custom infrastructure; see the architecture evidence rules in that file.

## Start locally

Requirements: Rust 1.95.0 with `wasm32-unknown-unknown`, Trunk 0.21.14, Python 3.11+,
Node.js/npm, ICP CLI 1.5+ and `ic-wasm`. Toolchain setup, if needed:

```sh
rustup toolchain install 1.95.0 --profile minimal --target wasm32-unknown-unknown
cargo +1.95.0 install trunk --version 0.21.14 --locked
npm install -g @icp-sdk/icp-cli @icp-sdk/ic-wasm
python3 scripts/setup.py
python3 -m venv .tools/images
.tools/images/bin/python -m pip install -r scripts/image-requirements.txt
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

The frontend's presync hook generates the operator-approved MCP discovery files
using that environment's deployed IDs. Assembly alone does not generate them.
See [MCP setup](mcp.md) and the [isolated MCP smoke test](../experiments/mcp-smoke/README.md)
for the account-authority alpha boundary and local protocol validation.
`icp network stop` stops this project's local network. `icp network start local -d`
restarts it. Keep `.icp/cache/` if you want the local data and network state to survive.

The workspace has its own local canisters and origin. Existing standalone
PhotoCraft mainnet/local deployments are not migrated. Their accounts, files and
canister IDs remain there. The local workflow does not deploy to mainnet or push Git changes.

## Code ownership

Edit portal HTML/CSS/JavaScript in `portal/`, editor metadata in
`registry/apps.json`, shared cloud behavior in `packages/`, and the backend in
`services/sovereign-cloud/`. `dist/` is assembled output.

### Landing-page images

Keep full-resolution originals in `portal/assets/`; never overwrite them with web
exports. Assembly runs `scripts/images.py` before localization, using the pinned
Pillow environment installed above (or the active Python environment when it has
the dependency). Builds require no image service or network after installation.

Add `data-image-widths="640,1280,1920"` and an accurate `sizes` attribute to a
template's `<img>` to generate up to three WebP candidates. Widths are capped at
the original, never upscaled. Choose widths for the image's rendered size: the
hero cloud uses 320/640/960 and the campaign preview 320/500/750. A phone with a
high-density screen can choose the middle size; these are pixel-width candidates,
not fixed device labels. `data-image-preload` generates an exactly matching
responsive preload; use it only for above-the-fold imagery. Keep lower images lazy.

The EFTI footer logo is displayed at 32 CSS pixels and gets one 96-pixel WebP for
all devices. The shared subnet PNG is capped at 192 pixels in the assembled output
for its small icon/brand uses. The original campaign PNG remains downloadable.
Generated WebP filenames contain content hashes and receive immutable caching at
`/images/`; `image-assets.json` records dimensions and byte sizes for verification.
Everything generated stays in ignored `dist/`. For image changes, run:

```sh
.tools/images/bin/python -m unittest discover -s scripts -p test_images.py
python3 -m unittest discover -s scripts -p test_localize.py
python3 -m unittest discover -s scripts -p test_scoped_workflow.py
python3 scripts/assemble.py
```

Check the rendered result and network requests at mobile, tablet and desktop widths,
including a high-density phone. After a local sync, `python3 scripts/verify.py`
checks generated images, their WebP media types and immutable cache headers.

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

The landing page and cloud drive are prerendered in Estonian at `/` and `/pilveruum/`,
and in English at `/en/` and `/en/pilveruum/`. Each URL serves complete, indexable HTML
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

## User-controlled Craft updates

The shared launcher serves a complete cached app by default and checks only the
small current-release manifest for updates. The version button shows the current
and available app versions, a short build ID (versions alone can be identical),
and ArtCraft release notes when there is a matching published release. Downloading
an update selects it for future launches; the current editor keeps running until
**Open updated version** is selected. That action requires a known clean document.
Other open tabs retain their version and their work.
The version popup closes on an outside click, Escape or its corner close button.
When the app is current, it has no footer action; update actions appear only when
a different server package is available.

The server contains **only the latest package** for each app, never old app binaries.
Each `apps/<app>/release.json` describes that package's files, sizes and SHA-256 hashes.
The browser stores verified complete packages in Cache Storage and a selected-build
pointer. The pointer changes only after all files have verified successfully. A
partial/failed download leaves the previous selection intact. The worker serves
virtual `/apps/<app>/_cached/<build>/` URLs exclusively from local storage; those URLs
return 404 on the server. A missing old asset never falls back to a newer asset.
Web Locks serialize installs and keep packages used by open tabs from being cleaned
up. Obsolete packages are removed after they are no longer selected or in use.

Clearing site/app Cache Storage (or browser eviction) causes the next launch to use
the latest server package. Clearing only the ordinary HTTP cache may leave Cache
Storage intact; browser controls differ. The first launch after adopting this feature
also downloads the latest package: a pre-existing HTTP cache is not a verified complete
app package. Browsers without service workers, Cache Storage or Web Locks use the
latest server editor and show that version retention is unavailable.

Only public editor assets are cached. The shared launcher, release checks, identity
configuration, cloud API and picker stay live. Cached HTML retains its CSP, while
the uncached launcher receives the current `ic_env` cookie. Backend authorization
still decides cloud access on every request. This is version choice, not offline
cloud synchronization or permanent browser storage. An older editor may not support
a document format introduced later; adopting an upstream release still requires the
normal application/format/cloud compatibility review.

### Release notes

Notes come **only from ArtCraft's published GitHub release for the exact app version**.
There are no subnet-authored release notes and no changelog generated from arbitrary
commits. `registry/craft-releases.json` is a snapshot, fetched explicitly after
assembling the adopted app versions:

```sh
python3 scripts/craft_release.py --refresh
python3 scripts/assemble.py
```

The refresh reads public `storytold/<app>/releases/tags/v<version>` metadata, rejects
drafts or mismatched tags/URLs, and leaves the previous snapshot intact on a network
failure. A missing release yields no notes. Assembly never contacts GitHub and never
attaches another version's notes. The dialog shows a short inert-text excerpt and a
link to the full publisher notes in their original language. Notes are for the target
release, not an invented comparison across all skipped versions. Editing notes does
not change a package's build ID or trigger an app update.

On 2026-10-15 the packaged versions were Photo 0.8.0, Word/Deck/Grid/Sound 0.6.0,
PDF 0.7.0 and CAD 0.6.1. GitHub had no exact published releases for these versions
(the next published patch versions were newer). The initial snapshot is therefore
empty and the dialog says that notes are unavailable. Do not substitute newer notes.

### Architecture evidence and validation

Status: implemented and locally tested; production deployment is a separate step.
The session's ICP [static-site skill](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
was consulted on 2026-10-15 before implementation. Its existing certified-assets
recipe remains the supported hosting solution. Its revalidated HTML/metadata and
immutable fingerprinted files are retained. The gap is selecting a *complete older
browser package* when the server only has the latest: ordinary HTTP caching cannot
inventory or select that package reliably. The smallest extension is one shared
browser package store, one narrow service worker and one update controller, plus
assembly-time metadata. No new canister, authentication layer, app fork dependency
or server archive was added. Version-specific bootstrap helpers live inside each
package; the shared launcher stays current.

Focused tests cover whole-package integrity, failed/corrupt/truncated updates,
incomplete/cleared caches, app isolation, live-tab retention, cloud-request exclusion,
launch parameters, explicit switching, dirty/unknown documents and edits made during
an asynchronous cache read. Browser QA uses real native Craft engines: remove A's
binaries from a local test server after caching A, publish B, reopen A, choose B,
retain an unsaved older tab, then clear the app cache and verify a fresh launch uses
latest. Local proof artifacts are in `.local/craft-cache-qa/`. See the dated history
for executed checks and their limits.

Browser references: [MDN Cache API](https://developer.mozilla.org/en-US/docs/Web/API/Cache),
[Web Locks](https://developer.mozilla.org/en-US/docs/Web/API/Web_Locks_API),
and [storage quotas/eviction](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria).

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

The shared Open/Save As dialog embeds the portal's selection mode, including
`My files` and `Shared with me`; do not implement a separate folder tree or
permission policy inside an editor. Selection messages contain IDs and a name,
never document bytes or authority. The SDK verifies the selected file's format
and current access, then uses the canonical drive transfer methods. Create permits
a new file in the selected folder; Edit permits a new revision of an existing
file. Read-only documents remain editable locally and can be copied with Save As.
App-native serialization stays in each adapter. A shared UI change must be adopted
and smoke-tested with a real save/reopen in all seven affected editors.

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
