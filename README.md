# subnet.ee — local Craft workspace

One portal and four independent Rust/Wasm editors, served by one certified frontend
canister. Only PhotoCraft currently uses Sovereign Cloud. Its account and file service is a separate
canister; editing and media processing run in the browser.

## Layout

* `portal/`: Estonian launcher, shared sign-in and My files; no editor preloading.
* `apps/{photocraft,pdfcraft,wordcraft,soundcraft}`: pinned Git submodules to your forks.
* `packages/sovereign-cloud/`: internal Rust SDK; canonical source and independent tests.
* `packages/sovereign-cloud-web/`: small Wasm facade for the portal, using the same Rust session and Files SDK as PhotoCraft.
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

Each submodule has `origin = eerikpotter/<app>` and `upstream = storytold/<app>`.
The root commit pins the tested app commit. It does not track a moving upstream branch.
Create/select a branch in a submodule before editing: initialized submodules normally
have detached HEAD. Review and test upstream merges inside the fork, commit there,
then commit the changed submodule pointer in this root. Push app commits before
pushing root pointers when sharing; local-only commits are not available on GitHub.

The shared-account milestone is on `codex/shared-account-files` in the root and PhotoCraft.
The first extraction remains on `codex/subnet-cloud-base`. The other three retain
their fetched fork commits. Each app keeps its own Cargo workspace and lockfile.
The root is not a giant Cargo workspace. Build caches can be shared with
`CARGO_TARGET_DIR`; `SUBNET_RUST_TOOLCHAIN` overrides the compiler when deliberately testing.

## Internal SDK

Edit `packages/sovereign-cloud`, then run `python3 scripts/sync-sdk.py`. The checked-in
PhotoCraft snapshot keeps that fork independently buildable. The build fails if its
snapshot differs. See [the SDK README](packages/sovereign-cloud/README.md).

The SDK owns identity adaptation, configured endpoint resolution, transport, upload
checkpoints, progress and verified downloads. PhotoCraft retains `.pcraft` encoding,
its file menus, naming panel, document revisions and save state. The service preserves
its existing Candid methods, serde shapes, stable-memory IDs and quotas. Its canonical implementation
lives under `services/sovereign-cloud`; `python3 scripts/sync-service.py` updates a checked
snapshot in the standalone PhotoCraft backend. The root owns service changes and deployment;
the PhotoCraft fork remains independently buildable. Do not edit the snapshots directly.

Use the same hostname consistently during local testing. The four app paths share one
browser origin. No production `derivationOrigin` is silently enabled. Moving existing
accounts to subnet.ee needs an explicit identity/data migration. `app_id` is not a credential.

## Checks

```sh
python3 scripts/sync-sdk.py --check
python3 scripts/sync-service.py --check
cargo +1.95.0 test --manifest-path packages/sovereign-cloud/Cargo.toml --locked
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

This base brings all apps into one launch environment without adding cloud features to
the other editors. Full file-manager UI, shared spaces, per-app grants, user-owned
canisters, encrypted storage, alternate blob stores and Cloud Engine migration are later
work. The V1 backend can hold opaque bytes, but does not yet classify other app formats;
do not connect another app without adding file-type metadata and compatibility checks.

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
are not copied. All V1 files are currently described as PhotoCraft format version 1;
a generic typed create API and multi-blob revisions remain a subsequent milestone.

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

File-menu refinement validation (2026-10-10): all 8 web-host tests pass, including actual
menu activation, placement and document guards. The shared UI suite passed 497 tests in
the sandbox; its one GPU-dependent test passed separately with GPU access (3 opt-in tests
ignored). All 27 layer checks, 21 Wasm crate checks and strict Wasm host clippy pass. Native
UI/web all-target clippy passes with the previously documented `collapsible_match` lint
excluded. The release build and local deployment verifier pass. Browser inspection confirmed
the three small menu marks, removed toolbar launcher and all three operation titles in the
signed-out flow; the console had no errors. No cloud upload was needed for this UI change.

Cloud-section validation (2026-10-10): 8 web-host tests and 498 UI tests passed (3 opt-in
tests ignored), including keyboard navigation past the heading and a hidden first action.
Native clippy passed with the existing collapsible_match lint excluded; cloud-host Wasm
clippy, all 27 layering checks and 21 Wasm crate checks passed. Release build, local frontend
sync and full served-asset verification passed. Browser verification confirmed the single
section/mark, disabled save actions without a document, enabled actions with a blank document,
and the signed-out Open dialog. No browser console errors or cloud file writes.
