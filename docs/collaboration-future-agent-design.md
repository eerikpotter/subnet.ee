# Future scoped agent collaboration design

Deferred design, preserved on 14 October 2026. This is not the active alpha release contract.
The user selected ICP MCP with the same authority as the signed-in account for the alpha.
See [the active specification](collaboration-spec.md). The detailed design below remains
reference material; no transport or custom authentication implementation is selected.

## 0. Required architecture evidence

Apply the repository's AGENTS.md before architecture decisions. The requirements
below do not preselect an MCP host or authorization implementation. Begin with the
current ICP skills, evaluate their supported integration, and identify the precise
remaining capability gap before designing an extension. Record source links,
retrieval dates, component revisions, alternatives, proof tests and uncertainties
in docs/collaboration-progress.md. Recheck this evidence in a separate review pass
before freezing the architecture. A library's dependency conflict is not evidence
that the standard ICP integration cannot satisfy a requirement.

For this release, the initial skills are service-discoverability,
agent-web-identity, internet-identity, canister-security and stable-memory; consult
the live skill index for their current names and additional relevant guidance.
Do not weaken selective agent grants or authenticated attribution to make an
integration appear complete. Conversely, do not build another MCP/OAuth service
merely because it was the first proposed implementation.

## 1. Outcome and scope

**Work on your files, with other people, and with your AI.** People can use the
included editors, save to personal or shared folders, give an AI selective access,
review proposed changes, and recover earlier versions. Internet Computer storage
and Internet Identity underpin one understandable experience.

The visible objects are files and folders. Sharing an ordinary folder gives access
to the same objects; it does not copy them or create another storage location.
The browser offers **My files** and **Shared with me**. The latter shows directly
shared roots, with inherited descendants inside, rather than duplicating every file.

Keep Space internally for ownership, quota and storage placement. Each space has
one human owner in v1. A collaborator's new file belongs to the destination's space;
record its creator separately. Show the destination owner before publishing.
No Team, Organization or Shared Drive setup is needed.

Release scope:

- A complete basic file browser: navigate folders, create folders, find items by
  name, upload/open/download, rename files and folders, move files and whole folders,
  and move to Trash/restore. These are core cloud features independent of MCP.
- Human–human file/folder sharing and human–agent sharing use the same capabilities.
- External MCP agents connect to subnet.ee; review and history are built in.
  No hosted chat or model service.
- The drive stores ordinary files as well as Craft projects: images, PDFs, text,
  Markdown and other binary formats, within the same size/quota limits. Humans and
  MCP agents can upload/download them through the same permissions and history.
  Storage support does not promise an editor or inline preview for every format.
- All seven registered Craft **web apps** support the shared cloud journeys.
  Standalone desktop/cloud integration is not implied by the existing web adapters.
- Native agent editing is a separate cloud integration and verification milestone.
  Start with one WordCraft proof using an isolated headless Craft engine. It must
  not block the independently useful cloud-drive MCP release. Advertise each native
  integration only after its exact format passes verification. All seven still
  require human proposals and shared Open/Save/history in the cloud release.
- One AI connection can have many file/folder grants. Multiple connections use the
  same records, without making multi-agent management part of ordinary setup.

Completion means a usable journey across backend, drive MCP, portal and the shared
Craft cloud UI, with the evidence in section 8. Native agent editing has an additional
feature-specific gate. Compilation or a settings mockup is insufficient.

## 2. Permissions and ownership

### Five capabilities, one meaning everywhere

Persist explicit capabilities, not a numeric role. Humans and agents use the same
checks in the canister, SDK, portal, Craft popup and MCP.

- **Read:** current content, metadata and authorized folder listings; download and
  copy readable bytes. No arbitrary history or other people's pending proposals.
- **Propose:** submit an immutable candidate for an existing file or a new file in
  a folder. Live content stays unchanged. The originator can view and withdraw their
  pending submission while currently authorized for that target.
- **Create:** publish new files and create folders inside the granted folder.
  It never permits overwriting, renaming, editing or deleting an existing file,
  including one's own earlier creations. A new file receives a new stable ID;
  a name collision is never an instruction to overwrite.
- **Edit:** revise and rename existing files, rename a granted folder, read retained
  history, and restore earlier content as a new revision. It does not itself grant
  Create, Delete or Propose.
- **Delete:** move files or folders to Trash and restore them to their retained
  location under current authorization. Folder actions cover the entire subtree;
  authority on a child never permits trashing its parent. No permanent deletion.

Read is required for every other capability. Create applies only to folder scope.
No blind uploads/deletes in v1. A permission on a folder covers it and existing and
future descendants, subject to the operation's applicable resource type.

Simple sharing presets:

- **Read:** Read.
- **Propose:** Read + Propose.
- **Edit:** Read + Propose + Create + Edit; omit Create for a file grant.

Delete is an explicit extra choice, labelled **Delete files and folders (move to Trash)**.
Expand Details for individual capabilities; label the narrow checkbox **Edit existing
files** to distinguish it from the preset. Display custom selections accurately,
such as Read + Create. Persist no custom named-role system.

Ownership is separate. The space owner manages human sharing, quota,
permanent deletion and revision pruning. Neither Edit nor
any combination of the five capabilities grants these powers. There is no ownership
transfer, collaborator re-sharing to other humans, or agent onward delegation in v1.

### Inheritance and the authorization rule

For each operation, resolve the authenticated actor, resource and current ancestry.
Effective capabilities are the union of valid applicable grants. Each grant has a
stable ID, subject, root object, capabilities, status, optional expiry and version.
Agents additionally require a valid connection and the supporting authority below.
Check Read and the operation's required capability. The human owner has ownership
authority; an agent sponsored by that owner does not inherit it.

Direct grants add access. No deny rules or restricted children inside a shared
folder. To make a child private, move it outside the shared scope with the required
rights or have the owner change the parent grant. The UI shows effective access and
its sources. Removing one grant must not falsely claim all access is gone when
another still applies.

Single-file sharing reveals neither private parent names nor siblings, unrelated
counts, access rosters or history. A deep link is a reference, never a bearer grant.
Unknown and inaccessible references receive the same external response.

The service returns effective capabilities and allowed intents for presentation.
Client buttons are conveniences, never the enforcement boundary. Every endpoint,
including legacy/raw calls, checks its own authorization.

### Agents: separate credentials, selectively delegated authority

Connecting creates a stable agent participant, its sponsoring human and a distinct
credential binding. It grants **zero files**. Share adds independent file/folder
grants to that participant. The usual experience is one connected AI with, for
example, Edit on Launch, Read + Create on Inbox, Read on Reference and Propose on
Contracts, with no grant on Private.

Start agent setup with no scope or action selected. The human explicitly chooses
locations and capabilities, directly or through a preset, and confirms the visible
summary. Their current rights are the maximum available choices, not an automatic
grant. A selected broad folder can be granted deliberately; do not impose an
agent-specific ban on otherwise authorized file organization. Backend default-deny
enforcement applies even if a client bypasses the setup UI.

The sponsor chooses each scope and capability set. Owning the folder or receiving
it through a human share uses the same rule. There is no owner-controlled permission
to bring an AI, and no additional approval from the person who shared the folder.

**Effective agent rights on an object = the sponsor's current effective file
capabilities on that object intersected with the union of the agent's active,
applicable grants**, provided the connection and credential are active.

- Grant creation/expansion requires the authenticated sponsor and cannot exceed
  their current file capabilities at the selected root. Re-evaluate the intersection
  on the actual target for every operation, including descendants and publication.
- The sponsor's authority may come from ownership or any combination of valid
  human grants. Do not bind an agent grant permanently to one supporting share.
  Losing one human grant removes only rights no longer provided by another.
- Ownership supplies the five file capabilities, not delegatable ownership powers.
  Managing one's own agent grants requires no human-sharing administration right.
  Agents cannot manage grants, connect other agents or acquire controller powers.
- Granting access to one folder includes its future descendants. It does not include
  other folders the sponsor can access now or receives later. New locations require
  explicit agent grants. Additional human capabilities above the stored agent
  ceiling do not expand it automatically.
- Human downgrade, expiry or revocation reduces agent access on subsequent authorized
  operations. A still-active agent grant can become usable again within its original
  ceiling if the sponsor regains access; show this in the grant's effective status.
  A revoked/expired agent grant or disconnected connection cannot reactivate through
  renewed human access, OAuth refresh or an unrelated same-name connection.

The sponsor narrows/removes their own agent grants or disconnects their whole AI.
Alice controls Bob's human share; Bob controls his AI's narrower grants. Alice does
not separately enable, disable or administer Bob's AI. History makes Bob's sponsorship
visible for its actions on her files. Disconnect disables all connection credentials
and grants; reconnection requires explicit confirmation of any restored grants.
Routine OAuth renewal preserves the same participant and grants. User-chosen grant
expiry and runtime credential expiry are separate.

Example: Alice shares Launch with Bob using the Edit preset. Bob gives his AI only
Read + Propose there. The AI submits a candidate; Bob can approve because he has
Edit. Alice need not intervene. If Alice reduces Bob to Read, the AI cannot submit
another proposal and Bob cannot approve. Any other current authorized writer may
still review the existing candidate under the proposal rules below.

A session can use the union of all its grants. This is not isolation by chat/task:
an AI with Read on Reference and Create on Launch can use that material in Launch.
Read access permits copying; revocation cannot retract previously received data.
Separate connections can provide a narrower boundary where actually needed.

### Who approves proposals

**Any one participant who can perform the change directly can approve it.** Edit
permits deciding an existing-file proposal; Create on the destination permits
deciding a new-file proposal. These capabilities also allow reading the corresponding
submitted candidate and its retained base for review. Propose alone cannot approve.

This applies to humans and agents, including writers accepting their own proposals.
No owner-only approval, vote, assigned reviewer or separate Approver role. To reserve
publication for humans, grant agents Read + Propose and make sure no other effective
grant gives them Create/Edit. Never label general proposal review as guaranteed
human approval.

Writers may save directly. Proposals are an optional review path, not an approval
queue for every save. Proposed deletion, rename and move operations are outside v1.

## 3. Identity, MCP and trust boundaries

### Identity contract

The IC authenticates a caller principal; it does not certify human versus AI.
subnet.ee recognizes a separately registered automation connection. A separate
verified credential is required; a separate IC signing key is not automatically
required for a canister-hosted HTTP MCP endpoint.

1. The sponsor signs in through the existing subnet.ee Internet Identity origin.
   Connection consent is short-lived, single-use and bound to the intended client
   and authorization transaction. Verify the human through signed IC calls, not a
   supplied sponsor ID or the HTTP gateway's caller identity.
2. The canister records authenticated credential binding → agent participant →
   sponsor. Registration adds no file grants. Initial grants, if offered, are
   explicit and included in the confirmation.
3. The selected authentication path resolves an internal actor context from a
   verified binding. The same canister capability evaluator handles this context
   and signed human calls. No public method accepts a trusted
   actor context, agent ID or sponsor ID merely because the client supplies it.
4. If a signed Candid agent path is needed, bind a distinct agent principal to the
   same participant. A delegation rooted in the unrestricted human principal alone
   would still appear as that human and does not provide the required distinction.
5. Every protected operation checks the binding, live connection status and current
   effective capabilities. Author, app ID and is_agent fields confer no authority.
   A revoked agent credential cannot fall through to an ordinary-human code path.

History establishes which authorized connection acted, not whether an LLM generated
its instructions. Someone using that credential appears as that connection.
Automation using an unrestricted human credential cannot be detected reliably;
do not claim otherwise. Provider/model names are descriptive unless independently
verified, and a matching display name never establishes identity continuity.

Owning files is not controlling a canister. MCP credentials must never be canister
controllers or contain a controller's delegation. Preserve the configured II
derivation origin; an NNS principal is not interchangeable with the subnet.ee one.

### ICP-first transport and first proof

Start with the [service-discoverability skill](https://skills.internetcomputer.org/skills/service-discoverability/):
ordinary canister methods, public Candid metadata, get_api_doc, the declared II
origin where necessary, and an operator-approved discovery manifest allow the
existing ICP MCP connector to reach the application. This does not require
embedding another MCP server in our canister. Compare against the current
[agent identity guidance](https://skills.internetcomputer.org/skills/agent-web-identity/)
and [Internet Identity skill](https://skills.internetcomputer.org/skills/internet-identity/)
before selecting an authentication design.

The gap to prove is specific: can the standard path provide a trustworthy agent
binding, narrower file/folder authority, immediate application revocation and
separate provenance without a fallback to the sponsor's unrestricted identity?
Inspect current delegation and caller-information support and test an attempted
human-authority bypass. A supplied agent label or voluntary choice of a limited
account is insufficient if the same connection can still use the human account.

Only after establishing that gap, compare the smallest supported extension with a
dedicated endpoint. If a dedicated endpoint is necessary, prefer ICP hosting and
the existing canister authority; reuse maintained protocol/auth components and
avoid unnecessary per-agent signing keys. [ic-rmcp](https://github.com/ByteSmithLabs/ic-rmcp)
and official rmcp-based adapters are candidates, not requirements. Their failures
do not invalidate the existing ICP MCP route. OAuth, if used, must not become a
second ACL database or embed a lasting permission snapshot. G1 must demonstrate
the chosen consent, credential verification and revocation flow end to end.

**Secret protection is a deployment requirement, not a property implied by ICP.**
[ICP's security documentation](https://docs.internetcomputer.org/concepts/security/)
states that standard application subnets do not keep canister memory secret from
node operators. The [public Confidential subnet re2t4](https://forum.dfinity.org/t/question-about-sev-enabled-production-subnets/74683)
is a candidate to evaluate for credential handling; verify current availability,
SEV-SNP guarantees and deployment placement.
Record which parties can see bearer credentials along the entire HTTP gateway,
ingress, execution and recovery path. TEE protection of memory alone is insufficient
evidence about that path. Do not assume a key held on a confidential subnet makes a
token or decrypted secret sent to an ordinary canister confidential.

Avoid unnecessary signing-key custody. If an issuer or other component needs secret
material, G1 must choose and verify its protection, rotation and recovery, considering
ICP threshold cryptography where appropriate. Do not invent custom cryptography or
store recoverable secrets in ordinary canister state. A separate auth canister is
justified only if the deployment requires it; then authenticate inter-canister
assertions and recheck current grants after awaits. Prefer one authority while it
satisfies the proven requirements.

G1 records exact components/versions, subnet and gateway trust, callback flow,
credential lifecycle and supported clients. Prove the flow locally and verify
deployment-specific guarantees separately. Do not silently substitute an external
MCP/key server if the ICP path fails; document the concrete blocker and revisit that
tradeoff before freezing the architecture. No deployment follows from this draft.

Security requirements for the selected transport:

- TLS; resource/issuer/audience/expiry validation; PKCE; exact registered redirect
  matching; single-use authorization codes; CSRF-protected, client-specific consent.
  Bind the OAuth transaction and canister-approved credential to the same connection.
- Validate client metadata/discovery URLs against SSRF, redirect and private-network
  access. Check HTTP origins where applicable. MCP session IDs are not credentials;
  authorize each HTTP request, including resumed streams.
- Keep credentials out of URLs, model context, logs and shared caches. Store token
  verifiers rather than recoverable bearer values where the chosen flow permits;
  hashing at rest does not protect credentials in transit or execution. Use rotating
  refresh credentials with reuse detection. Disconnect blocks the canister binding
  even if a transport token remains unexpired. Bind responses to the authenticated
  request and prevent cross-user caching or certified-data disclosure.
- Routine OAuth refresh retains the participant and live grants. Replacing a
  long-lived credential binding, including an optional IC signing key, requires
  authenticated sponsor confirmation and atomic revocation of the old binding.
  Ordinary refresh rotation does not require another consent screen.
- Fail closed if token verification or canister authorization is unavailable.
  No unrestricted human tokens, raw-call escape tools, arbitrary URL import,
  filesystem access, shell execution, or grant-management tools.

Follow the official ICP security and service-discoverability skills during
implementation. Publishing the optional ic-architecture discovery manifest has a
separate operator-consent requirement under that skill; do not create/publish it
as a side effect of this draft. A custom test endpoint does not require it.

### Native Craft files: reuse the editing engines

Cloud MCP and Craft editing tools solve different problems. Cloud operations
authorize and version bytes. Craft engines understand paragraphs, cells, layers,
slides, PDF pages, tracks and drawing entities. Agents call structured commands;
the engine parses and serializes the actual project. Do not make the agent rewrite
large JSON/container files or convert projects through Markdown/PNG to edit them.
An exported image or flattened document is not an equivalent editable project.

There is already relevant code, not just an upstream intention:

PhotoCraft's upstream documentation describes a shared registry of 500+ commands
used by its UI, CLI, control channel and MCP, with the document engine below a thin
UI. The local WordCraft, GridCraft, DeckCraft, SoundCraft and CADCraft architecture
rules similarly put commands beneath the UI; PdfCraft requires headless engine
APIs and automation tools for user-facing features. This is an existing design
foundation for agent work. Feature completeness still needs verification per app.
The unproven part of this plan is their end-to-end subnet.ee integration, including
cloud permissions, native persistence and attribution. Do not describe all existing
Craft agent support as merely experimental.

- WordCraft: headless MCP with text/selection/formatting, structure inspection and
  page rendering; optional bridge to a running desktop app.
- GridCraft: headless/desktop MCP with cell/range/formula/chart operations.
- DeckCraft: headless/desktop MCP with slide/shape/text operations and rendering.
- PdfCraft: headless automation/MCP for PDF operations, inspection and previews;
  a separate opt-in desktop control interface.
- PhotoCraft: headless automation/MCP for documents/layers and previews, plus an
  optional desktop bridge.
- SoundCraft: headless/desktop MCP for sessions, tracks, clips and rendering audio.
- CADCraft: headless/desktop MCP for drawing commands, entity inspection and rendering.

These are source/documentation findings, not new executions of their test suites.
They are generally local stdio/loopback integrations, not a deployed remote subnet.ee
connection. Desktop control keys and local filesystem-root restrictions do not
automatically enforce cloud grants. Document author labels are not verified cloud
actor identities. WordCraft's tracked changes are also distinct from a cloud
proposal containing a complete candidate revision.

For the first native cloud integration:

1. Open a permitted file/revision by cloud ID into an isolated WordCraft engine
   session. Use the exact cloud codec, not a generic extension-based importer.
2. Expose a bounded set of existing inspection, text/format and preview operations
   for that session. The model receives useful document structure and previews;
   embedded images/whole binaries need not enter its context.
3. Encode the candidate with the same native codec as the web editor. Reopen and
   validate it, preserving content not affected by the requested operations.
4. Submit through the normal Propose or Edit path under the agent credential.
   Publication checks current authority, base revision and quotas as usual.

For the separate native milestone, prove one bounded headless WordCraft execution
path. Evaluate ICP-compatible engine execution first; if it cannot meet runtime
limits, make the local-client or external-worker tradeoff explicit before choosing
hosting. Drive MCP does not require this worker. The worker receives
only authorized inputs, has no signing/controller keys, ambient user files or
unrestricted network access, and is isolated by connection/job with resource limits
and cleanup. File access/publication go through the authorized cloud APIs. Bind every
session lookup and command to the connection; stop managed operations on lost
authority. Do not expose arbitrary execute/call_app commands without constraining
filesystem, network and other out-of-document effects. A local agent may use local
Craft tools too, but that alone is not proof of a working consumer web connection.

Reuse engine APIs and their command registry; do not invent seven document models.
Existing desktop live-control can be integrated later with explicit buffer ownership,
permission checks and attribution. Never attach a cloud agent to an unrestricted
human editor session or silently mix its changes into a human-attributed save.

**Observed format hazards to test:** WordCraft's cloud native codec stores embedded
media and passthrough parts; generic engine JSON I/O currently uses plain document
serde, whose media/passthrough fields are skipped. Routing a cloud project through
that generic path can lose content. SoundCraft cloud uses a portable session bundle
with audio, while documented local MCP sessions use a project plus external audio.
Use the portable codec for cloud round trips. GridCraft cloud uses scjson; DeckCraft
uses deckcraft; PhotoCraft uses pcraft; PdfCraft uses PDF; CADCraft uses DXF.
Validate each exact registered format before advertising its native agent adapter.

For WordCraft acceptance, start with text, formatting, a table, an embedded image
and a preserved source part. Edit one paragraph through MCP, then reopen in the
web app and assert the intended text change plus preservation of untouched content.
This is separate from the root MCP test, which only needs a small valid file fixture.

### What sovereignty and the audit trail do not guarantee

State and authorization live on ICP, but v1 is not end-to-end encrypted. Canister
controllers can change code; standard subnet nodes can inspect state. Document the
selected confidential-subnet guarantees and gateway/credential visibility from G1,
without claiming that a TEE removes controller or client trust. Authorized AI
providers receive requested content. All Craft web apps currently share a trusted
origin; a compromised app can threaten a browser session. Do not promise protection
against a malicious controller, provider retention, a compromised authorized device,
or universal proof of authorship.

The revision log is append-only under the application contract, not an independently
tamper-proof record against controller upgrades. Exports let users retain their
files; private operator recovery copies are not a substitute for usable downloads.

## 4. Data lifecycle and operation contract

### General files and Craft projects

The drive is format-agnostic storage. Preserve exact bytes, filename, size and
content hash; permit valid empty files. Add a versioned catalog descriptor with
media-type hints and an optional recognized Craft format/version. Keep the existing
FileKind contract for old clients, but do not require a Craft app ID to store a PNG,
Markdown note or otherwise unknown binary file. Existing Craft files retain their
format and opening behavior after migration.

The app registry determines available Open/Edit/Import actions, not permission to
store a file. Uploading an image is immediately useful even without a native
PhotoCraft agent integration. Importing that image into a layered project is a
separate app operation; exporting PNG does not replace the editable project.

Provide portal upload/download plus the same byte-transfer operations through MCP.
Listing, sharing, copying, permitted rename/trash, revisions, proposals and restore
apply to generic files as well as native projects. An agent can create a note or
upload an image produced by another tool; the cloud MCP service does not itself
generate images or require a model service.

Offer bounded image and escaped plain-text previews where supported. A generic
file without a safe preview remains downloadable, including exact candidate
downloads for proposal review. Do not invent a semantic diff or editable view for
an unsupported format. Treat extension and MIME type as untrusted hints: uploaded
HTML/SVG/scripts must never execute on the trusted app origin. Binary transfer
responses preserve bytes and use safe download handling.

### Minimal records

Extend the existing single authority with: folder parent links; grants; agent
participants and credential bindings; upload sessions; immutable revision manifests;
proposals; bounded read handles; and audit events. Reuse account/space/file IDs.
Use typed actor references and separate stable participant IDs from rotating
credential IDs. Do not introduce a policy service, workflow engine or event bus.

Files/folders have stable logical IDs, owning space, parent, name, metadata version
and live/trash state. Prevent cycles, cross-space parents and unbounded ancestry.
Treat names as labels, never paths or object identity. Define and validate name,
page, tree-depth and request-size limits in one policy module.

Rename changes the object's label and metadata version, never its ID, content,
registered format or revision head. Moving changes the parent and metadata version,
retaining the object ID, owner, revisions and direct shares. Renaming/moving a folder
does not rewrite its descendants' IDs or content. Record folder creation, rename,
move, trash and restore as activity events with actor/sponsor, affected object and
before/after metadata. Content revisions and organization activity are distinct;
restoring old file content does not undo later moves, names or permissions.

For new create/rename/move operations, reject a live sibling name collision across files
and folders; never overwrite, merge folders or silently add a numeric suffix. Offer
an explicit different name and retry. Compare names after Unicode NFC normalization,
outer-whitespace trimming and case folding; preserve the normalized display case.
Reject empty names, '.', '..', path separators and control characters. Freeze length
limits and Unicode implementation in G1. Existing legacy duplicates retain their
names and IDs and remain individually selectable; a no-op rename of the same object
does not become an error. Never guess an object from a name when IDs are available.

A revision records content hash/length/manifest, parent revision, submitting actor,
sponsor when applicable, committing actor, server time, credential reference and
mutation ID. Capture the actual supporting grant references for provenance. App/model
labels supplied by clients are labelled as claims. Legacy unknown provenance stays
unknown; do not relabel legacy revisions as known human work.

### Reads and revocation: a concrete baseline

Use authoritative update calls for the new shared/agent catalog and read APIs.
Opening current content returns its immutable manifest and a short-lived read
handle bound to actor, credential where relevant, object and revision. A handle is
not a bearer capability. It pins that revision for a maximum 30-minute transfer;
every chunk update rechecks current Read, active ancestry and connection.

This allows a reader to finish a fixed revision when the head advances, without
opening arbitrary older versions. New read handles for retained history require
Edit, except the specific base/candidate allowed for proposal review.

Revocation and each chunk/commit are ordered at the canister. Once effective access
is removed, later chunk requests and commits fail. Bytes from an already completed
authorized call may still arrive or remain on the client's device. Do not claim to
cancel those bytes. Chunk length/hash is checked against the authorized manifest.

The legacy chunk query may remain available only to the human space owner under
the old contract. It must not become a shared/agent read bypass. Human logout
clears local sessions; it does not revoke already issued II delegations.
Any future query optimization must preserve or explicitly revise the revocation
contract. Measure the update-based transfer path at G1 before freezing it; do not
add a bespoke signed-ticket scheme without evidence that it is necessary.

### Staging and publishing

Create and revision uploads have an immutable target, destination/owning space,
mode, format, expected head, byte length/hash, initiating actor/credential and
idempotency key. A transfer cannot switch targets, actors or proposal/live mode.
No file becomes visible before its first content commit.

Chunks have bounded indices and size; identical retries are safe, contradictory
retries fail. Other editors cannot resume, cancel or write another actor's upload
by guessing its ID. Credential rotation invalidates incomplete agent transfers;
preserve the caller's local draft for a new upload.

At publication, recheck actor, current capabilities, parent/liveness, expected head,
size/hash, quotas and target consistency. Prepare all fallible validation/encoding
before writing state. Then publish revision/head, reservations and audit event in
one canister message with no external await. An error return must not leave partial
state; do not assume returning Err rolls back prior stable-memory writes.

Scope idempotency to actor and operation, record a canonical request digest and
result. Same key + same input returns the original result after authorization;
same key + different input fails. Permission checks precede retry shortcuts.
Document a bounded retry lifetime and return an expired-request error after it;
never silently replay an expired mutation as a new one. Retained revision/proposal
mutation IDs remain usable for reconciliation.

### Proposals and conflicts

A submitted proposal binds one target, candidate hash/manifest, originator,
original base revision (or new-file destination), creation/expiry and state:
Pending → Accepted, Discarded or Expired. Content/base cannot be edited after
submission. Withdrawal is Discarded by the originator.

Accept requires the relevant current Create/Edit capability, the submitted hash
and expected base/head. First valid terminal decision wins atomically. A retry or
concurrent reviewer sees the same result, never a second revision or decision.
Accepted revisions preserve both originator and accepting actor.

If the live head changed, acceptance fails. V1 offers Open latest and prepare a
replacement proposal with a new base and lineage to the old proposal. It does not
silently rebase or overwrite a newer revision. Discard never changes live content.
The same expected-head rule applies to direct saves and history restoration.

A revoked submitter cannot fetch or change their old proposal, but a currently
authorized reviewer may accept an already submitted candidate. A permission change
does not secretly delete useful pending work. Human modifications create a new
human-submitted candidate retaining the original proposal lineage.

### Moves, trash, history and quota

- Moving an object requires Read + Edit + Delete on the source and Read + Create
  on the destination folder, for humans and agents alike. A folder move requires
  those source rights throughout its subtree. Restrict moves to one owning space;
  reject cycles, movement of the space root and replacement of another object.
  Preview changed audiences, including inherited history access for editors.
  Direct grants remain on stable IDs; inherited rights and agent access are
  recomputed from the new ancestry. Moving a child out of an agent's granted folder
  removes that inherited access unless another grant applies.
- Save a copy requires source Read and destination Create. It has a new ID and
  destination ownership/inheritance, without copying source grants or history.
- Trashed objects and descendants of a trashed folder reject normal read/write/
  proposal publication. Authorized Trash views expose only recoverable items,
  not an alternate route to their bytes. Folder trash/restore requires Read + Delete
  throughout the subtree under the current retained permission tree. Restore checks
  current authority through its dedicated endpoint despite the trashed state; it
  never restores historical grants or revives descendants trashed separately before
  their parent. The retained parent must be live; otherwise
  restore authorized ancestors first or ask the owner to recover the location.
  The space root cannot be trashed. Show the scope and affected count before a human
  confirms a folder operation; MCP descriptions make the same subtree effect clear.
- Organization operations validate current rights, expected parent/tree versions
  and affected subtree bounds before one atomic mutation. Never partially move,
  trash or restore a folder when a descendant check fails. Bound subtree evaluation
  in G1. No separate Move role or automatic approval queue is required.
- A stale or newly occupied destination requires refreshing and a new explicit
  selection/name; never fall back to another folder. Restore also checks name
  collisions and offers an explicit restore name without replacing a live item.
  A different restore name additionally requires Edit; Delete alone must not become
  a rename bypass. Without Edit, an authorized editor/owner must resolve the conflict.
  Moving an object to its current parent is a no-op, not a content revision.
- Restoring old content creates a new revision with current permissions and links
  to the restored revision. It does not restore old grants or erase later history.
- Staging, proposals, trash and retained revisions consume the destination space's
  quota. Reserve capacity before accepting chunks; rejection releases reservations.
  Bound concurrent uploads/proposals/read handles and small metadata records per
  actor and space, so tiny or abandoned objects cannot exhaust service memory.
- Retention defaults: incomplete uploads 24 hours; pending proposals 7 days; trash
  30 days. Show deadlines. Expire and reclaim incrementally with bounded work.
  Keep compact decision/tombstone records with a documented bounded retention.
- Do not silently prune published history. At the revision/storage limit, explain
  the remedy. Owner pruning cannot remove the head or a referenced proposal/read
  base. Garbage collection respects all retained references. File purge explicitly
  terminates dependent proposals/handles and removes its revisions as one owner
  operation. Neither Delete nor an agent can purge.

Reuse existing ceilings initially: 64 MiB/file, 256 MiB/owning space, 2 GiB/service,
20 files/space and 20 revisions/file. These are real product limits, especially for
audio/images; expose them before saving. Define additional bounded metadata,
rate and retry limits in G1 from worst-case message/instruction measurements.
No larger capacity or unlimited history is promised.

### One cloud service contract, used by every client

The cloud service owns the object tree, authorization, organization commands,
transfers, revisions and activity. Its domain handlers take a server-verified actor
context and use one capability evaluator. The shared SDK exposes typed requests,
results and error codes; the UI controller supplies presentation state. Portal and
Craft renderers invoke that controller. MCP translates protocol requests to the same
domain handlers, including folder operations. It does not implement another drive,
permission store, tree or mutation path. No app-specific folder-management backend.

Freeze typed Candid DTOs and MCP JSON schemas at G1, with one operation-to-capability
mapping used by contract tests. Backend enforcement remains authoritative even if
UI state is stale. Semantic coverage is required; names may follow conventions:

- Browse personal/shared roots and paginated folder children; inspect an object,
  authorized breadcrumb context, effective capabilities and allowed actions.
- Sort by name or last updated time and filter names within the current location.
  Apply this across the authorized listing, not just the currently loaded page.
- Open current content; read a pinned chunk; list/open history with Edit.
- Create a folder; begin/upload/commit/cancel a new file, revision or proposal.
- List/read eligible proposals; accept/discard/withdraw with the rules above.
- Rename with Edit; file/folder trash and restore with Delete; move with source
  Edit/Delete and destination Create, always with Read and subtree checks as above.
- Inspect a move destination and its access consequences using the same rules as
  commit. Return only authorized labels and impact summaries, never a private
  roster or parent chain. This read is no reservation or lasting authorization.
- List authorized Trash entries and restore them; list visible organization activity.
- Restore earlier content with Edit.
- Resolve mutation status for authorized retry recovery.

Mutations carry stable object/destination IDs, expected versions and an idempotency
key. Return canonical changed metadata, new versions and a mutation reference.
Refresh affected listings and allowed actions from the result/current state. Error
codes include name collision, stale metadata, invalid destination, quota and changed
access where the caller may see those details. A lost reply is reconciled through
mutation status, never guessed from a duplicate filename. Pagination binds sort,
filter and listing version; invalidate a stale cursor and refresh predictably when
the tree changes. Transport/app display strings do not define these semantics.

Owner/share/connection administration stays on authenticated human APIs and UI,
not MCP tools. File scope never authorizes listing its parent. Reviewable proposals
and history use separate response shapes so current-file listings cannot leak them.
Pagination tokens bind scope and actor; recheck access on every page.

Expose small structured results with resource/revision IDs and hashes. Do not place
64 MiB blobs in one tool result: bound text/base64 chunks and reuse the transfer
protocol. Format IDs/schemas and useful operation descriptions are public; user
data is not. Unknown native operations fail clearly. Tool annotations are hints,
not security. Errors distinguish conflict, limit, expiry, unsupported format and
retryable transport failure only after establishing access; outsiders get a
non-revealing inaccessible/not-found response.

## 5. One elegant cloud experience

### Shared implementation

Extend the existing Rust SDK, cloud controller/model, shared egui popup and portal
facade. Extract only the view-independent logic needed for both renderers. One
literal renderer is optional; separate folder/permission implementations are not.
All seven apps consume the canonical shared contract through their existing
Editor adapters and deliberately updated vendored SDK snapshots.

The controller provides location, items, selection, capabilities, permitted intents,
operation/error state and account/document/request/dialog stamps. Each app supplies
native format, immutable snapshots, bounded parse/open, working-document mode,
dirty tracking and theme tokens. No app computes its own ACL.

Maintain navigation history, breadcrumb context, name filter, sort and destination
selection in this same controller. File management requires no document engine;
renaming or moving a folder must work with every Craft editor closed. An editor's
cloud binding uses a stable file ID, not a path. An external rename/move refreshes
its displayed location/name without losing unsaved work or binding to another file.
A normal content Save does not silently rename or move the file back. If the object
is trashed or access is lost, preserve the local draft and block publication; offer
an authorized Save a copy or local download, never silently resurrect the object.

Required document modes: Read-only, Proposed changes and Editing. Enforce mode at
editor command dispatch as well as buttons/shortcuts; preserve selection, scrolling
and nonmutating actions. A local working copy is not a permission to publish.
Save a copy detaches from the source and requires a chosen Create destination.

Read + Create opens existing files read-only but can save new work. After its first
publication, another save needs Edit; explain this immediately and offer Save a
copy. Propose provides a separate editable draft and Submit changes. When Edit is
also present, Save remains primary and Submit changes is an explicit alternative.
Submission marks only that captured draft submitted, never the live file saved.

Save completion cleans only the captured document revision; later edits remain
dirty. New/Open changes document identity. Account switching clears navigation,
cloud bindings and private caches while retaining the unsaved local buffer with an
explicit detached state. It never silently saves that buffer under another account.
Delayed callbacks cannot replace a different document or bind it to another user.

### File browser and organization journeys

The portal and every Craft cloud window use the same following interaction rules.
The popup may be smaller and emphasize Open/Save, but retains folder organization
through the same item menu and destination picker.

- **Navigate:** My files, Shared with me and Trash are views, not movable folders.
  Open folders, go Up, use breadcrumbs and Back/Forward, and return with selection
  and scroll position preserved. The portal's URL/back navigation and the popup's
  local navigation stack obey the same location model without closing the editor.
  A direct shared file exposes no private ancestor path. A directly shared folder's
  breadcrumb starts at its authorized shared root, not the owner's private root.
- **Find and select:** one clear list with folder/file distinction, name, type and
  last updated time. Sort by name/time; filter names within the current location.
  Keep all authorized formats visible and label which the current app can open.
  Support keyboard selection/open, accessible item menus, and useful empty/loading/
  error states. Empty folder and no filter matches have different messages.
- **Create:** New folder, Upload and New document are available where permitted.
  New document launches the selected Craft app with destination intent. The Save
  and Move pickers also allow creating a destination folder when authorized.
  Preserve the working document and chosen destination on cancel or failure.
- **Rename:** the item menu offers one focused name editor with Save/Cancel. Show
  validation/collision errors beside the name and retain the typed value. Preserve
  recognized Craft format extensions in the normal rename UI; changing a label
  never converts content. Refresh an open editor's title through its cloud binding.
- **Move:** Move to… opens the shared folder picker for files and whole folders.
  Show the selected item, current location and intended destination. Permit browsing
  readable locations, but enable the final action only for an eligible destination.
  Explain an unavailable destination without exposing private information. Reject
  the folder itself, its descendants and another owning space. Show that inherited
  access follows the destination and existing direct shares remain; do not imply
  moving out of a shared folder necessarily makes an item private. Confirm the
  chosen destination and any changed sharing consequence before committing.
- **Trash and restore:** say Move to Trash, not Delete forever. For a folder,
  explain that its contents are included and show the authorized affected count.
  Trash lists eligible entries, deletion time and retention deadline. Restore and
  immediate Undo use the same current-permission checks and collision handling.
  Do not promise undo after expiry, access loss or an incompatible parent change.
  Permanent deletion remains a separate explicit owner action.

After each successful action, update the list, focus and any bound editor from the
canonical result. If a viewed folder moves, preserve its ID and refresh the permitted
breadcrumb. If it is trashed or becomes inaccessible, return to the nearest still
authorized location and explain the change. Clear inaccessible cached labels/rows;
retain unsaved document work as described above. Recheck when a cloud window opens,
returns to focus or performs an operation; live push subscriptions are not required.
Stale async responses cannot reinstate a previous account, location or selection.

Start with single-item actions; a folder is one selected item with a subtree effect.
Move to… is the complete required path. Drag-and-drop, bulk selection/actions,
recursive folder copying and global/full-text search are later enhancements. Avoid
requiring a desktop file-manager feature set to deliver a complete basic drive.

### Sharing, editing and review journeys

The same cloud window supports Browse, Open, Save, Save a copy, Submit changes,
Share, Review and History. It remembers location within the current account.
Portal New launches the right Craft app carrying destination intent, not authority.
Recheck destination rights on save. The portal also has Upload for ordinary files.
The picker shows all authorized files and indicates which the current app can open;
unsupported formats remain downloadable without being converted or hidden.

Share selects a person or the current human's connected AI, scope and capabilities.
Human sharing administration is owner-only; managing access for one's own AI is
available to every authorized human within their current rights. Agent setup starts
with no selections; offer Propose as a useful explicit choice, not a preselected
grant. Human sharing may recommend Read. Advanced capabilities, expiry and inherited
access details are secondary. Explain that Delete includes folders and that moving
requires source Edit/Delete and destination Create.

Owners see human grants and relevant agent activity in their space; each sponsor
manages their own AI grants. Collaborators see their own authority without unrelated
roster disclosure. Show the granted ceiling and any effective reduction from lost
human access. A stable public account share ID/link resolves the exact account;
display names alone are never verified identity.

Restricted links let unauthorized people sign in and request access without first
revealing a file name or content. Owner approval binds the authenticated requester.
Deduplicate/rate-limit requests; use an in-product Activity view. Copying a link does
not send an email. No email infrastructure is required.

Connect your AI shows the real endpoint and current instructions for the advertised
Claude/OpenAI clients. Reuse the selected ICP connection and II consent flow, then
bind the restricted agent in subnet.ee → “No files shared yet” → Share a folder.
The exact linking steps must be proven in G1; do not assume subnet.ee must operate
an OAuth issuer. If a dedicated endpoint is justified, integrate its consent into
the same journey.
Connecting during Share may combine the last steps with an explicit grant summary.
Explain that the AI provider receives content it reads. No model API key is required
for the consumer connector flow.

Show Connected, Awaiting authorization, Expired, Revoked and Disconnected distinctly.
“Shared with this AI” lists its locations/actions. Remove access affects a grant;
Disconnect affects the entire connection. No technical key/session dashboard is
needed. Provider tool confirmations do not grant access or replace proposal approval.

Review changes shows real candidate content, source/base, submitting actor, sponsor,
and Accept/Discard when authorized. Use native read-only preview in the corresponding
Craft app when a portal preview is impractical; returning must retain review context.
Semantic diffs for every format and server-side rendering are not required.
An agent-written summary never substitutes for the content being approved.

History has one shell: time, actor, sponsor, operation, version and preview/open/
restore. Open a past version read-only and distinguish it from the current file.
Unsaved work is never replaced by opening a preview without the normal protection.
Activity includes grant changes, connections, publication and proposal decisions;
it also records file/folder organization with the same human/agent attribution.
Current access filters what can be seen. An old grant is not lasting visibility.
Before/after location details never reveal currently unreadable former ancestors
or their paths to a new collaborator; redact those details in the activity response.

### Failure and visual quality

Preserve drafts on network loss, conflict, expiry, denial and quota limits. Offer
Retry, Open latest, Save a copy or Download only where meaningful. On lost access,
retain already loaded local work but stop new protected reads/writes. Do not promise
offline sync, automatic merging or autosave.

Use plain copy: “Bob saved a newer version” and “Your access changed; local changes
are still here.” Distinguish Local changes, Uploading, Saved, Offline, Conflict and
Lost access. Recover response-loss saves through mutation status before starting
another operation.

One primary action per state; location/list/contextual action structure; focused
Share/Review/History views rather than stacked dialogs. Preserve selection and
focus on Back/Cancel/Escape. Use restrained surfaces, consistent spacing and semantic
theme tokens matching each app's light/dark appearance. Show ownership/role context
once where useful. No raw principals, canister IDs, tokens or permission JSON in
ordinary workflows.

English/Estonian, keyboard navigation, focus return, accessible labels, contrast,
long names and narrow widths are release requirements. Locale changes must not
reload editors. Inspect real screenshots and repair clipping, layout jumps and
unclear actions. A short walkthrough with an unfamiliar person must demonstrate
finding shared work, sharing, proposing, approving and restoring without technical
instructions, plus navigating, renaming and moving a folder. Record actual
observations; do not invent a usability score.

## 6. Security, migration and recovery

Protect against a malicious outsider, overreaching collaborator, stolen agent
credential, forged IDs, stale clients, hostile documents and interrupted operations.
Enforce these boundaries in implementation and tests:

- All protected metadata, bytes, previews, proposals, history, uploads and audit
  paths authorize the resolved caller and object. Never rely on inspect_message,
  hidden buttons, a cached role or a transport's claims. Recheck after any await.
- Treat file contents, names, thumbnails and agent summaries as untrusted data.
  Escape labels; no executable HTML/SVG in privileged origin. Native previews do
  not execute macros/scripts, load arbitrary external resources or gain OAuth
  credentials. Bound parser allocation/recursion/archive expansion; malformed
  documents leave the current editor buffer intact.
- File text can contain prompt injection. It cannot change grants or authorize
  tool actions. Do not claim ACLs prevent an agent misusing actions it legitimately
  has; narrow grants and optional proposals are the available controls.
- Never log tokens, signing keys or file contents by default. Audit server-derived
  decisions with actor/resource/mutation references and filter visibility.
- Bound public account resolution, access requests, tree traversal, pagination,
  metadata counts and all transfer reservations. Monitor canister cycles/storage,
  MCP/auth failures and quota exhaustion; reject work cleanly before capacity failure.

Preserve deployed V1 Candid methods, file IDs, serde encodings and stable-memory
slots. Add versioned records/indexes and compatibility adapters. Existing private
files migrate under their owner's personal root with no new grants. Legacy names
and IDs must survive, including duplicates. All paths share the same revision and
authorization state. Legacy destructive calls remain owner-only and gain the new
reference/retention safeguards; compatibility cannot create a bypass.

Use a pinned real previous Wasm in the upgrade test, not just new → new. Test
incomplete uploads, lost replies, legacy users and rollback procedure. Do not promise
that old code understands newly written state. Schema rollback requires a documented,
tested recovery path.

Keep backups private, access-controlled and integrity-checked, including their
metadata. A historical snapshot may contain grants/keys revoked since it was taken.
Restore into a stopped/isolated recovery instance; run migration into recovery mode
before enabling traffic. Suspend restored non-owner sharing and every agent binding
until reconciled with current revocations or explicitly reauthorized. Old tokens
must not regain access. Preserve grant records and attribution as recovery data,
not blindly active authority. Test this sequence before production release.

Document backup frequency, retention and measured recovery time/data-loss window
for the chosen deployment; do not invent an availability guarantee. Include the
responsible operator, cycle top-up/alert procedure and an emergency disable path for
MCP that leaves human file access working. These are short operating instructions,
not another administration product.

## 7. Existing foundation and design decisions

Inspected in this repository on 14 October 2026:

- The backend currently checks personal ownership, not shared grants. Content chunks
  use queries; metadata uses updates. Shared reads need the new contract in section 4.
- Existing protocol already has stable file IDs, manifests, expected-revision saves
  and bounded uploads. One pending upload per file must become isolated sessions.
- The shared Editor interface captures/opens/saves snapshots but has no universal
  Read-only/Proposed changes mode. All seven browser hosts use this shared package;
  interface changes need an adapter and command-path check in every app.
- PocketIC persistence and previous-Wasm SDK migration tests already exist.
  Human sharing and real MCP suites described below do not yet exist.

Keep these choices small: one canister authority, one capability evaluator, one
shared UI state contract and two thin existing renderers. Reuse the existing ICP
MCP route where the G1 proof supports it; add only the integration needed to enforce
the agent requirements. Retain
extension points through stable IDs, typed actors/owners, immutable manifests and
transport adapters; do not implement future organizational machinery now.

## 8. Acceptance tests and delivery gates

The core cloud release requires the evidence below except A09, which gates the
separate native agent-editing milestone. These are not claims of passing tests.
Map each requirement ID to a test or named manual observation, command, commit/version
and outcome. Mark missing, skipped and failed checks explicitly; identify A09 as
deferred until that milestone is implemented.

### Product acceptance

- **A01 — Human folder sharing:** Alice shares a folder with Bob; Bob sees one root
  in Shared with me and the same file IDs/content. Eve sees nothing.
- **A02 — Ownership:** Bob creates inside Alice's folder; Alice's space owns/pays,
  Bob is creator. No hidden extra rights for the creator.
- **A03 — File isolation:** a direct file grant exposes no parent/sibling/history/
  roster data; deep links, pagination, previews and raw calls obey the same boundary.
- **A04 — Effective permissions:** test inherited/direct overlap, expiry, downgrade,
  explicit regrant and moves; UI explains residual rights accurately.
- **A05 — Sharing setup:** account resolution and access requests bind exact signed-in
  recipients; no name spoofing, anonymous link access or unauthorized grant changes.
- **A06 — Identity:** distinct agent credential, verified sponsor binding, zero initial
  grants; forged caller fields, stale bindings and same-name connections cannot impersonate.
- **A07 — Selective delegation:** one AI has different rights on several folders;
  owner and collaborator sponsors use the same rule without another owner's approval.
  Bob chooses the scopes and ceilings; current human rights cap every call. Test
  overlapping human grants, downgrade/revocation/expiry, retained ceilings when
  human access returns, and explicit agent revocation that cannot reactivate.
  New unrelated human shares grant the AI nothing; disconnect blocks all credentials.
- **A08 — Capability matrix:** every operation tested allowed/denied for humans and
  agents; Read + Create cannot revise/delete even its own published files; no preset
  implies Delete/admin. Owner-only legacy calls cannot be used to bypass this.
- **A09 — Native integration milestone:** real MCP reads a native WordCraft file and uses
  its engine tools to propose a change; person previews, accepts and reopens it.
  Assert intended edits, untouched formatting/media/source parts and authenticated
  agent/sponsor/approver attribution. No open editor required for file operations.
- **A10 — Revocation ordering:** revoke during upload/download; later chunk updates
  and commits fail, earlier authorized results may arrive; other valid grants work.
- **A11 — Conflicts:** stale saves, restore and proposal acceptance cannot overwrite
  a newer head. Replacement proposals retain lineage. Local drafts survive.
- **A12 — Atomicity/retries:** lost replies, concurrent decisions, mismatched retry
  payloads and expired keys cause no duplicate files/revisions/charges or partial state.
- **A13 — Review/history:** human/agent writers and self-approval use the same rule;
  Propose alone cannot decide; discard changes no live bytes; restore creates a
  revision; revoked submitter loses access but an authorized reviewer may decide.
- **A14 — Lifecycle:** move/copy/trash/restore/prune/expiry preserve access boundaries,
  quotas, retained references and provenance; trashed parents block publication.
  Explicitly authorized humans and agents move/trash/restore folders; insufficient
  source or destination rights, cycles, cross-space moves and stale trees fail
  without partial changes. An agent with only Create cannot organize existing files.
- **A15 — All apps:** PhotoCraft, PdfCraft, WordCraft, DeckCraft, SoundCraft, CADCraft
  and GridCraft each pass Open, shared Save, Save a copy, Propose, Review and History,
  plus the common navigation and file/folder organization journeys in A22.
- **A16 — Work preservation:** offline, quota, expired access, account/document switch,
  in-progress editing/recording and delayed callbacks preserve the right draft.
- **A17 — UX:** both locales, themes, keyboard/focus, narrow layouts and long names;
  screenshots from portal/all seven apps and an observed nontechnical walkthrough.
- **A18 — Upgrade/recovery:** real old-Wasm data survives with original IDs/bytes;
  backup restore never resurrects revoked sharing, credentials or stale uploads.
- **A19 — OAuth/transport:** replay, wrong audience/issuer, substituted client/callback,
  CSRF, expired token, refresh reuse, SSRF and stream/session confusion fail safely.
- **A20 — Hostile input/resource bounds:** malicious labels/documents, huge/cyclic
  trees, decompression limits, over-quota staging and metadata floods are bounded;
  no privileged script execution or overwritten local buffer.
- **A21 — General files:** human upload and real MCP create/read/revise/propose/
  download work for a PNG, Markdown/text file, empty file, opaque binary and existing
  Craft fixture. Assert exact bytes/hash, independent Create/Edit/Delete checks,
  ownership, attribution and history/restore. Unsupported previews have a usable
  download path; claimed MIME/extension never permits script execution.
- **A22 — Core cloud browser:** in the portal and each Craft cloud window, navigate
  nested personal/shared folders using breadcrumbs and back/up; filter/sort beyond
  one listing page; create and rename folders/files; move a file and a nonempty
  folder; trash/restore both. Check name collisions, legacy duplicates, invalid
  destinations, changed access, stale callbacks and denied read-only actions.
  IDs, bytes and revision history survive organization; activity records actor and
  sponsor. Move/rename an open Craft file from another client, then save its local
  edits without reverting its location/name. Trash it and verify draft preservation
  and denied publication. Real MCP exercises the same organization command cases;
  core browser tests require no MCP connection or LLM.

### Root cloud tests: independent of Craft and any LLM

Use disposable Alice (owner), Bob (collaborator), Eve (outsider), Agent G and an
unrelated agent credential. Alice has one shared folder and a private sibling.
Use small checked-in PNG, Markdown/text, empty, opaque-binary and supported Craft
fixtures. Check exact bytes/hash without opening an editor. General-format support
is a product requirement, not a workaround introduced merely to simplify the test.

1. **Canister suite:** use existing PocketIC infrastructure for the human scenarios,
   full operation/capability matrix, live sponsor/agent intersection, races, revocation, resource
   bounds, upgrade and recovery. Include direct calls to every exported protected
   endpoint and randomized grant/tree/state sequences with invariant assertions.
   Run the same organization fixtures as human and agent actors, then through the
   advertised MCP tools; compare canonical state, errors and attribution. Test IDs,
   history preservation, name collisions and stale tree versions independently of UI.
2. **MCP suite:** start the actual selected MCP implementation (existing ICP MCP
   or a justified dedicated adapter) and a real deterministic MCP protocol client
   against local IC. Negotiate the supported protocol, authenticate,
   discover tools, list/read the granted fixture and create a second file through
   the advertised tool; repeat with a PNG and Markdown note. Alice independently
   reads and verifies exact bytes, owner, revision, agent and sponsor. Direct
   invocation of a tool handler is not this test.
3. Repeat through that connection with Read, Read + Create, Read + Edit, Propose and
   Delete; multiple independent scopes; approve/discard; sponsor downgrade/revocation;
   disconnect during transfers; credential renewal/rotation; forged IDs; retries.
   Count and hash actual backend objects, not only success messages.
4. Exercise the selected consent/linking flow in a browser against disposable local
   II where supported, plus adversarial credential/callback tests. Include OAuth
   tests wherever OAuth is part of that flow. PocketIC caller injection
   proves ACL behavior, not signature validation or real provider compatibility.
5. Repeat with Bob sponsoring the agent in Alice's shared folder, without an Alice
   delegation flag or approval. Bob selects Read + Propose; the agent submits and
   Bob approves under his Edit right. Reduce Bob to Read and verify denied proposal
   submission/approval; restore Bob's rights and check the stored agent ceiling.
   Revoke Bob's agent grant and verify later human regrant/token renewal cannot
   revive it. Give a separate explicit organization grant and exercise folder
   move/trash/restore, then repeat with each required capability missing.

Routine CI requires no provider subscription, model key or LLM decision. Keep local
test issuers and browser fixtures isolated; production build verification rejects
test-auth hooks. Record a separate real II flow and create/revoke smoke test from
both Claude and the OpenAI client(s) actually advertised as supported.

### Craft parity tests

One reusable conformance suite exercises the shared controller's intent/mode/async
state machine, including navigation, organization and destination selection. Each
real adapter additionally proves:

- Its own valid native document opens, edits/proposes, round-trips, previews and
  restores without losing format-specific content.
- Read-only/Propose guards work through menus, shortcuts and command dispatch;
  mode changes preserve buffers; a failed parse leaves the existing document intact.
- A save captures exactly one snapshot and marks only that revision clean. Account,
  dialog and document changes reject stale completions.
- Multi-document apps bind the right document. Single-document apps protect unsaved
  replacement. SoundCraft handles active recording without losing audio or pretending
  an uncaptured recording is saved.
- A22 runs in the portal and each app: the themed renderer exposes the same file
  management operations, and external rename/move/trash affects its open document
  binding correctly. Browser navigation and popup cancel/back preserve local work.

Run real portal/browser and per-app smoke checks. Mock Editor tests alone cannot
prove actual command guards, native serialization or visual parity.

### Gates for a single implementation task

- **G1 — Prove and freeze the risky boundary:** first record the current skill-led
  comparison of the existing ICP MCP route and any necessary extension. Prove
  actual MCP/II consent and a separately authenticated agent participant; zero grants →
  scoped create → revoke → denial. Prove owner and collaborator sponsorship. Pin
  protocol/auth components and typed schemas; document token/key protection across
  gateway, ingress, subnet execution, rotation and recovery. Verify any required
  confidential-subnet placement and guarantees separately from local functional
  tests. Measure shared read latency, a 64 MiB transfer/commit and worst-case permitted
  ACL/tree/subtree evaluation; set explicit policy limits. If the ICP path fails,
  document the blocker and revisit architecture before UI work or external hosting.
- **G2 — Cloud correctness:** A01–A08, A10–A14 and A18–A21 automated at the root,
  plus A22's organization API invariants. Existing backend/SDK/migration checks and
  real MCP round-trip pass.
- **G3 — Complete cloud experience:** all seven app cloud adapters, portal upload/
  download, folder navigation/organization and document modes; A15–A17 and A22 with
  real visual/browser observations and provider linking smoke. No headless Craft
  engine is required for drive MCP.
- **G4 — Release evidence:** no unresolved authorization/data-loss failures; inspect
  the implementation against the threat boundaries, verify recovery/operating
  instructions and record remaining limitations. UI review and security review
  must use evidence beyond the happy-path implementation tests.

Follow docs/development.md for scoped builds and SDK synchronization. Existing
test entry points include the service persistence and sdk_migration integration
tests, package unit tests, and root locale/shared-shell checks. Supply the real
baseline/current Wasm paths required by those tests. Add a documented root
collaboration-test entry point for the new suites; it is a deliverable, not a
command that exists today. Pin every changed app submodule to its tested commit.

Suggested implementation request:

> Implement docs/collaboration-spec.md in subnet-cloud. Complete G1 first and update
> the concrete transport/schema decisions with evidence. Continue through G2–G4,
> preserving existing data and unrelated work. Use the canonical shared packages,
> integrate all seven Craft web apps and run the independent real MCP tests.
> Repair failures, inspect the running UX, and report A01–A22 evidence, commands and
> limitations, with A09 explicitly deferred to the native integration milestone.
> Do not claim completion from compilation or mocks. Do not deploy to
> production. If a required external-account/usability check cannot be performed,
> finish independent work and report that exact remaining gate honestly.

One Astra task can perform many build/test/fix iterations. There is no guaranteed
conversion from a two-engineer estimate to an agent runtime or a reliable one-shot
generation. This is one release with staged evidence, not permission to omit UX.

## 9. Explicit exclusions

Organizations/groups, public anonymous links, deny/restricted-child policies,
assigned approvers/voting/comment threads, live co-editing, automatic offline merge,
autosave, ownership transfer, cross-canister federation, automatic user canisters,
E2EE, atomic multi-file acceptance/whole-run rollback, universal semantic diffs,
general remote editor control and seven native agent command adapters.
The first WordCraft native agent integration is specified as the next separate
milestone; the cloud-drive release does not depend on it.

Keep extension points, not speculative frameworks or administration screens.

## References

Local evidence: packages/sovereign-cloud/src/protocol.rs;
services/sovereign-cloud/src/{lib,store}.rs and tests/{persistence,sdk_migration}.rs;
packages/sovereign-cloud-ui/src/lib.rs and README.md; registry/apps.json;
the seven web-host cloud adapters; docs/development.md.

Native agent tooling: apps/wordcraft/docs/mcp.md, apps/gridcraft/docs/mcp.md,
apps/deckcraft/docs/mcp.md, apps/pdfcraft/docs/mcp.md, apps/photocraft/docs/mcp.md,
apps/soundcraft/docs/mcp.md and apps/cadcraft/README.md. WordCraft codec evidence:
apps/wordcraft/crates/engine/src/native.rs and io.rs, and the skipped media fields
in apps/wordcraft/crates/doc/src/lib.rs. Existing formatted-document protocol test:
apps/wordcraft/crates/mcp/src/tests.rs (not run as part of this design review).
Architecture rules: the corresponding app AGENTS.md files, inspected for their
command/engine separation and automation requirements.

- [PhotoCraft: Built for agents](https://github.com/storytold/photocraft#built-for-agents)
  and the adjacent architecture section, checked 14 October 2026 — existing shared
  command engine, headless operation and MCP support; upstream still labels the
  overall application early alpha, so architectural coverage is not a blanket
  maturity claim.

- [ICP canister security skill](https://skills.internetcomputer.org/skills/canister-security/)
  — method authorization, async/state safety and operational guidance.
- [ICP service-discoverability skill](https://skills.internetcomputer.org/skills/service-discoverability/)
  and [DFINITY ICP MCP](https://github.com/dfinity/imcp2) — discovery and existing identity flow.
- [ICP principals](https://docs.internetcomputer.org/concepts/principals/)
  and [Internet Identity](https://docs.internetcomputer.org/guides/authentication/internet-identity/).
- [MCP authorization](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)
  and [MCP security practices](https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/security_best_practices).
- [Claude custom connectors](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp),
  [ChatGPT custom MCP setup](https://developers.openai.com/api/docs/guides/custom-mcp-server),
  [OpenAI MCP authentication](https://developers.openai.com/plugins/build/auth)
  and [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).
  Recheck provider UI/account availability at release; custom setup is sufficient.
- [Google file access levels](https://support.google.com/docs/answer/16722399?hl=en)
  and [shared drives](https://support.google.com/a/users/answer/7212025?hl=en) — useful
  precedents for editor approval and the separate organizational ownership model.
