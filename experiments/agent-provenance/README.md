# Internet Identity MCP provenance experiment

This isolated workspace tests the identity boundary of the II MCP delegation flow.
It is not linked into the cloud service and never touches its deployed canisters.
All tested calls use real signed HTTP ingress through ic-agent; PocketIC only
installs disposable canisters and controls the local network. No caller is injected
into a tested application or II call.

## Observed on 14 October 2026

All four checks passed with II `release-2026-10-13`, PocketIC 16.0.0,
ic-agent 0.49.2, ic-cdk 0.20.3 and Rust 1.95.0:

- Browser app delegation and full MCP app delegation expose the same principal,
  no caller-info signer and empty caller-info data. The MCP delegation can invoke
  an ordinary user-authorized edit endpoint.
- Query-only MCP delegation permits a query under that same principal, with no
  additional caller-info, but the replica rejects an update.
- A distinct signing principal, registered by the user in the probe, supports
  narrower access, user sponsorship, denied edits/administration and immediate
  application-level revocation.
- Disabling MCP in II stops new delegation issuance; a previously issued full
  delegation still works before its expiry.

The separate-principal control is a possible application architecture, not an
implemented imcp2 connection option. The experiment does not exercise a hosted
MCP transport, Claude/ChatGPT client, interactive passkey ceremony or UI consent.
Those require separate integration tests. It does not prove that no other ICP
mechanism can carry the required context.

## Reproduce

Download the official
[II backend artifact](https://github.com/dfinity/internet-identity/releases/download/release-2026-10-13/internet_identity_backend.wasm.gz)
and an appropriate [PocketIC 16 server](https://github.com/dfinity/pocketic/releases/tag/16.0.0).
Decompress the PocketIC executable and keep its required filename `pocket-ic` or
`pocket-ic-server`. The runner requires explicit artifact paths and does not download
executables. It checks the compressed II artifact SHA-256 against:

```text
a0e242ae7c802b688cfe18977878a56701eb92576aa75fef27da7a001b5a430c
```

From the repository root:

```sh
cargo +1.95.0 build --manifest-path experiments/agent-provenance/Cargo.toml --locked -p agent-provenance-probe --release --target wasm32-unknown-unknown
cargo +1.95.0 build --manifest-path experiments/agent-provenance/Cargo.toml --locked -p agent-provenance-runner
```

Set `POCKET_IC_BIN` to that server, `II_WASM` to the compressed II artifact, and
`PROBE_WASM` to `experiments/agent-provenance/target/wasm32-unknown-unknown/release/agent_provenance_probe.wasm`
(use absolute paths). Then run:

```sh
experiments/agent-provenance/target/debug/agent-provenance-runner
```

The environment must permit binding loopback HTTP ports. Missing files and failed
assertions are errors, never skipped successes. Four `PASS` lines and exit code 0
establish the reported result. Fixed test signing seeds are public fixtures, never
production credentials. The probe intentionally uses disposable heap state.

Delegation signatures use `DelegatedIdentity::new_with_root_key`; clients receive
their trusted root directly from the PocketIC fixture and never call `fetch_root_key`.
PocketIC's own local gateway may log that it fetches its test network root key;
that is local test infrastructure, not production client configuration.

## Related primary sources

- [ICP service-discoverability skill](https://skills.internetcomputer.org/skills/service-discoverability/)
- [ICP agent-web-identity skill](https://skills.internetcomputer.org/skills/agent-web-identity/)
- [II MCP source inspected](https://github.com/dfinity/internet-identity/blob/2f175eb56a339458618e218f8862fa121a621502/src/internet_identity/src/mcp.rs)
- [imcp2 identity source inspected](https://github.com/dfinity/imcp2/blob/355d8deb181c5cf0486de5c3ad8be93c5f2dfa76/crates/imcp2-core/src/identities.rs)
- [Earlier scoped delegation discussion](https://forum.dfinity.org/t/adding-parameters-to-delegated-identity/33071)

## Actual cloud permission boundary

Set `CLOUD_WASM` to the built `sovereign-cloud-service` Wasm to additionally run
`runner/src/cloud.rs` against a fresh disposable canister. Build it first:

```sh
cargo +1.95.0 build --manifest-path services/sovereign-cloud/Cargo.toml --locked --release --target wasm32-unknown-unknown
```

This optional suite passed on 2026-10-14. Alice creates a folder/file and grants
Bob Read + Create. Bob's normal II app delegation and II-issued MCP delegation
both create and read files, with Alice's ownership/quota and Bob's account
attribution. Both are denied Edit, Delete and access outside the share. Files and
grants survive an actual canister upgrade. Revoking Bob denies reads and creation
retries for both credentials. All tested user calls use signed HTTP ingress.

This adds a fifth `PASS` line. It still does not exercise JSON-RPC MCP transport,
hosted discovery, provider setup or the browser/Craft UI. Those remain separate
release gates. No manifest or operator agreement is published by this experiment.
