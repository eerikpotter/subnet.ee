# subnet.ee — Craft workspace

One portal and seven independent Rust/Wasm editors, served by one certified frontend
canister. All seven editors use the same Sovereign Cloud UI and SDK. The account and file service is a separate
canister; editing and media processing run in the browser.

## Layout

* `portal/`: Estonian/English public landing page at `/`; the launcher, shared sign-in and My files live at `/tooruum/`. No editor preloading on the landing page.
* `apps/{photocraft,pdfcraft,wordcraft,deckcraft,soundcraft,cadcraft,gridcraft}`: pinned Git submodules to your forks.
* `packages/sovereign-cloud/`: internal Rust SDK; canonical source and independent tests.
* `packages/sovereign-cloud-ui/`: shared File-menu section, sign-in, naming, versions and save/open controller.
* `packages/sovereign-cloud-web/`: small Wasm facade for the portal, using the same Rust session and Files SDK as every Craft editor.
* `services/sovereign-cloud/`: canonical backend source, accounts and paginated file catalog; V1 API preserved.
* `registry/apps.json`: app metadata, current cloud capability and portal content.
* `scripts/`: reproducible setup, build, SDK vendoring, assembly and local verification.
* `icp.yaml`: one frontend plus the shared account/file backend (same local canister ID).
* `dist/`: generated deployment artifact; each app lives at `/apps/<id>/`.

## Languages

The landing page and workspace are prerendered in Estonian at `/` and `/tooruum/`,
and in English at `/en/` and `/en/tooruum/`. Each URL serves complete, indexable HTML
with a canonical URL and reciprocal `hreflang` links. Canonicals use the mapped mainnet
frontend origin; set `SUBNET_PUBLIC_ORIGIN` when a custom domain is configured.

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

## Start locally

Requirements: Rust 1.95.0 with `wasm32-unknown-unknown`, Trunk 0.21.14, Python 3,
ICP CLI 1.5+ and `ic-wasm`. Toolchain setup, if needed:

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

The new workspace has its own local canisters and origin. Existing standalone
PhotoCraft mainnet/local deployments are not migrated. Their accounts, files and
canister IDs remain there. The local workflow does not deploy to mainnet or push Git changes.

## Mainnet trial (2026-10-12)

The mainnet trial uses two new canisters, separate from all local data and existing
standalone Craft deployments:

* Frontend: `vigkv-fqaaa-aaaac-bgeha-cai`, https://vigkv-fqaaa-aaaac-bgeha-cai.icp.net/
* Accounts/files: `vbfbj-tyaaa-aaaac-bgegq-cai`.

Both are controlled by the existing `efti-nns` identity. Mainnet mappings are in
`.icp/data/mappings/ic.ids.json`; retain them so later updates reuse these canisters.
This trial does not configure the `subnet.ee` custom domain.

For a portal-only mainnet update, assemble the current portal and reuse its packaged
editors, then sync assets explicitly to `ic`:

```sh
python3 scripts/assemble.py
icp sync frontend -e ic --identity efti-nns
python3 scripts/verify.py portal --environment ic
```

`python3 scripts/verify.py all --environment ic --timings` also downloads and verifies
all seven editor bundles, reporting compressed HTTP transfer sizes and durations.
These durations measure individual HTTP requests, not browser startup or cache hits.
The landing page does not preload editor bundles. Each editor is fetched when opened;
its fingerprinted JavaScript/Wasm uses a one-year immutable cache. HTML and unversioned
portal assets revalidate so updates are picked up. Cache rules must not combine
`no-cache` with the fingerprinted bundles' immutable policy.

The initial mainnet verification passed for all seven editors, including payload
hashes, certified responses, runtime canister IDs/root key, cache headers and 404s.
PdfCraft and SoundCraft also started in the browser. Initial gzip Wasm transfers
were 6.1–12.7 MB and took 3.6–24.8 seconds in one run; these are variable HTTP
measurements, not complete browser startup times. A separate WordCraft Brotli
request transferred 7.4 MB in 7.3 seconds; its conditional repeat returned 304
with no body. The landing page loads no editor Wasm.

## Git and updates

Use `python3 scripts/update-crafts.py` to download upstream commits and produce reports
without changing or building the apps. `prepare` merges pinned commits in a separate
review copy; `promote --commit <sha>` accepts a reviewed result. See
[scripts/UPDATING.md](scripts/UPDATING.md) for the workflow, rollback and security boundaries.

Each submodule has `origin = eerikpotter/<app>` and `upstream = storytold/<app>`.
The root commit pins the tested app commit. It does not track a moving upstream branch.
Create/select a branch in a submodule before editing: initialized submodules normally
have detached HEAD. Review and test upstream merges inside the fork, commit there,
then commit the changed submodule pointer in this root. Push app commits before
pushing root pointers when sharing; local-only commits are not available on GitHub.

The current portal and all seven app integrations are published on each fork's `main`.
Earlier integration milestones remain on `codex/shared-account-files` in the root and
PhotoCraft, and `codex/sovereign-cloud` in the other six forks.
The root remote is `https://github.com/eerikpotter/subnet.ee.git`.
The first extraction remains on `codex/subnet-cloud-base`. Each app keeps its own Cargo workspace and lockfile.
The root is not a giant Cargo workspace. Build caches can be shared with
`CARGO_TARGET_DIR`; `SUBNET_RUST_TOOLCHAIN` overrides the compiler when deliberately testing.
Give each repository's `xtask` its own target directory, or run `cargo clean -p xtask`
before switching repositories: their identically named binaries embed the workspace path.

Upstream refresh (2026-10-12): the fork merges retain both upstream history and the
existing subnet integration. Reviewed upstream tips were PhotoCraft `bb17dd555c16`,
PdfCraft `87b448b664e1`, WordCraft `fc52079f587a`, DeckCraft `0ec0cc5e815c`,
SoundCraft `df0c4790b729`, CADCraft `4cb9e8a535b2` and GridCraft `b3acae6b7782`.
GridCraft's cloud actions now live in its new Backstage view. WordCraft uses upstream's
document identity API; SoundCraft keeps its portable cloud format alongside upstream's
new local file handling. CADCraft and PdfCraft retain their new upstream host bridges.
All seven forks retain the same pinned Sovereign Cloud SDK and UI source snapshots.
All seven release Wasm builds pass. The 25 focused tests pass: PhotoCraft web-host/menu
tests (13), WordCraft native-format/document identity (3), SoundCraft portable audio
format (5), CADCraft cloud adapter (1), and GridCraft cloud adapter/Backstage wiring (3).
All seven browser cloud save/open checks pass using a disposable identity on the local
test network, including edited image, text, spreadsheet, CAD and PDF content and the
slide/audio demos. No browser console errors were observed. Temporary session fixtures
and their generated redirects were removed; `python3 scripts/verify.py all` passes for
the final local bundle, including exact asset payloads, certification, cache/CSP headers,
runtime trust and missing-file responses. These checks do not cover Internet Identity
issuance or constitute a deployment of this upstream refresh to mainnet.

Strict editor-wide CI is not green on Rust 1.95.0: it stops at upstream Clippy warnings
in CADCraft `geom/nurbs.rs`, GridCraft's XLSX code/tests, DeckCraft's ISOBMFF codec,
WordCraft `proof/affix.rs`, SoundCraft's video codec and PdfCraft `forms/af.rs`.
PhotoCraft's scoped Wasm Clippy check likewise stops at `drop_non_drop` in upstream
`format/store.rs` when checking the browser target.
These files are unchanged by the cloud integration; the refresh does not suppress
their lints or modify unrelated editor behavior. PhotoCraft's 29-crate layer check passes.

## Internal SDK

Edit the canonical packages, then deliberately adopt them in an affected fork with
`python3 scripts/sync-sdk.py photocraft`. Use `all` only for an intentional fleet-wide
SDK update. Each fork keeps its own checked-in snapshot; builds verify that snapshot's
integrity without requiring it to match the latest canonical version. `--check` compares
with canonical sources; `--check-pinned` checks the fork's existing snapshot. See
[the SDK README](packages/sovereign-cloud/README.md).

The SDK owns identity adaptation, configured endpoint resolution, transport, upload
checkpoints, progress and verified downloads. The shared UI owns the menu actions and dialogs;
small editor adapters retain native encoding, document revisions and save state. The service preserves
its existing Candid methods, serde shapes, stable-memory IDs and quotas. Its canonical implementation
lives under `services/sovereign-cloud`; `python3 scripts/sync-service.py` updates a checked
snapshot in the standalone PhotoCraft backend. The root owns service changes and deployment;
the PhotoCraft fork remains independently buildable. Do not edit the snapshots directly.

Use the same hostname consistently during local testing. All app paths share one
browser origin. No production `derivationOrigin` is silently enabled. Moving existing
accounts to subnet.ee needs an explicit identity/data migration. `app_id` is not a credential.

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

The dated validation notes below record past milestones, not a checklist to repeat on
every change.

PhotoCraft's existing browser, layer, ownership and persistence tests remain applicable.
`sdk_migration` additionally takes `PHOTOCRAFT_BASELINE_WASM` and `PHOTOCRAFT_CLOUD_WASM`
to test old-backend data through the extracted SDK after upgrade, including a dropped
commit response. These tests create isolated disposable PocketIC canisters.

Initial validation (2026-10-10): all four release Wasm builds succeeded. 25 focused
tests passed: SDK configuration/transfer (8), backend unit tests (5), PocketIC
persistence/upgrade tests (4), and PhotoCraft web-host unit tests (8). Clippy passed
for the SDK (native and Wasm), backend and PhotoCraft web host; PhotoCraft's layer
check passed for all 27 crates. The deployed local site passed `scripts/verify.py`,
including exact asset bytes, certification-header presence, MIME/cache/CSP rules,
runtime trust configuration and missing-file responses.

All four editors were opened from the portal in the browser without console errors;
PhotoCraft's Sovereign Cloud sign-in panel rendered. The portal was checked at desktop
and 390px mobile width without horizontal overflow. At that initial milestone, authenticated
browser save/reopen had not yet been exercised in this new origin; see the shared-account
validation below. Editor-wide functionality
and mobile editor ergonomics are outside this setup's smoke check.

## Next boundaries

All seven editors now share cloud storage and compatible format metadata. Shared spaces,
per-app grants, user-owned canisters, encrypted storage, alternate blob stores and
Cloud Engine migration remain later work.

Build output includes each project's license and attribution notices. Portal artwork is
original HTML/CSS. Root and SDK changes use MIT OR Apache-2.0; upstream notices remain.

## Shared account and PhotoCraft flow (0.2)

Sign in on the portal, open PhotoCraft and use **File → Sovereign Cloud → Save**. The same
account's committed files appear under **Minu failid**. Opening a row passes a logical
service/space/file reference to PhotoCraft; the backend rechecks ownership and the SDK
checks file integrity. Further saves keep the file ID and add revisions. The catalog
only exposes committed files and reads via consensus-backed updates.

The File menu has one **Sovereign Cloud** section after the local save block: **Save**,
**Save As…**, and **Open…**. One ICP mark appears beside the section heading. Save As
creates a new cloud file and links subsequent cloud saves to it. Open shows the file picker
without leaving the document. There is no separate cloud toolbar button, and the cloud
dialog uses a compact header.

`get_or_create_account`, `list_files` and `get_file` extend the existing API. Accounts
and their file index use new stable-memory IDs 6 and 7. V1 schemas/IDs 0–5 remain intact.
A legacy owner's catalog is indexed once when their account is first resolved (the
old store is bounded to 1000 files). New files update that same index. The account and
personal-space IDs are stable within the logical `cloud` service. Existing file bytes
are not copied. V1 files remain PhotoCraft format version 1; the typed create API described below
adds the other editors. Multi-blob revisions remain future work.

The portal and PhotoCraft share the pinned Rust AuthClient storage adapter. A loaded
session carries a fingerprint of its persisted credentials. Each SDK network call
rejects changed/expired sessions; UI responses are scoped to their originating session.
Cross-tab changes restore identity without reloading the editor. Open documents retain
their original account binding; saving one to another account requires **Sovereign Cloud → Save As**.
Already submitted calls may complete, and logout does not revoke previously issued
IC delegations at the replica. This is one trusted first-party origin, not per-app isolation.

The facade is built separately from the editors. `scripts/verify.py` verifies its served
Wasm/JS too and rejects the temporary browser fixtures. `scripts/prepare-browser-test.py`
creates opt-in loopback-only fixtures with disposable direct-signing identities for UI
QA. Use an isolated canister hostname, never a real user's origin. They are excluded from
normal assembly and must be removed by reassembling/syncing before final delivery. These
fixtures exercise real signed canister calls, but do not test Internet Identity issuance.

Shared-account validation (2026-10-10): SDK tests (9), service tests (6), PocketIC
persistence/upgrade tests (4), standalone backend tests (6), and PhotoCraft host tests (8)
pass. Clippy passes for the SDK, service, portal facade and Wasm PhotoCraft host. All 27
PhotoCraft dependency-layer checks pass; release builds pass for the changed Wasm packages.
The old-Wasm upgrade test preserves file IDs and content, indexes legacy files, appends
a revision and denies another caller access.

Browser QA used two disposable signing identities on an isolated local canister hostname.
A painted PhotoCraft document appeared in the portal, reopened with the same content via
its catalog link, and saved a second revision under the same file ID. Switching accounts
cleared file lists without reloading or losing the document; saving that document to the
other account required an explicit copy. Portal logout propagated to PhotoCraft. Corrupt
persisted delegation JSON recovered to a signed-out state without a browser error. The
file list was visually checked at desktop and 390px width. Real Internet Identity/passkey
issuance and cross-device use remain interactive validation steps; the browser driver did
not expose the provider popup. No mainnet deployment was made.

The final local deployment passed `scripts/verify.py` for the portal and all four editors,
including exact served bytes, runtime configuration, response headers and 404 responses for
both removed browser fixtures. The regular frontend origin was reopened in its signed-out
**Minu failid** view after cleanup.

## Shared Craft navigation

Every `/apps/<id>/` entry point uses the same 40px subnet.ee bar: **← subnet.ee**,
the application's name, and **Minu failid**. `portal/craft-shell.html`, `.css` and
`.js` own the navigation. Assembly renders the name from `registry/apps.json` and
moves each original Trunk entry point to `editor.html` beside its unchanged assets.
A same-origin iframe gives the editor the remaining viewport without covering its
menus. Launch query parameters and fragments are forwarded, including cloud-file
references and renderer options. No other editor is preloaded.

The browser host in each Rust app publishes `data-craft-dirty="true"` or `"false"`
on its document element after both logic and UI updates. PhotoCraft, PdfCraft and DeckCraft
aggregate all open documents; WordCraft reports its session; SoundCraft includes
both dirty state and active recording. This tiny host contract exposes no document
or account data and adds no dependency on the portal to standalone builds.

The common shell reads that state synchronously before navigation. Clean documents
leave immediately; unsaved work opens an Estonian confirmation with **Jätka tööd**
and **Lahku salvestamata**. Cancel/Escape keeps the editor loaded. The shell also
requests the browser's native warning on reload, Back, or tab closure when dirty;
browser restrictions still apply to native unload prompts. Missing/unknown state
uses a conservative save reminder. Explicitly opening a portal link in another tab
does not interrupt the document. The dialog does not claim to save files itself.

To integrate another Craft, register its metadata/build and publish the same dirty
attribute from its web host. The navigation, layout and guard are shared unchanged.
After host changes rebuild the affected apps; shell-only edits need reassembly.

Navigation checks: `node --test scripts/craft-shell.test.mjs` covers both destinations,
cancel/confirm, fresh save/edit state, unknown state, modifier clicks, history restore,
and launch references. `python3 scripts/verify.py` checks served shell assets, editor
entry points, original Wasm/JS assets, CSP and runtime configuration.

File-menu refinement validation (2026-10-10): all 8 web-host tests pass, including actual
menu activation, placement and document guards. The shared UI suite passed 497 tests in
the sandbox; its one GPU-dependent test passed separately with GPU access (3 opt-in tests
ignored). All 27 layer checks, 21 Wasm crate checks and strict Wasm host clippy pass. Native
UI/web all-target clippy passes with the previously documented `collapsible_match` lint
excluded. The release build and local deployment verifier pass. Browser inspection confirmed
the three small menu marks, removed toolbar launcher and all three operation titles in the
signed-out flow; the console had no errors. No cloud upload was needed for this UI change.

Navigation validation (2026-10-10): all four release Wasm builds and scoped web-host
Clippy checks pass, along with 7 shared-shell tests, 8 PhotoCraft web-host tests and
the 27-crate PhotoCraft layer check. Browser QA exercised a painted PhotoCraft image,
a typed WordCraft document, a new PdfCraft PDF and a modified SoundCraft demo. Each
reported dirty state and showed the shared leave confirmation; Cancel preserved the
editor. PhotoCraft local Save cleared the guard and allowed immediate navigation to
My files. Escape, explicit discard, clean navigation, and the 390px shell/dialog
layout were also checked. No mainnet deployment was made.

Cloud-section validation (2026-10-10): 8 web-host tests and 498 UI tests passed (3 opt-in
tests ignored), including keyboard navigation past the heading and a hidden first action.
Native clippy passed with the existing collapsible_match lint excluded; cloud-host Wasm
clippy, all 27 layering checks and 21 Wasm crate checks passed. Release build, local frontend
sync and full served-asset verification passed. Browser verification confirmed the single
section/mark, disabled save actions without a document, enabled actions with a blank document,
and the signed-out Open dialog. No browser console errors or cloud file writes.

The final managed-local deployment passed `scripts/verify.py` for the shared shell,
all four wrapped entry points, their original editor pages, and exact Wasm/JS bytes.
The final PhotoCraft page also rendered on the certified local origin with a clean
dirty-state report. Concurrent asset syncs required rechecking the final served
bundle; the completed server contents match `dist/`.

## DeckCraft

DeckCraft is registered at `/apps/deckcraft/`, with the same shared navigation and
leave guard. Its browser host reports unsaved changes across all open presentations.
The standalone editor's file picker/download flow supports `.deckcraft`, PPTX and
PDF export; Sovereign Cloud stores the editable `.deckcraft` format. The upstream sample
opens by default, and `?blank` starts a new presentation.

`python3 scripts/build.py deckcraft --assemble` builds just DeckCraft and reuses the
other editor outputs. The build selection and portal counts come from
`registry/apps.json`, so adding an editor does not require updating fixed counts.

DeckCraft's ordinary browser download/save path is unchanged from upstream. A local
save-state fix was removed to keep this fork focused on integration. Cloud completion
marks only its captured, unchanged presentation clean. Formatting, asset audit and
the 27-crate layer check pass. Upstream's full baseline CI stops at the existing
`collapsible_match` lint in `deckcraft-matroska`; that unrelated codec is unchanged.

## Shared cloud in all Craft editors

All five web hosts build with `icp-cloud` and vendor the same SDK and UI sources.
Use File → Sovereign Cloud → Save, Save As… or Open…. WordCraft exposes the section
as a File backstage page. My files routes each format/version to its compatible editor.
PhotoCraft stores `.pcraft`, PdfCraft `.pdf`, WordCraft `.wcraft.json`, DeckCraft
`.deckcraft`, and SoundCraft a portable `.scraft.zip` containing session metadata and
lossless planar float sources. Linked videos are rejected by SoundCraft's portable writer instead
of silently dropping media. The service's existing 64 MiB file limit still applies.

Metadata for new formats occupies stable memory 8; memory IDs 0–7 and existing file
bytes are unchanged. Missing kind metadata defaults to the legacy PhotoCraft format.
The old create/list API remains PhotoCraft-compatible; new typed create/list methods
serve the other editors. Ownership is still enforced by the caller, independently of
format metadata. Upload completion marks only the captured, unchanged revision clean.
Single-document editors confirm before replacing unsaved work with a cloud file.

Validation for the shared cloud implementation: SDK transfer/configuration tests,
shared UI tests, service unit tests and isolated PocketIC persistence/upgrade tests
pass. Native format tests preserve a WordCraft document with an inserted picture
and passthrough XML; SoundCraft tests preserve the editable demo and exact float
headroom. New/Open document identities are distinct from edits. Strict SDK, shared
UI (native/Wasm), service and SoundCraft engine Clippy checks pass. WordCraft engine
Clippy exposes a pre-existing `overly_complex_bool_expr` in `cmd/para.rs:277`; it passes
with only that existing lint excluded. Full workspace CI is not claimed.

WordCraft's media-preserving JSON codec is used only by its cloud adapter; ordinary
local JSON open/save remains upstream-owned. SoundCraft's portable bundle is likewise
a separate cloud format, without replacing upstream local session saves.

Browser checks on the isolated local QA origin saved and reopened a DeckCraft sample,
a WordCraft document, a blank PdfCraft document and SoundCraft's synthesized demo
(63.1 MB including its audio). Each appeared in the shared catalog with the matching
editor link, while in-app pickers only listed compatible formats. WordCraft's second
save kept the same file ID and produced revision 2; cancelling replacement of unsaved
text kept the edits, and successful save cleared the shared navigation warning.
These use disposable test identities and real calls to the local replica, not real
Internet Identity/passkey issuance or mainnet.

## Reviewed PhotoCraft update

PhotoCraft's merge `b2fd7337` incorporates upstream `7722172585a0` (0.6.0).
The follow-up includes upstream `2b79c7c25dca` (nine additional test lines) and
the fork's existing `main` merge, preserving both histories.
The update contained 887 upstream commits and 11 conflicted files. Conflicts included
manifests/lockfile, attribution, README, startup and menu plumbing, rather than 11 new
cloud features. Dependency versions from the incoming lockfile were preserved; cloud-only
dependencies were added. The generic menu hook now adds only 10 lines to upstream's
`menus.rs`; its native service initializer and layer checker match upstream exactly.
The service commands and submenu heading renderer live in a separate module. Cloud UI
and transport remain shared, with generated snapshots for independently buildable forks.

The merge passed 13 web-host tests, the host-command guard test, Wasm checks with
`icp-cloud,heif`, and the 29-crate layer check. Scoped Wasm Clippy passes with upstream's
existing `collapsible_match` and `nonminimal_bool` lints excluded. Strict Clippy reports
seven instances of those lints in unchanged upstream code; this is not a full green
upstream CI run. The updater's five offline integration tests pass, including a conflict
that leaves the working app untouched and exact-commit promotion with a backup ref.

The other upstream updates have only been fetched for inspection; they were not merged.
Future reviews should normally select one app at a time. Fetching does not build or deploy
anything. Neither a small diff nor passing tests establishes that upstream code is safe;
see [the update guide](scripts/UPDATING.md) before running an unfamiliar build.

### CADCraft and GridCraft cloud adapters

The catalog now includes CADCraft (drawings) and GridCraft (spreadsheets). Both use
our shared shell, File → Sovereign Cloud menu, account and versioned file service.
Cloud Save / Save As / Open use upstream's DXF codec (`cadcraft.dxf`, version 1)
and complete JSON workbook codec (`gridcraft.scjson`, version 1), respectively.
Ordinary local saves and exports remain upstream-owned. Cloud files reopen in new
tabs, preserving existing work. GridCraft commits/validates a cell still being typed
before requesting a cloud save. An upload only marks its exact document revision
clean; newer edits and unfinished text remain protected by the navigation guard.

The fork integration is confined to the browser host and an optional File-menu
callback; the storage protocol and UI are generated vendor snapshots. Both apps
are included in `scripts/update-crafts.py`. Rebuild these two apps with
`python3 scripts/build.py cadcraft gridcraft --assemble`; other editor outputs are reused.
