# Standard ICP MCP → actual cloud canister

This harness adds a test-only module to a **disposable DFINITY imcp2 checkout** at
`355d8deb181c5cf0486de5c3ad8be93c5f2dfa76` (imcp2-core 0.5.1). Its original Cargo.lock
is unchanged: rmcp 2.2.0, ic-agent 0.49.0 and PocketIC 15.0.0. No imcp2 dependency is
added to the application. The upstream tool implementation is unchanged.

## Run

From the subnet-cloud repository, build the actual service with public Candid:

```sh
RUSTUP_TOOLCHAIN=1.95.0 icp build cloud
python3 -m unittest discover -s scripts -p test_mcp_discovery.py
python3 -m unittest discover -s scripts -p test_scoped_workflow.py
python3 experiments/mcp-smoke/site-check.py
```

`site-check.py` creates a temporary project and two local canisters with the official
recipes. It exercises actual presync environment injection, the assembly-generated
headers, certified JSON discovery, local identity fallback and 404 behavior, then
stops its local network. It does not touch the main project's canisters or publish
anything to mainnet. The site content is a tiny assembly fixture, not a Craft UI test.

Prepare a fresh upstream checkout and the matching
[PocketIC 15 server](https://github.com/dfinity/pocketic/releases/tag/15.0.0).
Keep the executable's basename `pocket-ic`. For example, with the repository at
`/path/to/subnet-cloud` and the disposable checkout at `/tmp/imcp2-cloud-test`:

```sh
git clone https://github.com/dfinity/imcp2 /tmp/imcp2-cloud-test
git -C /tmp/imcp2-cloud-test checkout --detach 355d8deb181c5cf0486de5c3ad8be93c5f2dfa76
python3 /path/to/subnet-cloud/experiments/mcp-smoke/prepare.py /tmp/imcp2-cloud-test
cd /tmp/imcp2-cloud-test
POCKET_IC_BIN=/path/to/pocket-ic \
SUBNET_REPO=/path/to/subnet-cloud \
CLOUD_WASM=/path/to/subnet-cloud/.icp/cache/artifacts/cloud \
cargo +1.95.0 test --locked -p imcp2-core --features e2e subnet_mcp_smoke -- --nocapture
```

The test fails if its required inputs are absent; it does not silently skip. All
canisters and identities are disposable. The root key comes from the local fixture,
not a network `fetch_root_key` call. PocketIC's gateway performs its own local-only
root-key setup and emits a warning about it.

## Evidence and limits

The test uses a real MCP JSON-RPC byte stream and negotiated initialization, lists
the tools, discovers the installed canister's actual public Candid/API document,
and creates/reads a file through signed HTTP ingress. Alice owns a folder and file;
Bob's connection is initially denied. Alice grants Read + Create, Bob creates a
file through MCP, Alice sees it, and bytes and account attribution are checked.
Repeated creation is idempotent. Edit/Delete/outside-folder actions fail. Revocation
blocks subsequent reads and creation retries. An undeclared canister is refused by
the connector. Discovery consumes this repository's generated manifest and origin.

Two boundaries use upstream's test-only facilities: public web retrieval (loopback
is correctly refused by its SSRF protections), and a genuine signed delegation
issued by a disposable key instead of Internet Identity. There is no synthetic
caller injection into the operations under test. The separate
[agent-provenance runner](../agent-provenance/README.md) tests real II issuance and
the actual cloud service with signed HTTP calls.

These tests complement one another. They do **not** establish a hosted OAuth/browser
round trip, production deployment, Claude/ChatGPT compatibility, or completed Craft
UI flows. The JSON-RPC transport here is a byte stream, not hosted Streamable HTTP.
