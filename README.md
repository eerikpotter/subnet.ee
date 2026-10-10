# subnet.ee — local Craft workspace

One portal and five independent Rust/Wasm editors, served by one certified frontend
canister. All five editors use the same Sovereign Cloud UI and SDK. The account and file service is a separate
canister; editing and media processing run in the browser.

## Layout

* `portal/`: Estonian launcher, shared sign-in and My files; no editor preloading.
* `apps/{photocraft,pdfcraft,wordcraft,deckcraft,soundcraft}`: pinned Git submodules to your forks.
* `packages/sovereign-cloud/`: internal Rust SDK; canonical source and independent tests.
* `packages/sovereign-cloud-ui/`: shared File-menu section, sign-in, naming, versions and save/open controller.
* `packages/sovereign-cloud-web/`: small Wasm facade for the portal, using the same Rust session and Files SDK as every Craft editor.
* `services/sovereign-cloud/`: canonical backend source, accounts and paginated file catalog; V1 API preserved.
* `registry/apps.json`: app metadata, current cloud capability and portal content.
* `scripts/`: reproducible setup, build, SDK vendoring, assembly and local verification.
* `icp.yaml`: one frontend plus the shared account/file backend (same local canister ID).
* `dist/`: generated deployment artifact; each app lives at `/apps/<id>/`.

## Start locally

Requirements: Rust 1.95.0 with `wasm32-unknown-unknown`, Trunk 0.21.14, Python 3,
ICP CLI 1.5+ and `ic-wasm`. Toolchain setup, if needed:

```sh
rustup toolchain install 1.95.0 --profile minimal --target wasm32-unknown-unknown
cargo +1.95.0 install trunk --version 0.21.14 --locked
npm install -g @icp-sdk/icp-cli @icp-sdk/ic-wasm
python3 scripts/setup.py
python3 scripts/dev.py --build
python3 scripts/verify.py
```

The dev command deploys **only to local**. It prints the actual URL; never assume a fixed
gateway port. Later runs without `--build` reuse editor outputs. To change just one app:

```sh
python3 scripts/build.py photocraft
python3 scripts/dev.py
```

After editing the portal Rust facade, run `python3 scripts/build.py portal`.
After editing only the HTML/CSS/JavaScript portal, `python3 scripts/dev.py` reassembles and serves it.
`icp network stop` stops this project's local network. `icp network start local -d`
restarts it. Keep `.icp/cache/` if you want the local data and network state to survive.

The new workspace has its own local canisters and origin. Existing standalone
PhotoCraft mainnet/local deployments are not migrated. Their accounts, files and
canister IDs remain there. No mainnet deployment or remote push is part of this setup.

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

The shared-account milestone is on `codex/shared-account-files` in the root and PhotoCraft.
The other four app integrations are on `codex/sovereign-cloud` in their forks.
The root remote is `https://github.com/eerikpotter/subnet.ee.git`.
The first extraction remains on `codex/subnet-cloud-base`. Each app keeps its own Cargo workspace and lockfile.
The root is not a giant Cargo workspace. Build caches can be shared with
`CARGO_TARGET_DIR`; `SUBNET_RUST_TOOLCHAIN` overrides the compiler when deliberately testing.

## Internal SDK

Edit `packages/sovereign-cloud`, then run `python3 scripts/sync-sdk.py`. The checked-in
snapshots keep every fork independently buildable. The build fails if any snapshot differs. See [the SDK README](packages/sovereign-cloud/README.md).

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

```sh
python3 scripts/sync-sdk.py --check
python3 scripts/sync-service.py --check
cargo +1.95.0 test --manifest-path packages/sovereign-cloud/Cargo.toml --locked
cargo +1.95.0 test --manifest-path packages/sovereign-cloud-ui/Cargo.toml --locked
cargo +1.95.0 clippy --manifest-path packages/sovereign-cloud/Cargo.toml --all-targets --locked -- -D warnings
cargo +1.95.0 test --manifest-path services/sovereign-cloud/Cargo.toml --lib --locked
cargo +1.95.0 test --manifest-path apps/photocraft/icp/Cargo.toml -p photocraft-cloud --lib --locked
python3 scripts/verify.py
```

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

All five editors now share cloud storage and compatible format metadata. Shared spaces,
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
