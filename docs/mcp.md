# AI access through ICP MCP

The alpha uses [DFINITY's ICP MCP connector](https://internetcomputer.org/icp-mcp/).
subnet.ee exposes ordinary signed canister methods; it does not run a separate MCP
server or store an agent's private keys. Implementation and test status is recorded
in [collaboration-progress.md](collaboration-progress.md). Production activation
was completed on 2026-10-14 at `https://www.subnet.ee`. Hosted ChatGPT/Internet Identity
end-to-end acceptance remains a separate interactive check.

## Connect after deployment

1. Add `https://mcp.internetcomputer.org/mcp` as a remote MCP connector in a compatible
   assistant. Follow the [official connection instructions](https://internetcomputer.org/icp-mcp/)
   for the current client setup.
2. Connect with the same Internet Identity and application account used in subnet.ee.
3. Choose **Actions & questions** and a session duration. In this alpha, private
   metadata reads are consensus update calls, so Questions-only cannot browse the
   drive; it can read the public API reference.
4. Give the assistant the application URL, `https://www.subnet.ee`, and the task.

An initial demo prompt:

> Open https://www.subnet.ee and read its API documentation. Create a folder named
> “AI demo” in my personal cloud space. Inside it, create hello.txt containing
> “Hello from my assistant”, then read it back and report the file ID.

The initial API supports folder/file creation, listings, reads, sharing,
rename/move and Trash/restore. Native Craft document editing depends on that
application's format and commands. The root drive supports shared replacement uploads
and the shared-folder UI. Proposals and shared Craft editor integration remain unfinished.

## Alpha permission boundary

An authenticated MCP connection has the **same application permissions as its
account**, including ownership administration and deletion. There is no separate
agent folder selector or agent-specific permission reduction yet. A shared folder
granted to Bob is available to Bob's MCP connection with exactly Bob's rights;
Alice can revoke that shared access in the backend. History identifies the account,
not whether a human or AI used the credential.

II and connector session controls govern connection lifetime. Disconnecting stops
new client use; it does not retroactively erase downloaded data or invalidate a
delegation already issued until its expiry. Application sharing revocation is
checked on subsequent operations, including retries.

## Deployment contract

`icp.yaml` uses the official static-site recipe's `presync` hook to run
`scripts/mcp-discovery.py` after canister IDs exist. It generates:

- `/.well-known/ic-architecture`: exactly the current environment's cloud API.
- `/.well-known/ii-derivation-origin` on mainnet: the existing frontend-canister
  origin, preserving the browser's application identity on the custom domain.

Local development omits the identity pointer and uses the visible origin. Switching
environments removes a stale mainnet pointer. Build/assembly does not generate a
manifest; a sync does. Missing/invalid canister IDs abort the sync before upload.
The Rust recipe embeds public `candid:service`; `get_api_doc` documents actual API
behavior and limitations. No third-party canister is declared.

The operator authorized the opt-in on 2026-10-14 after being presented with the
[App Operator Terms v1.1](https://internetcomputer.org/icp-mcp/app-operator-terms/),
effective 2026-09-02. Production deployment is separate. Deploy the reviewed backend
before publishing discovery so its interface and API documentation match.

After deployment, `scripts/verify.py` checks the served manifest, certification,
headers, environment IDs and agreement with the browser's identity configuration.
The [MCP smoke tests](../experiments/mcp-smoke/README.md) run against disposable local
canisters; hosted authentication and individual assistant compatibility still need
an interactive check after deployment.
