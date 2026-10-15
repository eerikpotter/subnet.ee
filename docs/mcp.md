# AI access through ICP MCP

The alpha uses [DFINITY's ICP MCP connector](https://internetcomputer.org/icp-mcp/).
subnet.ee exposes ordinary signed canister methods; it does not run a separate MCP
server or store an agent's private keys. Implementation and test status is recorded
in [collaboration-progress.md](collaboration-progress.md). Production activation
was completed on 2026-10-14 at `https://www.subnet.ee`. Hosted ChatGPT/Internet Identity
end-to-end acceptance remains a separate interactive check.

## Choose an identity

Open **Work with AI** in the cloud space's left menu. Setup is optional and has its
own `#ai` view, outside My files, Shared with me, Trash and the editor file picker.

**Recommended: a separate Internet Identity for the agent.** This uses existing
account sharing to limit the agent's access to your files:

1. Create a separate Internet Identity and sign in to the Subnet cloud space with it.
   Signing in initializes the recipient account required by folder sharing.
2. Copy its **Subnet account ID (principal)** from Work with AI or the file-view
   footer. This is the principal for this application, not an II identity number or
   a principal copied from another app. The setup page labels the current account;
   it cannot determine whether the user considers that identity their agent.
3. Sign out and sign back in with your usual identity. Open a folder's **Share**
   dialog, paste the agent's account ID and grant only what it needs: Read is
   included; Create, Edit and Delete are optional. Permissions inherit to contents.
4. Connect ICP MCP with the **agent's identity**, as described below. Verify that
   the connector's principal for Subnet matches the ID you shared with, including
   the selected application account and Subnet's declared derivation origin.

To remove access, revoke sharing at the folder where it was granted. Grants are
additive: check parent folders and separate grants as well. Revocation cannot take
back files already downloaded. The agent still has full authority over its own
separate account, and any additional access granted to that account.

**Simpler: use your own Internet Identity.** Connect with the same II and Subnet
application account you normally use. No extra identity or sharing setup is needed,
but the agent has your account's permissions, including deletion and sharing
administration wherever you have them. This option does not restrict it to selected
folders.

## Connect ICP MCP

1. Add `https://mcp.internetcomputer.org/mcp` as a remote MCP connector in a compatible
   assistant. Follow the [official connection instructions](https://internetcomputer.org/icp-mcp/)
   for the current client setup.
2. Connect with the identity chosen above: the separate agent identity for limited
   shared access, or your own identity for your account's full access.
3. Choose **Actions & questions** and a session duration. In this alpha, private
   metadata reads are consensus update calls, so Questions-only cannot browse the
   drive; it can read the public API reference.
4. Give the assistant the application URL, `https://www.subnet.ee`, and the task.

An initial demo prompt:

> Open https://www.subnet.ee and read its API documentation. Show my Subnet principal
> so I can compare it with the account ID I shared with. List the folders shared
> with this account and their permissions.

After granting Create on a test folder, ask the assistant to create `hello.txt`
there, read it back and report its file ID.

The initial API supports folder/file creation, listings, reads, sharing,
rename/move and Trash/restore. Native Craft document editing depends on that
application's format and commands. The root drive supports shared replacement uploads
and the shared-folder UI. Proposals and shared Craft editor integration remain unfinished.

## Alpha permission boundary

An authenticated MCP connection has the **same application permissions as its
account**, including ownership administration and deletion over its own files.
There is no agent-specific permission reduction within the same account. A shared folder
granted to Bob is available to Bob's MCP connection with exactly Bob's rights;
Alice can revoke that shared access in the backend. History identifies the account,
not whether a human or AI used the credential.

A separate identity makes the accounts distinguishable in history; it does not
prove that a particular action came from AI. The restriction on the owner's files
comes from the ordinary recipient's grants, not from a new agent credential type.

II and connector session controls govern connection lifetime. Disconnecting stops
new client use; it does not retroactively erase downloaded data or invalidate a
delegation already issued until its expiry. Application sharing revocation is
checked on subsequent operations, including retries.

## Setup decision — 2026-10-15

Status: implemented locally; hosted two-identity MCP sign-in remains an interactive
acceptance check. Requirement: move optional setup into navigation and explain both
full account access and recommended, folder-limited access through a separate II.

Consulted on 2026-10-15:
[service-discoverability](https://skills.internetcomputer.org/.well-known/skills/service-discoverability/SKILL.md),
[agent-web-identity](https://skills.internetcomputer.org/.well-known/skills/agent-web-identity/SKILL.md),
[internet-identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md),
and [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md).
The supported approach is ICP MCP's app-specific II delegation and ordinary signed
canister methods with backend caller checks. Subnet already implements recipient
registration, principal-based grants, inherited rights and revocation in
`store/collaboration.rs` and `permissions.rs`.

Observed gap: the UI appeared under every drive listing and described only using
one's own identity, obscuring the existing separate-account sharing path. The
smallest extension is a dedicated frontend view, two setup explanations and access
to the existing principal-copy action. No new server, dependency, transport,
credential, schema or permission evaluator is necessary. A custom account-based
MCP authorization system remains deferred; it could avoid switching identities but
would introduce a separate authority lifecycle. This is a scope decision, not a
claim that ICP cannot support it.

Proof checks: navigation/deep links without listing requests, late listing response
isolation, sign-in/account changes, clipboard behavior, picker exclusion, both
languages, and responsive browser layout. Existing backend grant enforcement is
unchanged. Live acceptance still needs two real II sign-ins and connector principal
comparison, followed by shared read/create, denial outside the grant, and revocation.

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
