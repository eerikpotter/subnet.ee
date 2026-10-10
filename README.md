# subnet.ee — local Craft workspace

One portal and four independent Rust/Wasm editors, served by one certified frontend
canister. Only PhotoCraft currently uses Sovereign Cloud. Its file service is a separate
canister; editing and media processing run in the browser.

## Layout

* `portal/`: lightweight Estonian launch page, HTML/CSS only; no app preloading.
* `apps/{photocraft,pdfcraft,wordcraft,soundcraft}`: pinned Git submodules to your forks.
* `packages/sovereign-cloud/`: internal Rust SDK; canonical source and independent tests.
* `registry/apps.json`: app metadata, current cloud capability and portal content.
* `scripts/`: reproducible setup, build, SDK vendoring, assembly and local verification.
* `icp.yaml`: one frontend plus the existing PhotoCraft file backend.
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

After editing only the portal, `python3 scripts/dev.py` reassembles and serves it.
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

The first PhotoCraft refactor is on `codex/subnet-cloud-base`. The other three retain
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
its existing Candid methods, serde shapes, stable-memory IDs and quotas. Its implementation
remains under `apps/photocraft/icp/backend` for this first extraction; there is one authority
for storage code. Its deployment is now composed by the root independently of the editor build.

Use the same hostname consistently during local testing. The four app paths share one
browser origin. No production `derivationOrigin` is silently enabled. Moving existing
accounts to subnet.ee needs an explicit identity/data migration. `app_id` is not a credential.

## Checks

```sh
python3 scripts/sync-sdk.py --check
cargo +1.95.0 test --manifest-path packages/sovereign-cloud/Cargo.toml --locked
cargo +1.95.0 clippy --manifest-path packages/sovereign-cloud/Cargo.toml --all-targets --locked -- -D warnings
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
and 390px mobile width without horizontal overflow. A real Internet Identity sign-in
and authenticated browser save/reopen have **not** been exercised in this new origin;
the transfer and upgrade tests use isolated test identities. Editor-wide functionality
and mobile editor ergonomics are outside this setup's smoke check.

## Next boundaries

This base brings all apps into one launch environment without adding cloud features to
the other editors. Shared file UI, service-scoped spaces, per-app grants, user-owned
canisters, encrypted storage, alternate blob stores and Cloud Engine migration are later
work. The V1 backend can hold opaque bytes, but does not yet classify other app formats;
do not connect another app without adding file-type metadata and compatibility checks.

Build output includes each project's license and attribution notices. Portal artwork is
original HTML/CSS. Root and SDK changes use MIT OR Apache-2.0; upstream notices remain.
