# Familiar cloud drive: architecture and product decisions

Updated 18 October 2026. Rust is selected for the cloud backend, SDK and Craft
integrations. The proposed Motoko replacement is withdrawn; its unused project
configuration has been removed. The feature assessment remains the roadmap.
The browsing, Suggested Home, transfer, email, navigation/recovery and UUID/scoped-storage milestones
are implemented locally. The current gap assessment and remaining stages are below; older dated sections retain
their original scope and evidence. Validation is recorded in [history](history.md). This roadmap does not
replace the evidence for the delivered [alpha](collaboration-spec.md).

## Scale-out direction — accepted design target (18 October 2026)

### Daily-file workflow completion — authorized scope (18 October 2026)

Finish the existing preview/thumbnail/Open-with, shared autosave/draft recovery,
sharing/request/notification and keyboard/mobile workflows, then reconcile all
36 feature groups with implementation and end-to-end evidence. Reuse the existing
catalog, grants, immutable revisions, upload receipts and shared editor adapter.
Do not rebuild editor-native functionality or introduce a second permission model.
The cached ICP canister-security and stable-memory skills above remain the standard
for authenticated updates, bounded records, session isolation and safe upgrades.
The gap in this stage is workflow completion and verification, not new storage
infrastructure. Add only focused UI/controller corrections demonstrated by tests.

Content search, OCR, semantic search and ML are explicitly outside this milestone.
Preserve canonical object/space/revision boundaries and current authorization so
future derived search data can be versioned, invalidated and rebuilt. Do not add
a naive whole-catalog/content scan or preselect an indexing/inference engine now.
Search needs a separate design covering 100,000 users, large data volumes, bounded
work, freshness and useful results; keyword and AI layers may be complementary.
This is a design target, not evidence of measured production capacity.

Acceptance includes real browser preview rendering, two-account sharing and
revocation, shared-adapter autosave/reload/recovery, keyboard/mobile navigation,
and focused regression tests. The 36-row HTML review distinguishes delivered core
flows, explicit limits, unfinished subfeatures and deferred product decisions.

Requirement: design for 100,000 users and later growth by adding serving/storage
capacity, without implementing multiple data canisters now. All application data
stays in one platform-owned canister initially, including the move to Cloud Engine.
The target is an architectural constraint, not a measured capacity claim or a
requirement that this development build already serve that load. Account count,
files per account, total content bytes, concurrent sessions and read/write rates
must be measured separately. Moving hosting does not remove application-level
whole-tree work, contention or query invalidation.

Contracts to establish in the single-canister implementation:

1. Logical account/space ownership determines data boundaries. Many accounts may
   share one canister; an account is not inherently a canister. Keep content
   placement behind the revision/transfer API so a large space can later use
   multiple blob stores without changing its catalog identity.
2. Canonical object IDs and references remain independent of physical deployment.
   Reuse the SDK's endpoint/transport boundary for resolution. Today resolution
   selects the configured canister; do not introduce a directory service, factory
   or generic routing framework merely to simulate future deployment.
3. Ordinary reads and mutations touch the object, relevant ancestor path and
   necessary indexed records, not every object belonging to the owner. Large
   subtree operations explicitly bound work and can become resumable jobs; they
   must not silently turn an ordinary command into an unbounded transaction.
4. Listings use scoped indexes and bounded pagination. Writes by an unrelated
   account must not invalidate another account's listing cursor. Version scope
   follows affected folders/spaces or the viewer's discovery state. Recipient
   indexes support Shared without scanning every account or serving canister.
5. Local commands maintain canonical records and derived indexes together, with
   explicit object versions and retry receipts. Future cross-canister discovery
   updates and relocation must handle partial failure and stale references; they
   must not assume a transaction spanning independent canisters. Serving storage
   always checks current authority; discovery indexes never grant access.
6. Keep account/space data, viewer-private state, discovery and placement as
   distinct responsibilities in existing modules. Add interfaces where a real
   dependency boundary needs them, not a new framework per feature. Future
   placement/discovery indexes must themselves be partitionable rather than a
   single mandatory write bottleneck for all users.

Gaps identified before this milestone: owner-wide graph reconstruction for mutations, browse candidate
materialization before sorting, a shared catalog revision used by cursors, and
service-local numeric object IDs. The smallest current extension is to correct
these paths and the canonical ID contract within the existing Rust backend and
SDK. No multi-canister deployment, ownership provisioning, data relocation or
automatic load balancing is claimed or required in this stage.

Proof: add data-distribution fixtures and operation-cost checks showing that a
fixed account's ordinary operations do not scan unrelated accounts, and that
one-file mutations do not reconstruct all of that owner's files. Exercise large
folders, shared roots, scoped cursor validity, retries, revocation and persistence.
Measure stable reads/writes, Wasm instructions and memory as well as latency.
Benchmark representative small and large accounts in a 100,000-account dataset;
separate these dataset tests from concurrent load tests on the target hosting.
Record assumptions and results before making capacity claims. The object-scope
checks and optimizations below are complete; the 100,000-account dataset and
target-hosting load benchmarks remain deferred.

Architecture evidence: the session's cached ICP
[stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md),
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [multi-canister](https://skills.internetcomputer.org/.well-known/skills/multi-canister/SKILL.md)
guides were reviewed on 18 October. Stable indexed records, authenticated Candid
methods and independent deployments supply the standard building blocks. The
gap is the application's operation scope and future placement/discovery contracts,
not evidence of a missing ICP storage platform. Distributed routing remains
deferred; the existing modules are the implementation boundary today.

## UUID/scoped-storage milestone — locally tested (18 October 2026)

Implemented in the canonical Rust service, SDK, shared controller/picker and portal:
UUID text addresses with sixteen-byte stable keys; object/path changes prepared
before writes; direct name and ordered catalog indexes; folder/space/ACL/viewer
versions; and bounded query continuations. Editor-native code and the `Editor`
trait are unchanged. One shared adapter snapshot is compiled into the eight hosts.
No additional canister, routing service or numeric-ID translation is introduced.

Queries inspect at most 512 candidates and return at most 50 matches. An empty
page can carry a continuation, and the UI keeps the next-page action available.
Discovery permits 64 source spaces/256 direct shared roots before requiring a
narrower view. Folder prefetch inspects at most 200 records. Suggestions rank up
to 150 candidates from bounded modified/starred/recent windows, not the whole
catalog. Recent opens and dismissals each retain 200 private summaries; their
updates no longer scan unrelated stars or folder colors. Subtree/aggregate safety
bounds are explicit, with resumable jobs and distributed aggregates deferred.

Validation: 68 service unit tests, 19 SDK tests, 12 shared UI tests and 15 actual
Wasm/PocketIC contract, permission and populated-upgrade flows. The identity test
generates 100,000 distinct UUIDs; this is not a 100,000-user load test. Storage-cost
fixtures compare an ordinary rename with 200 and 5,000 same-owner objects and
reject proportional whole-account work. Adversarial tests cover inaccessible
filtered pages, tampering, cross-account bookmarks, independent folder versions,
revocation and persistence. Browser/deployment evidence is recorded separately in
history after verification. Production throughput and multi-canister hosting
remain unmeasured and unimplemented.

## Permanent file addresses — implemented locally (18 October 2026)

Implementation started after owner authorization on 18 October. Use the standard
`uuid` crate's v4 builder over ChaCha20 output seeded by ICP `raw_rand`, following
the [ICP randomness guide](https://docs.internetcomputer.org/guides/backends/randomness/)
and [UUID builder contract](https://docs.rs/uuid/latest/uuid/struct.Builder.html),
checked on 18 October. The shared ObjectId type uses UUID text in Candid/JSON and
16-byte stable keys; numeric revision/upload/event identifiers are distinct.
Initialize entropy before allocating, recheck state after the asynchronous call,
and seed afresh after upgrades. No custom random generator or UUID alias table.
The current-to-new schema is a deliberate fresh installation, while subsequent
same-schema upgrades must preserve objects and retry receipts.

Editor boundary clarified by the owner: UUID changes belong in the canonical SDK,
shared controller and picker. The Editor trait's local document IDs remain local
and unchanged. The eight editors statically embed pinned adapter snapshots, so
adoption needs builds/integration checks, not eight independent implementations
or full editor feature test suites. Do not leak cloud identity into editor engines.

Requirement: files and folders need one globally unique ID used both in storage
and in links. The owner clarified that a separate public-ID-to-numeric-key alias
layer is unwanted. The existing `#file/<u64>` and `#folder/<u64>` addresses are not
the long-term external identity contract.

The owner reopened the URL choice on 18 October, prioritizing stable identity,
efficient shared listings, flexible placement and minimal additional state.
Accepted: one immutable random UUID per catalog object,
with `/pilveruum/file/<object-uuid>` as an illustrative file address. A copy receives
a new identity; rename, folder move, revision, trash/restore and a future ownership
transfer retain it. A shortcut has its own identity and references its target.
The UUID is the canonical object key, not an alias over an incrementing number.
An ID is not a bearer permission.

Keep ownership in the canonical object/space metadata and physical placement at
the storage boundary. Today every ID resolves directly in the configured shared
canister's existing object map: no separate location table, directory canister,
factory, proxy or multi-canister transport is required. All entry points (links,
folders, Shared, Recent, Starred, search, editors and MCP) use the same object ID
and authorization evaluator.

Future constraint, not an implemented capability: a random opaque ID does not
encode an independently changing location. If metadata and content later move to
arbitrary shared or user-owned canisters, stable links require a location lookup
or a stable home that forwards them. A minimal directory could map object IDs to
logical storage spaces, with space placement resolving the serving canister.
This would be new placement information, not a second file identity. Such a
directory can be partitioned by ID; it must not become a full central copy of
private metadata or the authority for access. Owner-based partitioning alone
cannot resolve an owner-free URL until the owner/space has first been found.
Do not claim an account placement map solves that missing lookup.

Alternative previously proposed: `/pilveruum/<owner-principal>/<object-uuid>`.
It avoids a per-file global location directory while placement remains account
based, but still needs account-to-canister resolution and cannot preserve the same
canonical URL through owner changes without additional forwarding. An immutable
namespace instead of the owner is another tradeoff, not the current owner and not
a way to eliminate all location state. Packing two components into one URL token
does not simplify the underlying model. Do not freeze a distributed routing
scheme before implementing and testing that deployment boundary.

Preserve owner, parent and recipient indexes for bounded browsing. The existing
`read_subjects` index finds directly shared roots; inherited folder access should
not materialize one grant per descendant and recipient. Indexes are maintained
with canonical changes and every returned object rechecks current access. A
future cross-canister recipient index needs authenticated, retryable updates and
source authorization checks; a cached discovery reference never grants access.
The initial scaling review found that `graph_for_object` loaded an owner graph
and general browse queries collected candidates before sorting. The scoped-storage
milestone now loads object paths and seeks ordered indexes with bounded scans.
Changing IDs alone would not have removed those costs. Large subtree operations
remain explicitly bounded; resumable jobs and distributed execution are deferred.

The application is still private development, and its local data is disposable.
The owner explicitly rejected transition layers: do not preserve numeric links,
add numeric-to-UUID mappings, retain compatibility endpoints or backfill old test
records. Replace the object-key contract coherently across catalog storage,
grants, revisions, shortcuts, activity, SDKs, editors and generated links. Adopt
the new schema with fresh local test state. The authorized local reinstall is now
complete, without converting the old numeric data. Persistence and
populated upgrade tests still apply to the resulting schema going forward.

Skills: [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
were consulted this session; [static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
was retrieved on 18 October 2026 for this requirement. The cached
[multi-canister](https://skills.internetcomputer.org/.well-known/skills/multi-canister/SKILL.md)
guide was reviewed on 18 October for the future placement boundary: ordinary
canister calls support it, but cross-canister writes are not one atomic mutation.
Standard certified-assets
`200` rewrites support a scoped cloud-space shell route without a custom HTTP
server. The platform supplies persistence, caller identity and randomness; public
file identity and account placement are application domain data. No new cryptographic
scheme is needed for file identity. Canonical UUID allocation, the storage/API
contract and routes are implemented; validation and deployment evidence are in
[history](history.md). Opaque browse continuations separately use the standard
authenticated-encryption construction documented below.

Reference checked on 18 October: Google supports
[ownership transfer](https://developers.google.com/workspace/drive/api/guides/transfer-file)
subject to account restrictions and documents
[stable opaque file IDs](https://developers.google.com/workspace/drive/api/guides/about-files).
Those public contracts do not reveal Google's internal routing architecture.

Proof required: unique concurrent allocation and retry stability, populated
upgrade preservation of the new schema, copy versus move identity, revoked/expired/blocked access,
uniform inaccessible responses, sign-in return, fresh-load/reload/back/forward,
ET/EN links, email and clipboard links, and storage relocation retaining canonical IDs.

## Navigation, details and recovery — delivered locally (18 October 2026)

The owner accepted this next stage after the transfer and email milestones. The
HTML feature review is the current comparison; the earlier assessment below is
historical. Keep the single data canister and the existing Rust domain/SDK/UI.

Delivered and verified: 52 backend unit tests, 113 JavaScript tests, four
localization tests, a populated old-to-current Wasm upgrade test, browser workflows
and served-asset checks. No stable schema or memory IDs were added. The final
local code-only upgrade preserved configured email. See [history](history.md) for
commands, evidence and limits.

Scope:

1. Expandable sidebar folders, paginated and loaded on demand, with keyboard
   navigation and account-scoped transient expansion state. Reuse catalog browsing;
   do not fetch the whole account or create a separate folder database.
2. A file details panel combining authorized metadata, revision information and
   the caller's effective access/provenance. Keep the complete recipient roster
   owner-only. Do not reveal inaccessible ancestors or other people's email addresses.
3. Undo rename, move and Trash as a fresh authorized operation. Recheck the current
   object version, destination and access impact. Start with the most recent action
   in this account/tab; reuse mutation receipts and ordinary commands. No snapshot
   overwrite, ACL rollback, permanent-delete undo or general event-sourcing system.
4. Restore a trashed item with an explicit alternative name/destination when its
   original location is unavailable or occupied. Reuse move validation and access
   impact review; never overwrite another object or silently expand access.

Architecture evidence: the session skill index and cached
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
guides were reviewed on 18 October. Standard support is authenticated Candid
methods, explicit authorization, optimistic versions and stable structures with
stable memory IDs. Existing browse, organization, move-impact and revision APIs
cover most requirements. Observed gaps are the sidebar/panel UI, a caller-scoped
access explanation, Undo orchestration and restore with a different destination
or name. Add only these cohesive extensions. Physical service placement stays in
the SDK/service boundary; no canister factory, sharding or routing table is needed.

Proof: cover lazy folder pagination, hidden ancestors, account switching, stale
Undo after another edit, revoked authority, missing/occupied restore destinations,
access-impact confirmation and partial batch retries. Verify any added persistent
schema with a populated upgrade. An in-memory UI view must not become an authority
or duplicate the canonical catalog/grants.

Later stages remain independent: activity history with explicit visibility and
retention; shortcuts and sharing conveniences; previews; revision retention before
autosave/draft recovery; full-text extraction/indexing; comments and live editing;
offline/sync and team ownership. Activity events describe successful mutations and
do not replace the catalog or collect private opening history into a shared log.
Google's [details/activity](https://support.google.com/drive/answer/2409045?hl=en)
and [keyboard commands](https://support.google.com/drive/answer/2563044?co=GENIE.Platform%3DDesktop&hl=en)
document the corresponding familiar UX, not an implementation architecture.

Known email polish: the current rate-limit error conflates the verification
cooldown with daily account/address/service budgets. A follow-up should return a
safe reason and retry time, distinguish accepted requests from delivery outcomes,
and present the actual wait. The existing bounds are application anti-abuse policy,
not ICP capacity. No provider/account limit should be inferred from this message.

## Settled direction

### All 36 feature groups — authorized continuation (18 October 2026)

The owner authorized the ten “Seejärel” groups as the next implementation stage
and all 36 groups as the overall product target. “Olemas” continues to mean a
tested core workflow, not every subfeature in a row. Larger stages remain on the
plan; AI still follows the agreed Intelligence Gateway integration, and desktop
sync, team administration and live collaboration require their own complete
flows. Do not silently replace those with placeholders or mark them delivered.

Dependency order: (1) mutation activity, revision controls and storage breakdown;
(2) personal organization and sharing workflows; (3) previews/accessibility;
(4) shared save/draft recovery in the eight editors. Retention policy precedes
automatic saves. Each change reuses canonical catalog, authorization and upload
receipts. No user-canister provisioning or sharding is introduced by these UX
features.

Architecture evidence before implementation: requirement is durable, bounded
mutation history and owner-controlled version/storage management. The session's
cached ICP [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
were reviewed again on 18 October. Their standard approach is authenticated
methods, authorization inside the update, stable structures with unique IDs,
validation before publishing mutations, and populated upgrade tests. The service
already provides these plus revision/chunk accounting and deduplicated commands;
the gap is the domain record/projection and user controls. The smallest extension
is an activity module (IDs 36–38), a revision-metadata module (ID 39), and a derived
storage view. No external index, generic event bus or second ACL model is needed.

Proposed activity contract: append successful mutations once, inside the same
message as their ordinary receipt. Keep the latest 100 events per object. Reads
recheck current access and use current object names; sharing/retention changes
are owner-only. Store no email, old names, old parent names, read/open events or
credential secrets. Purging an object removes its activity. Version metadata is
separate from content hashes and immutable bytes. Storage totals derive from
existing ownership/chunk reservations, with current content, old revisions,
Trash and pending uploads presented distinctly. Validate old-to-new upgrade,
exact retries, current ACL filtering, retained versions and quota conservation.

The owner accepted retention defaults on 18 October: named/pinned and manual
versions are never automatically removed; retain the latest 20 unpinned automatic
checkpoints. Owner removal is explicit and cannot delete the current revision.

Personal organization uses the same catalog: a Shortcut is an explicit object
kind with its own parent/owner and a target ID, never a content copy or a grant.
Following it rechecks both shortcut and target authority. Folder colors remain
caller-private optional browse state. No target bytes, recipients or protected
ancestor names are copied into shortcuts. Older clients require the updated SDK
before shortcuts are enabled; adopt the shared contract across the eight editors.
Shortcuts cannot target shortcuts, preventing chains/cycles. Ordinary move/Trash,
version checks and retries act on the shortcut itself.

Account communication uses separate bounded stable structures (40–45): inbox,
requests, private contact labels and notification preferences. Request access
returns no object/owner metadata and uses the same requester budget for existing
and missing IDs. Only the current owner can approve; approval creates an ordinary
grant and never a parallel permission. Notices contain IDs and resolve current
names behind the normal authorization check. Keep 100 notices per recipient;
evict associated old requests together. Retry tests cover lost responses, owner
checks, request expiry and revoked names. No external service is required.

Recipient controls (46–48) keep hidden IDs, directional block choices and reports
separate. A block applies bidirectionally through the existing access evaluator;
read-path loading derives blocked owner/recipient pairs only for relevant grants.
Stored grants survive and become effective again after unblocking, as the UI states.
Hide only filters aggregate views, not direct access or content ownership. Reports
are bounded per account and readable only through a controller-checked endpoint;
they contain reason/account/object IDs, never file bytes. Proof: expired/revoked
reads, both directions of existing grants, new grants, notifications, upgrades,
and hiding without deletion. No claim of automatic moderation is made.

The sharing dialog now offers the same Viewer/Editor presets for principal IDs
and email recipients (owner feedback, 18 October). Editor includes reversible
Trash and folder Create; only owners can purge or change sharing. The canonical
capability evaluator remains intact for least-privilege APIs/agent use. Existing
non-preset grants are displayed as disabled “Custom access” until an owner
explicitly chooses a replacement; opening a dialog never broadens them.
Commenter is deferred with actual commenting, not offered as a nonfunctional role.

Previews remain derived browser UI: native PDF rendering, bounded OOXML text and
cached values, ephemeral image thumbnails, and a format matrix that distinguishes
preview from native editing/import. Requirements: no external document service,
no active HTML/Office code, no second content database. Primary sources reviewed
18 October: [fflate 0.8.2](https://github.com/101arrowz/fflate/tree/v0.8.2)
for an MIT-licensed browser ZIP reader and [MDN iframe](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe)
for native PDF embedding. The standard library closes ZIP decoding; our extension
only allowlists OOXML parts and renders inert text. The pinned browser module hash
is `8cc1f687e0159e977addb6b85e274dbd11e622cf151f4fcb7b85d49622ea43e7`.
Sheet coordinates and workbook/presentation relationships control display order;
no macros, external relationships, layout engine or formula evaluator is added.
Tests cover XML entities, ZIP bounds, format routing and cancellation; actual
browser previews are required before marking this flow delivered.

Automatic saves use the same expected-revision upload and retry receipts as manual
Save. They only update an explicitly opened/saved editable cloud binding in the
current account, after five quiet seconds and at most once per thirty seconds.
Conflicts preserve the captured snapshot for explicit retry or Save As. Native
adapters still own encoding and dirty revisions. Local drafts use standard
IndexedDB, with metadata and bytes in one transaction, scoped by backend canister,
principal and app. Their metadata is not uploaded. They require explicit restore
as an unbound copy; no automatic overwrite after browser recovery. Retain up to
20 drafts / 128 MiB per app/account in this browser, with explicit removal and
visible storage failures. Browser eviction is outside cloud durability. Read/delete
compare the selected snapshot to protect against another tab updating it.

Implementation and final defaults remain marked in the dated history; the HTML
review only advances after the corresponding end-to-end flow passes.

- Familiar file navigation, selection, search, sharing, transfers and recovery;
  original implementation and Subnet visual identity.
- One Rust implementation and canonical domain API shared by the portal, editors
  and authorized agents. Improve cohesive modules as features are added.
- Plan for SEV-enabled hosting, with the actual subnet and protection path verified
  before making deployment confidentiality claims. No client-side encryption for
  this release; authorized backend operations can read contents. This is a product
  decision, not evidence that the current deployment already has that protection.
- Email support is authorized through the user's Resend account. Optional contacts
  are verified by Subnet, independently of Internet Identity. A principal may have
  multiple addresses; each address has one current principal. Proven transfer
  requires explicit confirmation and changes only future address-based sharing.
  Principal ACLs and existing files never follow an email address automatically.
- All application data stays in one platform-owned data canister now and initially
  on Cloud Engine. Platform-managed sharding and optional user-owned data canisters
  are future extensions, not requirements for the next feature milestone. Normal
  pagination, bounded operations and chunking remain engineering requirements.
- Embedded conversation comes later through ICP Intelligence Gateway. No interim
  model-provider integration, conversation subsystem or gateway placeholder.
- Existing deployment data is disposable if a deliberate storage redesign needs
  fresh state. This is not an instruction to reset a deployment now. Once the new
  product is in use, upgrade safety is a permanent acceptance requirement.

## Architectural defaults

Keep the current canister boundary and explicit internal modules: accounts/profile,
catalog, authorization/sharing, content/revisions/transfers, metadata search and
user preferences. Introduce invitation delivery only with its feature. Thin Candid
endpoints resolve caller identity and invoke domain operations. The existing shared
Rust SDK owns transport/session/transfer behavior; the shared picker/controller
owns common UI workflows. Editor adapters retain native format and dirty-state logic.

Avoid a generic repository framework or microservices merely to make the code look
layered. Extract a boundary when it owns a distinct invariant or external service.
Split portal selection, navigation, actions and transfer state into focused modules
as those workflows change. Preserve the existing localization and accessibility.

Use one authoritative catalog and grant model. Indexes are derived and rebuildable,
never alternate permission databases. The current store has an access-graph record
and persistent read indexes; any simplification must first identify their writers
and prove consistency through mutation, upgrade and interrupted index rebuild.
Do not add a third copy of this state for search.

Continue `ic-stable-structures`, with explicit record encodings and schema versions,
stable memory-ID/key assignments, and deliberate index migrations. No whole-heap
serialization at upgrade. An isolated PocketIC test must upgrade populated state
across a real schema change and verify permissions, content, pending uploads and
retry results. A same-Wasm redeploy is insufficient evidence of schema safety.

Validate permissions, expected versions and arguments before publication. Ordinary
returned errors do not roll back preceding writes. Keep file/grant publication
within one message where possible; external delivery and inference happen outside
that mutation. Durable operation IDs, immutable revisions and explicit per-item
results support retries, bulk operations and future AI actions.

Keep logical space/file identity conceptually separate from service location.
The current endpoint configuration serves one backend; dynamic routing, distributed
search, cross-canister moves and identity-directory federation are deferred.

### Single-canister hosting, future placement and minimal suggestions

Owner clarification, 17 October 2026: keep all application data in one
platform-owned data canister, including the initial move to Cloud Engine. Do not
implement user-owned storage or multi-canister routing in this milestone. Preserve
the ability to extend the same application in two directions later:

- Add platform-owned data canisters when aggregate storage requires it.
- Offer an optional data canister provisioned by the platform and owned by the
  user, alongside the common service. Provisioning, controller/upgrade policy and
  the user's future large-file requirements will be designed in that workstream.

These are future capabilities, not existing support or prerequisites for Suggested
Home. Feature work should respect the existing data boundaries and collect only
metadata needed for the feature.

Keep three concepts independent: the authenticated principal (who acts), the
logical space (which catalog and permissions apply), and its storage location
(where its records/content are stored). A space is the file-tree/permission
boundary, not a new identity or a physical canister. Avoid assuming one principal
or one space must always equal one canister. Ownership of a file and control of its
hosting canister are separate concepts.

Evidence at the earlier placement review: `ServiceEndpoint` separates deployment configuration from file
references, and the canonical evaluator authenticates callers and enforces
ownership/grants. However, `CloudLaunch` accepts only `cloud_service=cloud`, the
browser facade constructs one configured client, item IDs are locally numeric,
viewer keys are `(principal, local_object_id)`, and browse cursors use a shared
catalog revision. These support today's deployment; they do not prove independent
placement or safe aggregation of multiple canisters. Do not describe it as already
supporting user-owned storage.

Keep endpoint knowledge in the SDK/transport and storage access in the canonical
backend modules. Reuse existing space, owner and permission concepts; no placement
table, routing framework or second canister is needed now. The 18 October identity
discussion above supersedes the earlier decision to retain local numeric IDs:
canonical UUIDs are now implemented without a numeric compatibility layer. Preserve
explicit ownership of records and schema versions so future placement work can
identify what belongs together.

Future placement work should preserve logical references and sharing links while
resolving storage through trusted configuration. Globally unique canonical IDs
must be established before combining independent catalogs.
Partitioning whole spaces may be a useful first step, but is not a permanent
constraint: a large space may eventually need content spread across storage
canisters. Keep content access behind the existing revision/transfer API so that
physical blob placement can evolve separately from the file tree. Exact partition
keys, routing records and migration protocols remain undecided.

Use the canonical Rust implementation and versioned domain API as the basis for
both future hosting options. Keep ordinary mutations local where possible. When
cross-canister operations are introduced, design resumable transfers, verified
completion and idempotent retries around partial failures. Preserve the II
derivation origin when relocating backend data, and authenticate at the serving
endpoint; a forwarded principal argument cannot become authority. No factory,
directory, provisioning, relocation or multi-endpoint client is needed now.

Keep the data responsibilities distinct, initially as modules, not new canisters:

- **Space data:** canonical file/folder metadata, revisions, content and grants
  retain their logical space/owner association. Operation receipts stay associated
  with the operation's authority and affected records. Existing indexes may serve
  the shared canister; the 18 October scale-out target brings scoped cursor revisions
  into the single-canister optimization work, ahead of sharding. Access checks
  occur at the source for every query and action,
  including suggestions.
- **Private user data:** preferences, stars, recent access, suggestion dismissals
  and references to accessible spaces belong to the viewer. Opening
  somebody else's shared file must not expose the viewer's private usage summary
  to that file owner through the application API. Viewer state stays in the common
  backend now. Keep its ownership distinct from file ownership; its placement and
  portability can be designed with the future user-owned-storage feature.
- **Derived views:** search results, suggestion ranks and display explanations
  are computed from authorized facts. They confer no permissions and are not a
  second authoritative file catalog. Future routing should not require a duplicate
  central catalog or a global behavioral profile.

Suggested Home MVP should reuse existing modification/revision facts and private
stars/last-opened timestamps. The only proposed additions per viewer/item are
the revision last explicitly opened and an optional `hidden_until` for dismissal.
Record explicit folder visits in the same access summary; metadata listing,
prefetching, background refresh, previews generated in the background and retries
are not visits. Record a file revision as seen only after its intended foreground
open succeeds. Coalesce duplicate acknowledgements and use server time. An
activity update failure must not prevent access to the document.

Compute a deterministic mix of recently used items, recent revisions of items
the viewer uses, and starred items. Return a truthful reason code with the result;
do not persist scores or localized explanations. Keep Recent chronological and
Home stable while a user interacts. No lifetime open counter, raw clickstream,
dwell tracking, embeddings or inferred interests are needed. Frequency can be a
later bounded aggregate if it demonstrably improves selection; newly shared
ranking waits for reliable sharing timestamps. Do not retrofit timestamps from
unrelated fields or claim an item was edited based only on a rename timestamp.
For an account with no activity, use authorized newly created items as an explicit
fallback. Derived state can be discarded; intentional stars/preferences must be
preserved. Expired dismissals and obsolete activity need bounded cleanup.

A future AI ranker can consume the same authorized candidates and, when that
feature is introduced, explicit content-extraction APIs. It must not become the
owner of catalog data or a prerequisite for listing/opening files. MVP telemetry
does not need to anticipate or collect a complete future AI training history.

Next-milestone acceptance proof uses the current single data canister:

1. Multiple principals retain isolated preferences/activity. Suggestions use the
   canonical permission evaluator; revocation removes items from Home, Recent,
   Starred and search after revalidation. No private ancestor or another viewer's
   usage is exposed. Metadata preferences never grant access.
2. Missing or duplicate activity acknowledgements cannot corrupt content or create
   false revision/usage claims. Ranking tests cover time, dismissal, deterministic
   ordering, empty history and access; browser flows distinguish successful opens
   from background listing/refresh.
3. Any new persisted metadata survives a populated-state schema upgrade together
   with existing content, grants, preferences and operation receipts.

Only when multi-canister support is commissioned, add proofs for independent
canonical-ID allocation, authenticated routing, cross-endpoint sharing/revocation,
relocation with stable references and partial endpoint failure. These distributed
proofs are deferred. Bounded operations and scoped pagination are now part of the
accepted single-canister design target above.

Architecture evidence (17 October): the session-cached skill index was reused;
[multi-canister](https://skills.internetcomputer.org/.well-known/skills/multi-canister/SKILL.md)
and [internet-identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md)
were retrieved for this review; the already-consulted
[stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
and [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
remain applicable. Standard support is versioned Candid APIs, authenticated callers,
stable structures and independent canister deployments. The current single-service
addressing and state scoping satisfy the present hosting requirement. The future
gap is placement/routing and migration across independent stores; no platform
limitation is inferred from the application's present implementation. The smallest
extension now is feature-specific viewer metadata in the existing store, with the
boundaries above preserved; distributed infrastructure is deferred.
The [message-execution reference](https://docs.internetcomputer.org/references/message-execution-properties/)
and [inter-canister guidance](https://docs.internetcomputer.org/guides/security/inter-canister-calls/)
confirm why multi-canister mutations need explicit partial-failure handling.
Rejected alternatives are a global mirrored catalog/behavior log, a universal
proxy and a factory built before it is needed. Cross-canister discovery, complete
global search/pagination and user-controlled upgrade/controller policies still
need separate implementation proofs when that work begins. Deferring them keeps
the present service simpler while accepting a deliberate future schema/client
migration; this design does not promise that adding a canister will be automatic.

## Sharing now: principal-based, with less friction

Expose a copyable **My sharing ID** for this application's principal, familiar
Viewer/Editor presets and readable labels for known collaborators. Labels are not
unique identifiers or evidence of identity. Remember previous collaborators within
the user's account rather than exposing a global principal/email directory.

Pair grants with stable restricted file/folder links. A link points to an item;
it does not grant access. The recipient signs in to this app and returns to that
item. A future in-app request-access flow can convey the authenticated principal
to the owner without an email integration. No email/provider setup is a prerequisite.

## Sharing by email

Email was deferred during the navigation/transfer milestones. The owner authorized
it on 18 October; the current implementation contract and evidence are in
[Email sharing](#email-sharing--implementation-contract-18-oct-2026) below. That
contract supersedes the earlier II-attribute proposal: contacts are verified at
Subnet account level and do not require modifying an Internet Identity account.

## Recommended product defaults

- **Roles:** Viewer = Read; Editor = Read/Edit/Delete and Create on folders.
  Delete means recoverable Trash, not permanent purge. Only the owner changes
  sharing or purges. Keep Advanced capabilities and explain inherited access.
  Roles map to the existing evaluator, not a second authorization system.
- **Links:** Restricted by default, stable through rename/move, sign-in returns to
  the item, and a request-access flow. Defer anonymous/public links for this release.
- **Search:** names plus type, owner, modified date and location; all accessible
  files, including unloaded folders. Use normalized metadata and ordered indexes
  in the existing canister. Prefix/ordered indexes do not solve arbitrary substring
  search; begin with bounded matching and measure before adding token/ngram indexes.
  Enforce access before exposing results, counts or paths. Preserve metadata trust.
- **Ownership:** a shared folder and new files inside it belong to its space.
  Starred, Recent and view/sort preferences belong to each viewer.
- **Moves and inherited access:** show the access changes before a move. A move
  that changes the effective audience needs sharing authority, including inherited
  changes to descendants; ordinary edit/move rights must not bypass owner-only
  sharing. Preserve explicit grants and reevaluate inherited grants at the new
  location. This is a proposed policy extension requiring acceptance tests.
- **Collisions and copies:** explicit replace/keep-both/skip, no silent overwrite;
  copies have new identity and follow destination access, with a clear ownership
  indication. Replace creates a revision of the existing file and requires its
  edit authority; merging folders is a separate explicit action. Cross-space
  operations start as copy, not an implicit ownership move.
- **Recovery:** no automatic Trash expiry initially; permanent deletion is an
  explicit owner action. Restoring old content creates a new revision. Undo rechecks
  versions and authority. Decide checkpoint frequency and bounded history retention
  before autosave, not before file browsing. Conflicting edits preserve a recoverable
  draft rather than silently overwriting another revision. Local drafts remain
  account-scoped and recoverable after failures and account switches.
- **Formats:** preserve uploaded file bytes. Publish an explicit matrix of supported
  preview, native editing, import and export formats for each Craft app; unsupported
  files remain downloadable. Test round-trip fidelity for each promised format.

The owner accepted these defaults on 17 October 2026 and authorized the first
milestone: navigation, selection, list/grid, Recent, Starred, search and sorting.
Detailed autosave retention and the exact format matrix belong to the later editor
milestone. Destination deployment is out of scope; expect SEV-enabled hosting.

### First milestone implementation contract

Use the existing per-object stable records and parent/owner/grant indexes (the old
graph cell is migration-only). A dedicated browse module resolves authorized
candidates, applies normalized filename/type/owner/date/location filters and sorts
the complete result before pagination. Reuse the canonical permission evaluator;
never search only browser-loaded rows. Working sets retain the existing 10,000
object bound; no external database, duplicate permission store or search service.
If that deployment bound is lifted later, extend the retrieval implementation behind
the same contract with bounded index traversal. Do not silently truncate results.

Recent means files explicitly opened by this account, newest first by default;
background metadata refreshes do not count as opens. Stars, recent timestamps and
view/sort settings are account-private stable records in new memory regions. They
confer no access, and inaccessible, trashed or purged objects disappear from results.
All metadata remains consensus-read. Bind cursors to caller, filters, sort, catalog
and viewer-state revisions; restart after invalidation instead of mixing pages.

The portal owns presentation and transient selection only. Use a small tested
selection/navigation module, account-scoped server preferences, accessible controls,
single-click selection and double-click/Enter opening. Shift-range, Ctrl/Cmd-toggle,
select-all-loaded, Escape, keyboard navigation and action menus share one selection
model. Keep the embedded picker on the same browse path with its own selection rules.

Evidence before coding, 17 October 2026: the cached session index and the linked
stable-memory, canister-security and service-discoverability skills below were
consulted. Standard approach: stable structures, authenticated Candid methods and
accurate API behavior docs. Observed gap: ID-ordered pages and no private viewer
state. Smallest extension: one browse domain module and viewer-state storage, shared
types/facade and a selection module. Alternative external search is unnecessary for
this metadata milestone. Proof: sorted multipage results, unloaded shared descendants,
revocation/expiry and cursor isolation, per-account preferences, real schema upgrade,
browser navigation/selection/search/layout and localization. No production hosting change.

Implemented on 17 October 2026: `store/browse.rs` owns metadata retrieval and
private viewer state; stable regions 24/25 hold viewer items/preferences without
changing existing region assignments. `portal/drive-browser.js` owns transient
selection and route parsing. The Rust facade and portal use `drive_query` for
globally sorted pages and `drive_browser_home` for the initial account/root page.
All permission decisions still use the existing evaluator. No new dependency,
search service, editor fork change or alternate catalog was introduced.

Recent retains the latest 200 explicit file opens. Since the Suggested Home update,
the portal and all eight rebuilt Craft clients acknowledge successful foreground
opens, including direct editor launches. Older independently pinned clients retain
their earlier behavior until updated. Metadata reads are not opens. Search covers names and metadata,
not file contents or semantic/AI search. Selection supports stars, per-file downloads,
same-space moves, principal-based sharing, recoverable Trash and restore. Rename
applies to one item. Folder downloads and folder/multi-file copies remain later
milestones; multi-file downloads use one ZIP archive. The Suggested Home milestone
adds single-file copies to an authorized destination, including another space.
The current 10,000-candidate engineering bound returns an explicit error when
exceeded, rather than incomplete results. These are implementation boundaries,
not claims about future Cloud Engine capacity.

Local browser validation additionally corrected a sort/layout save race, retained
keyboard focus after cancelling rename, and kept file names readable at 390px width.
Following the owner's reference screenshots, selection actions replace the existing
toolbar in its reserved space, without moving rows during double-click. Desktop
selection uses clicks, Shift and Cmd/Ctrl; touch retains row checkboxes. Selected
stars use a muted gold treatment.

Selection extension evidence, 17 October: the already-consulted canister-security
and service-discoverability skills support reusing authenticated domain methods.
The gap was batch orchestration and presentation, not a new storage/API service.
The portal snapshots the selection and session, uses one durable request ID per
item, reports each failure and retries only unfinished items. Move/Trash resolve
selected ancestry to avoid processing descendants twice; each backend mutation
still checks current rights and expected versions. Permissions in the toolbar are
affordances, not authorization. A session change stops subsequent calls. Proof:
partial retries, mixed rights, ancestry, account changes and actual local flows.

Bulk-download evidence, 17 October: browser QA confirmed that multiple generated
download clicks did not reliably deliver every selected file. The smallest extension
is one browser Blob containing a standard ZIP STORE archive (no compression or new
service). The [PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT)
was checked for the local/central headers, UTF-8 names and CRC32. One focused module
encodes this bounded format; Python's independent ZIP reader verifies interoperability.
Archives are limited to 256 MiB in this buffered browser implementation; larger
selections receive an actionable message. Folder recursion and streaming ZIP64 are
deferred. Per-file authorization failures are reported; only successful downloads
enter the archive, and session changes cancel publication.

The Home/My Space reference is implemented as two separate views. The initial
Recent-based Home was superseded on 18 October by derived Suggested Home, described
below; My files remains the authoritative folder tree. Suggested folder cards are
navigation links, not stored shortcut objects. There is no separate recommendation
service. The New menu reuses creation, upload and registered editor entry points.
Filter chips, sortable column headers, owner labels, and sidebar storage expose
existing metadata and operations. Private-ancestor names are not synthesized for
a Location column. Cross-canister search and AI recommendations remain deferred.
The original local environment is retained at port 61304; disposable browser
accounts use the separate QA network on port 51488. No mainnet destination or
confidential-hosting setting was changed.

## Confidential hosting and readable application data

Confirmed by the owner on 17 October 2026: plan for SEV-enabled hosting and no
client-side encryption for now. Contents and metadata remain readable to authorized
application code, supporting ordinary previews, metadata search and later content
indexing. Permission checks still apply to every result and action, including AI.

The current official node-infrastructure documentation describes AMD SEV-SNP:
encrypted GuestOS memory, attestation of the approved VM image, and sealed keys for
disk encryption. It describes a rollout rather than universal coverage. The HTTPS
outcalls documentation specifically recommends TEE-enabled subnets for protecting
credentials from node operators. This is confidential processing: the application
can read data inside the protected environment. It is not client-only end-to-end
encryption, and it retains trust in the hardware, runtime and upgrade governance.

Deployment acceptance must verify the chosen subnet's actual protection, supported
attestation evidence, storage/recovery path and any service receiving plaintext.
The current project's target subnet has not been verified in this review. Do not
infer confidential hosting merely from HTTPS, principal ACLs or ICP replication.
Official sources checked 17 October 2026:
[node infrastructure](https://docs.internetcomputer.org/concepts/node-infrastructure/),
[HTTPS outcalls](https://docs.internetcomputer.org/concepts/https-outcalls/).

Future Intelligence Gateway adoption must separately establish inference location
and data-processing guarantees. Protected application hosting does not automatically
protect plaintext sent to another service. Treat future extracted text, indexes and
conversation history as private application data with the same authorization rules.

### Client-side encryption remains a future option

ICP's EncryptedMaps/vetKeys is the supported starting point for a future proof,
including Rust support and plaintext metadata alongside encrypted values. Its
map-level Read/ReadWrite/Manage rights do not directly express this drive's inherited
Create/Edit/Delete rules or immutable chunked revisions. Evaluate that mapping and
the metadata control-plane variant before choosing an extension. No library failure
or platform limitation has been established, and no custom cryptography is proposed.
Future Cloud Engine deployment also needs the skill's vetKD proxy/key-identity
constraints checked before choosing durable key scope.

Client-side encryption would add key recovery, sharing, preview and content-search
design work. No browser performance benchmark or encryption integration has been
performed. Later encryption cannot undo earlier plaintext exposure or revoke copies
already received. Covering existing revisions would require explicit conversion.

Email was subsequently authorized with the owner's Resend account and subnet.ee
sender domain. See the [current implementation contract](#email-sharing--implementation-contract-18-oct-2026)
for Subnet-owned contact verification, principal-bound invitations and credential
custody. Internet Identity email attributes are optional platform functionality,
not a requirement of this account model.

Exact API boundaries, indexing, storage schemas and test structure are engineering
decisions. Product decisions such as reshare rights, anonymous links, retention and
format support should be explicit before the affected feature, not all prerequisites
for selection/navigation improvements. Mail delivery can be decided independently.

## Implementation order and proof

After the first milestone, implement minimal Suggested Home and ordinary
organization/sharing improvements on the current single-canister backend. Preserve
the documented identity, ownership and storage boundaries as those features change.
Multi-canister proofs, user-owned hosting, factories and data relocation are deferred
to their own workstream; they are not prerequisites for the next drive milestone.
AI/ML is also outside the MVP.

1. Selection, keyboard/context actions, compact list/grid, Recent/Starred, metadata
   search and correct sorting. Prove access-filtered results across pagination and
   selection/focus through bulk operations and locale changes.
2. Principal-based people/role sharing, copyable sharing IDs, restricted links and
   in-app access requests. Prove sign-in return paths, wrong accounts, revoked/moved
   access, role changes and private-ancestor isolation. Email invitations remain
   deferred; their separate future proof must cover signed attributes, address
   changes, expiry/replay, delivery retries and real II verification flows.
3. Folder transfer queue, duplicate decisions, copy and bulk download; New-in-folder,
   preview/Open with and supported format round trips across affected Craft editors.
4. Save status, bounded autosave checkpoints, activity/history and recovery.
   Embedded AI follows when Intelligence Gateway is adopted; it uses the same typed
   operations and authorization, with content extraction added only when needed.

## Architecture evidence

Session skill index retrieved 16 October 2026. Applicable cached skills reread for
this decision on 17 October; HTTPS outcalls retrieved 17 October. Official II web
documentation also checked 17 October. These are proposed extensions, not tested
integrations; no mail has been sent and no deployment changed.

- **Confidential hosting:** the canister-security skill covers application checks;
  current [node infrastructure](https://docs.internetcomputer.org/concepts/node-infrastructure/)
  and [outcalls](https://docs.internetcomputer.org/concepts/https-outcalls/) docs,
  checked 17 October, establish the SEV-SNP deployment approach. Requirement:
  process readable application data while protecting it from node operators.
  Gap: this project's selected subnet and full data path are unverified. Smallest
  extension: deployment selection and attestation/protection checks, not an app
  cryptography layer. Client-side encryption is deferred by owner decision. Gateway
  inference privacy remains a separate future proof.
- [Internet Identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md)
  and [Rust identity attributes](https://skills.internetcomputer.org/.well-known/skills/internet-identity/references/rust-identity-attributes.md):
  standard signed attributes and replay/origin checks. Gap: profiles/invitations in
  this app and proof in its browser adapter. Smallest extension: attribute intake,
  pending invitations and ordinary principal grants. Custom verification is deferred.
- [Stable memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md):
  the existing Rust store follows the supported approach. Gap: new records/indexes
  and their evolution. Extend the current service; no language replacement or
  separate database is justified. Proof: populated-state schema upgrade.
- [Canister security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md):
  caller authority and async boundaries. Extend the canonical permission evaluator
  and durable operations; test revocation/races and invalid input, not only success.
- [HTTPS outcalls](https://skills.internetcomputer.org/.well-known/skills/https-outcalls/SKILL.md):
  supported mail-API transport, with repeated-call semantics. Gap: service selection,
  credential custody and delivery assurance. Smallest extension: outbox plus one
  adapter after policy selection. Alternative: copyable invitations first.
- [Service discoverability](https://skills.internetcomputer.org/.well-known/skills/service-discoverability/SKILL.md):
  Candid and behavior documentation serve clients/agents. No custom MCP server.
  Future gateway integration is deferred; no inference/privacy claim is established.
- [EncryptedMaps](https://skills.internetcomputer.org/.well-known/skills/encrypted-maps/SKILL.md)
  and [metadata reference](https://skills.internetcomputer.org/.well-known/skills/encrypted-maps/references/metadata.md),
  retrieved 17 October 2026: supported client-side encrypted storage and metadata
  separation. Evaluate against inherited capabilities and chunked revisions before
  any custom extension. Deferred proof: actual browser/device latency, unauthorized
  key access, sharing/revocation, schema upgrades, recovery and Cloud Engine routing.

Component challenge: retain one backend and one evaluator; reuse II verification,
storage, transfer SDK and picker; defer federation, custom identity verification,
external search and AI-specific infrastructure. Email delivery is explicitly
deferred; no new external service is required for the current milestone.

### Suggested Home implementation contract (17 October 2026)

Authorized scope: deterministic Suggested Home, successful-open acknowledgements,
seven-day private dismissals, Viewer/Editor presets over existing grants, restricted
item links with sign-in return, and single-file copies through existing verified
download/staged upload methods. Folder copying and access-request inboxes remain
separate later workflows. No new canister, placement/routing layer or catalog.

The cached stable-memory, canister-security and service-discoverability skills above
were consulted before implementation. Standard support remains stable structures,
authenticated update calls, canonical permissions and retryable staged transfers.
Observed gaps: Home aliases Recent; open intent is recorded before decoding succeeds;
sharing exposes only capability checkboxes; item links and copies lack UI entry points.
Smallest extension: optional fields in the existing viewer record (opened revision,
dismissal expiry), a derived suggestions module, small facade methods and UI actions.
No recommendation service, behavioral event history or separate role authority.

Suggested files/folders prefer a newer revision of recently opened items, recent
successful opens, stars, then creation time. Stable IDs break ties. Recent remains
chronological. Return at most 20 files and six folders; keep the current Home order
through background revalidation while removing inaccessible items. Summaries retain
at most 200 opens and 200 active dismissals per viewer; intentional stars are separate.
Duplicate acknowledgements of the same revision coalesce within one minute. Existing
clients' legacy open-intent method remains compatible but cannot assert a seen revision.

Proof: caller isolation, revoked/expired/trashed targets, truthful reasons after rename
and content updates, dismissal expiry/bounds, successful versus failed opens, real
schema upgrade, shared read-only copying, duplicate/retry behavior, sharing role
changes and link sign-in/account changes. Actual native editor opens exercise the
shared acknowledgement integration. Future distributed placement remains deferred.

## Feature reassessment — 18 October 2026

Historical assessment before the transfer/email milestones. For the latest scope,
see the delivered milestone above and `cloud-drive-feature-review.html`.

This assessment is against the current local source and browser-tested build,
not the older mainnet release. It covers the major Google Drive product surfaces;
it does not claim every Workspace edition, admin policy or editor command has
been exhaustively compared. Google features differ by plan and platform. Capacity
and deployment migration remain outside the requested UX work.

### Available now

- Home, My files, Shared with me, Recent, Starred and Trash; breadcrumbs, compact
  list/grid, sorting and account-private preferences.
- Single-click selection, double-click/Enter opening, Shift ranges, Cmd/Ctrl
  toggles, keyboard focus, context menus and bulk move/share/star/Trash/restore.
  The selection toolbar occupies reserved space rather than moving the rows.
- Metadata search across authorized unloaded folders, including name, type, owner,
  modified date and location; sorting happens before pagination.
- Multi-file upload and external file drop, staged transfers, integrity-checked
  downloads, progress and retry; multi-file download as one ZIP.
- Folder creation, rename, same-space move, recoverable Trash, restore, owner-only
  permanent deletion; single-file Make a copy with explicit name and destination.
- Principal-based sharing IDs, Viewer/Editor/Advanced grants, versioned direct-role
  edits and revocation, inherited access display, stable restricted item links and
  sign-in returning to the requested item.
- Immutable content revisions, history download and restore as a new revision;
  explicit Save/Save As and stale-head conflict protection in Craft clients.
- Inert image/media/small-text previews and native Craft open/save integration.
  The eight rebuilt editors acknowledge successful opens after native decoding.

Suggested Home is now a derived view, not a renamed Recent list. It uses a newer
revision since a recent successful open, recent successful opens, stars and creation
time, with truthful reasons and private seven-day dismissal. It stores only bounded
viewer summaries. It does not use frequency counts, ML, calendar access or a second
catalog. Google's documentation describes recent opening/sharing/editing and
upcoming meetings as Home signals but does not disclose the complete scoring
formula. Our ranking is an explicit product choice, not Google's algorithm.
[Google Home](https://support.google.com/drive/answer/2424384?hl=en).

### Everyday workflow gaps: highest priority

1. **Folder transfers and collision handling.** Folder upload preserving nested
   paths; a visible per-file queue with cancel/retry and recovery; explicit Replace,
   Keep both and Skip. Current upload stops on a failed file; the SDK's resumability
   is not a complete restart-resumable queue in the portal. Replace already exists
   as an explicit revision upload but is not offered by a unified collision dialog.
   Folder ZIP download and multi-file/folder copy remain missing. Folder copy is an
   optional convenience, not an asserted requirement for matching Drive's web UI.
   [Google upload workflow](https://support.google.com/a/users/answer/9310458?hl=en).
2. **Organizing without repeated dialogs.** Cut/copy/paste across folders, internal
   drag-to-folder, undo for reversible operations, expandable folder navigation,
   shortcut objects and private folder colors. Selection and external drop already
   work. A folder card on Home is not a stored shortcut. A future shortcut must
   recheck target access and handle a moved, trashed or inaccessible target without
   revealing private metadata or acquiring its permissions.
   [Organization](https://support.google.com/drive/answer/2375091?co=GENIE.Platform%3DDesktop&hl=en),
   [shortcuts](https://support.google.com/drive/answer/9700156?hl=en).
3. **Predictable moves and recovery.** Preview effective access changes before a
   move; enforce the agreed owner authority for a move that changes the audience,
   including descendants. The current API checks subtree Edit/Delete/Create and
   reevaluates inheritance, but does not yet enforce that stronger policy. Also
   add alternate destination/name when restore's original parent or name is blocked.
   These are backend contract extensions, not merely dialog copy.
4. **Create here and open with.** Carry the current folder into New → Craft and
   first Save; offer explicit compatible Open with choices and an import/edit/export
   format matrix. App links in New already exist; ordinary Office files are not yet
   routed through verified Word/Grid/Deck import workflows. Cloud byte storage is
   not proof of Office-format editing fidelity.
5. **Richer previews and file information.** Thumbnails, next/previous preview,
   zoom, lightweight PDF/Office previews, a details panel, readable collaborator
   labels, descriptions and a storage breakdown. Current grid uses type icons;
   PDF opens its editor. Show only authorized paths and labels.

### Sharing and recovery: partly present, with important gaps

- **Request access and notifications:** an in-app request inbox, owner decisions,
  visible pending state and account switching from an inaccessible link. No email
  is needed for that first version. Missing/inaccessible objects must remain
  indistinguishable until there is authority to disclose them. Rate limiting and
  explicit invitation/request identifiers need a separate design.
- **People and effective access:** known-collaborator labels and picker, grouped
  per-recipient effective rights, editing custom grants and expiry in the UI.
  Backend expiry exists and role edits preserve it; there is no expiry picker.
  Multiple direct grants and inherited rights still combine, so lowering one grant
  cannot promise that all other access was removed.
- **Additional sharing policies:** anonymous/public links, groups, ownership
  transfer, reshare delegation and limited-access subfolders are not implemented.
  Viewer/Editor and owner-only sharing are intentional current defaults. A Commenter
  role needs actual comment operations; the reserved Propose bit is not that feature.
  Download/copy/print controls would need a distinct policy; current Read includes
  byte access and copying, and already received bytes cannot be retracted.
  [Google sharing controls](https://support.google.com/drive/answer/7166529?hl=en-193).
- **Activity:** file/space activity for uploads, edits, rename/move and grants,
  human-readable actor labels and optional in-app notifications. Existing content
  history is not an organization/sharing activity log. Avoid logging every read or
  using the activity log as catalog authority; scope and retention must be explicit.
  [Google activity and versions](https://support.google.com/drive/answer/2409045?hl=en-4).
- **Saving and recovery:** consistent cloud autosave and saved/offline/conflict
  status, account-scoped durable draft recovery, useful version checkpoints and
  Undo. Existing explicit saves and conflict rejection are the foundation. Decide
  checkpoint/retention behavior before adding autosave. Trash has no automatic
  expiry by deliberate choice; do not silently adopt Google's 30-day deletion.
  [Google Trash](https://support.google.com/drive/answer/15701190?hl=EN).
- **Unwanted sharing:** hide/remove a shared entry from the recipient's view,
  block/report abuse and a spam surface are missing; these become relevant before
  expanding invitation/public discovery features.

### Larger parity surfaces: separate milestones

- **Content search and OCR.** Search inside supported documents and image/PDF text,
  advanced operators, richer people/description/label filters and indexed snippets.
  Current search is metadata only. Add format extraction and a rebuildable index
  scoped by space/object/revision, with current ACL checks before results/snippets.
  This can be deterministic search without AI. Do not add vectors or a remote
  database until a measured requirement warrants them.
  [Google search](https://support.google.com/drive/answer/2375114?co=GENIE.Platform%3DDesktop&hl=en).
- **Document collaboration.** Comments, replies, mentions, assignments, suggestions,
  presence and concurrent live editing are not provided by the shared cloud drive.
  Individual editor menus or local history do not prove these integrated workflows.
  This is a separate editor/domain capability, not another file-list control.
  [Google collaboration](https://support.google.com/a/users/answer/9305685?hl=en).
- **Offline and device sync.** Offline file pinning/editing and reconciliation,
  Finder/Explorer synchronization, local-folder backup, mobile capture/scanning and
  native share-sheet workflows. Cached editor binaries are not offline file sync.
  [Drive for desktop](https://support.google.com/drive/answer/13401938?hl=en-Link),
  [offline files](https://support.google.com/drive/answer/2375012/?co=GENIE.Platform%3DDesktop&hl=en).
- **Team and enterprise administration.** Team-owned shared drives, groups and
  membership administration; organization sharing policies; labels/classification,
  approval workflows, administrative audit/retention and compliance tools. Our
  shared folder has one owning space/account and is not an enterprise shared drive.
  These need their own requirements; they are not prerequisites for a familiar
  personal drive. [Shared-drive roles](https://support.google.com/a/users/answer/12380484?hl=en),
  [labels](https://support.google.com/a/users/answer/13005112?hl=en&rd=1).
- **Integration ecosystem.** Generic Drive API/webhooks, third-party app registration
  and mail/calendar/task integrations are not provided by our typed Candid/MCP API.
  An authorized agent can use today's API; embedded conversational search, cited
  document answers and guided legacy-drive migration are separate future work.
  Intelligence Gateway remains the chosen future AI integration point.
- **Cross-cutting polish.** Broader screen-reader and signed-in phone testing,
  complete localized shared-editor status/recovery dialogs, keyboard shortcut help,
  preview accessibility and a browser regression matrix across editors. Do not
  turn eight successful native round trips into a claim of complete compatibility.

### Proposed next milestone: reliable file transfers and organization

Keep this bounded to work that removes daily friction and establishes reusable
operations for future migration:

1. Extract a cohesive transfer-job controller from the portal. Reuse the canonical
   staged upload/download SDK; represent each item with its destination, captured
   bytes or a reselectable source, operation ID, progress and explicit outcome.
   Never call browser reload recovery automatic if access to the local file was lost.
2. Folder upload plus Replace / Keep both / Skip; failures must not discard
   successful siblings. Cancel must not publish incomplete content. Distinguish
   cancelling a transfer from undoing files already committed.
3. Folder download to ZIP and multi-file copy using the same bounded traversal and
   job outcomes. Recursive folder copy can follow after those invariants are proven.
4. Cut/paste and internal drag-to-folder through the existing move operation, with
   audience-change preview and owner-authority enforcement first. All drag actions
   retain keyboard/menu alternatives. Undo must recheck authority and versions.
5. New-in-current-folder and explicit destination ownership in the shared picker.

Acceptance scenarios: nested paths and same-name collisions; permission revoked
mid-transfer; network interruption and uncertain commit; partial success/retry;
account switch; illegal descendant move; audience-changing shared-folder move;
restore with occupied/missing parent; keyboard-only flow; populated-state upgrade
for any new records. Folder operations must remain bounded and expose progress
instead of assuming one unbounded atomic recursive operation.

After that: effective sharing/access requests and details/activity; then preview,
formats and save/recovery; content search; finally collaboration/offline/enterprise
and AI as independently scoped products. This order is a recommendation, not new
authorization to implement all of the backlog.

No new canister, routing layer, duplicated catalog, speculative AI state or mail
service is needed for the next milestone. Per-user presentation remains private;
space content/ACLs remain canonical. Future sharding and user-owned placement stay
separate from identity and logical ownership. Email will use the owner's shared
project-wide delivery service when its contract is available, with invitation state
and principal-bound acceptance retained inside this application.

## Reliable transfers and organization — implementation contract (18 Oct 2026)

Status: implemented and locally tested. The seven transfer/organization groups
in the HTML review are this milestone; email is the newly authorized next stage.
Later groups remain deferred.

Requirements: preserve folder paths; individual transfer outcomes and retry/cancel;
explicit collision choices; hierarchical ZIPs and multiple-file copies; keyboard
clipboard and internal drag alternatives; reviewed access-changing moves; and a
Craft document's first save in the selected destination with visible ownership.

Skills consulted in this session on 18 Oct 2026:
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md),
[stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md),
[internet-identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md),
[service-discoverability](https://skills.internetcomputer.org/.well-known/skills/service-discoverability/SKILL.md).
The supported approach is ordinary authenticated canister updates, existing stable
structures, consensus metadata and digest-verified SDK transfers. There is no gap
requiring another service, canister, database, identity system or dependency.

Observed gaps and smallest extensions:
- The portal's serial upload loop loses remaining work on one failure. Extract an
  ephemeral job controller with stable per-attempt request IDs, source handles,
  captured destinations, outcomes and progress. Reuse existing upload staging and
  receipts; never create a second catalog. Retrying a failed job reuses its request
  ID, including after an uncertain commit. Explicit cancellation aborts an
  uncommitted stage after draining in-flight chunks; a completed commit stays
  completed. Reload recovery requires reselecting local sources; do not imply
  browser file permissions or bytes persist automatically.
- Folder selection uses browser `webkitdirectory` / `webkitRelativePath`; drops
  use entry traversal with repeated `readEntries` until exhausted. Validate paths,
  bound depth/items/bytes, and expose unsupported-browser fallback. A selected
  folder's existing counterpart is explicitly merged; file collisions offer
  Replace / Keep both / Skip. Conflicts are still decided atomically by the server.
- ZIP generation already has CRC32 and an independent reader test. Extend that
  encoder with validated relative paths and empty directories, retaining its
  bounded buffered model. No recursive-copy protocol or archive dependency.
- Existing move checks allow an Editor to change inherited access. Add one shared
  move planner, used by preview and mutation. Compare each affected object's
  effective capability expiry envelopes (including additive direct grants and
  agent/sponsor intersections), so changed durations cannot bypass owner authority.
  Reviewed moves bind to the existing catalog version; mutation rechecks all
  authority and versions. Ordinary legacy moves enforce the same owner boundary.
  Preview details are owner-only when access changes. No ACL snapshots are stored.
- Shared launch parsing gains a typed new-document destination. The common picker
  validates space/folder/create rights and shows ownership. Editors retain native
  formats and saving behavior; URL IDs convey no authority.

Alternatives deferred: persistent browser copies of user file contents, parallel
unbounded uploads, recursive folder copy, background synchronization, new routing
or per-user canisters, and a durable activity log. None is necessary for these
flows. Folder creation and file commits are individually atomic; a batch may have
partial success and never pretends to be a transaction.

Proof: SDK cancellation/uncertain-commit tests; backend owner/Editor and expiry
move tests; hierarchy/collision/queue/session tests; independent ZIP extraction;
portal browser flows; all eight editor save/reopen checks after canonical SDK/UI
adoption. No stable memory IDs or stored records change in this milestone. Local
validation does not establish SEV or production deployment properties.

Browser clipboard integration uses standard [copy](https://developer.mozilla.org/en-US/docs/Web/API/Element/copy_event) and [paste](https://developer.mozilla.org/en-US/docs/Web/API/Element/paste_event) events (checked 18 Oct 2026), including macOS Edit-menu commands. Clipboard text contains restricted links, never authority or file bytes. Paste requires matching text and the same account's live in-memory selection; arbitrary pasted links cannot become move commands.

Validation: 37 backend tests, 17 SDK tests, 10 shared UI tests, focused portal/queue/
archive/locale tests, release builds and native save/reopen in all eight adopted
editors passed. The populated pre-suggestions-schema PocketIC upgrade also passed;
this stage adds no stable record types. Browser proof includes actual folder upload,
Cmd+C/V copy, access-loss preview and recipient denial after moving a shared file,
and FigCraft's first save into the launched folder. Folder cancellation and failed
parent dependencies have separate regression tests: children cannot recreate a
cancelled folder or silently retry a failed parent, while unrelated siblings finish.

Limits: browser folder selection omits empty directories; directory-drop traversal
retains them. The upload/copy queue is ephemeral and requires source reselection
after reload. A cancelled stage does not undo committed objects. Hierarchical ZIP
download uses the existing bounded buffered download flow, not the upload queue.
Recursive folder copy, automatic durable transfer recovery, background sync and
a complete cross-browser/native-editor regression matrix remain future work.

## Email sharing — implementation contract (18 Oct 2026)

**Requirement.** Familiar sharing by email, with optional, multiple Subnet contact
addresses per authenticated principal. Each address has one current binding.
Mailbox proof plus an explicit confirmation may transfer the binding; it must not
merge accounts, move files, transfer existing ACL grants or inherit old invitations.
Internet Identity remains the login mechanism. Email is not a login/recovery key.

### Supported integration and the smallest extension

Consulted on 18 October 2026:
[Internet Identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md),
[identity attributes](https://skills.internetcomputer.org/.well-known/skills/internet-identity/references/rust-identity-attributes.md),
[HTTPS outcalls](https://skills.internetcomputer.org/.well-known/skills/https-outcalls/SKILL.md),
[stable memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md),
[canister security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md),
and [Cloud Engine](https://skills.internetcomputer.org/.well-known/skills/cloud-engine-canisters/SKILL.md).

II offers signed `verified_email` attributes. The released auth SDK 10.0.2 supports
attributes, and DFINITY's Rust verifier exists in the official identity-attributes
repository (inspected at `24da86edfd05bee13874e3f5a5da92f5b7978ee2`), even though
`cargo info identity-attributes` did not find a published crate. Neither package
observation establishes a platform limitation. The product gap is that Subnet
contacts must not require linking addresses to II. The smallest extension is a
caller-bound, expiring mailbox code, using management-canister randomness and
ordinary authenticated updates. The shared Rust login/session remains unchanged.

Resend's [send API](https://resend.com/docs/api-reference/emails/send-email) receives
fixed plain-text messages via the standard `ic-cdk-management-canister` 0.2 HTTP
builder. Non-replicated results only describe delivery; they never establish mailbox
proof or grant access. Immutable request bodies and stable idempotency keys support
manual retries within Resend's [24-hour window](https://resend.com/docs/dashboard/emails/idempotency-keys).
No background worker, extra canister, HTTP relay or alternate authentication service.

The September local launcher rejected a v2-priced call before emitting an HTTP
request. On 18 October, upgrading to the official launcher
`v16.0.0-2026-10-08-03-33` resolved this with its ordinary defaults. Real Wasm tests
now use no beta flag or custom launcher. The app retains the standard management
HTTP wrapper, with no cycle padding, pricing fallback or relay. The user explicitly
deferred SEV hosting as a prerequisite: email works now; future hosting protection
will be verified separately. See history for the local restart/test-data incident.

### Account, invitation and delivery boundaries

- Contacts: up to ten independently verified addresses per principal; no primary
  address. Conservative ASCII normalization lowercases the address, preserving dots
  and plus tags. Eight-digit codes expire after ten minutes and five attempts, are
  caller/request-bound and single-use. Randomness is requested after reserving the
  challenge and send quota. Account switching invalidates pending UI responses.
- Transfer: only a proved current challenge can request explicit transfer. A
  monotonically increasing address generation detects intervening changes, including
  removal/rebinding. The old principal loses only that contact. Other contacts,
  files and grants stay put. Old pending invitations cannot follow a transferred
  address. This treats proof as current mailbox control, not permanent identity.
- Sharing: the owner needs a verified contact to send. A bound recipient receives
  an ordinary principal grant and a restricted file/folder URL. Re-sharing updates
  the direct role instead of adding duplicate grants. An unbound recipient gets a
  revocable seven-day invitation. The link alone grants nothing and reveals no file
  details. Acceptance requires the invited current verified address, explicit action,
  a live target and the inviter's current authority. Accepted retries cannot recreate
  revoked grants. A failed notification never rolls back a successful grant.
- Delivery: stable outbox records distinguish queued, sending, provider-accepted,
  retryable, failed, unknown and expired. Provider acceptance is not inbox delivery.
  No automatic retries: at most five attempts, a two-minute lease, immutable payload
  and key, and a maximum 24-hour retry window. Verification delivery expires with
  its ten-minute code. Accepted/failed payloads are erased; bounded maintenance
  removes expired payloads and eight-day-old receipts without touching ACLs.
- Abuse bounds: 30 sends/account/day, 10/address/day, 1000/canister/day; verification
  cooldown 60 seconds. Bounded maps and cleanup complement caller authentication.
  There is no public email directory or arbitrary lookup API. An authorized owner
  can infer whether a real share granted immediate access or created an invitation;
  absolute account-existence secrecy is not claimed.

The one data canister retains canonical objects and ACLs. Private contacts,
versioned address routing, invitations and outbox use new stable memory IDs 26–35;
existing IDs 0–25 are unchanged. Email addresses never become object ownership keys.
Future multi-canister placement must preserve contact uniqueness and routing policy;
no sharding or user-owned-canister mechanism is implemented here.

Sender: `subnet.ee <info@subnet.ee>`. The credential lives in the **backend canister's
environment variables**. This is a canister-held runtime secret, not an application
account field. The owner's signed management `update_settings` call installs,
rotates or removes it; lazy reads mean no rebuild or upgrade is required. No custom
admin endpoint, duplicated admin allowlist, public secret getter or frontend key.
Controller-only status, snapshot and log visibility is required. Under the current
single-owner policy, provisioning refuses any controller list other than the
signing private identity alone. Future backup/governance controllers need an explicit
custody decision; controller access includes the ability to replace the code.

This follows the [ICP CLI skill](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md)
and its [runtime environment-variable reference](https://skills.internetcomputer.org/.well-known/skills/icp-cli/references/canister-env-vars.md)
(checked 18 October 2026). The supported management API already satisfies owner
authorization. The small tooling gap is that CLI `settings update` takes secret
values in command arguments. `scripts/configure-mail.py` therefore uses the CLI's
file-based settings sync with a mode-0600 file inside a mode-0700 **system temporary
directory outside the project**, removed on normal completion or exception. Sync
merges variables; explicit `--remove-environment-variable` commands remove them.
Each mutation verifies the resulting settings without printing values. Code-only
upgrades use `icp build` and `icp canister install --mode upgrade`; the broader
`icp deploy` was observed replacing environment settings and removing the key.
The tested install-only workflow preserves it. Re-provision after deliberately
re-applying deployment settings. Remote
targets require an explicit canister ID and the pinned mainnet root key. No remote
deployment or credential provisioning has been performed in this milestone.

The local provisioning copy is in macOS Keychain (`ee.subnet.cloud.resend`, account
`info@subnet.ee`). The original `resend.rtf` was deleted. The local controller key
also uses ICP CLI's standard Keychain identity; its old plaintext PEM was removed.
No key enters source, frontend assets, command arguments or application logs. The
isolated real-send replica and its historical checkpoints were removed after the
test. Local replicas persist their canister state on disk: configuring a real key
there necessarily puts it into that replica state. With the user's instruction to
proceed on current hosting, the main local backend is configured through the
owner-only helper and sending is enabled. No standalone key file is kept in the project.

The owner-only management boundary is proven with a real PocketIC canister: users
and anonymous callers cannot read or change settings, owner rotation/removal works,
and the key survives upgrades. Controller-only access does not hide memory from
ordinary subnet operators; future SEV hosting remains a separate deployment choice.
Keychain protects the local provisioning copy, not remote execution or arbitrary
processes already authorized under the same OS user. Resend bounce/webhook handling
is not implemented.

**Proven locally:** model tests for multi-contact uniqueness, transfer, stale proof,
wrong/replayed/expired codes, wrong recipients, rebind isolation, revoked/expired
invitations, trashed targets, role replacement, revoked-grant replay, retry identity,
cleanup and stable reopening. A real populated-Wasm upgrade test executes management
randomness and mocked HTTP requests, verifies mailbox binding, upgrades again and
checks existing file bytes and authorization. Portal tests cover ET/EN, multi-contact
verification, explicit transfer, invitation routing and account-switch races.
Browser and authorized real-provider outcomes are recorded in history when run.

**Deferred:** delivery/bounce webhooks, aliases/IDN policy beyond the documented
normalization, notification preferences and a cross-project email service. Future
SEV hosting must be verified before claiming node-operator confidentiality.
Controller-only secret access is already required by the current provisioner.

### Pagination bookmarks — implementation evidence (18 October 2026)

Requirement: a bounded search may stop on a filtered or inaccessible record. Its
continuation must not reveal that record's UUID or metadata, nor permit a caller
to probe private sort positions by supplying arbitrary IDs. The standard stable
B-tree seek remains the retrieval mechanism (stable-memory and canister-security
skills retrieved this session); a plain JSON bookmark does not close this gap.

Use one service-local authenticated-encryption boundary for opaque bookmarks,
with RustCrypto's standard nonce-free AES-SIV (RFC 5297), rather than a cursor
registry or a custom cipher. The key is derived from ICP raw_rand, kept in stable
memory, and never returned. Actor, query, versions and expiry remain inside the
sealed bookmark; every page independently authorizes its records. No file-content
encryption or user key management is introduced. Deterministic SIV is suitable
for replayable bookmarks and does not depend on mutable query RNG state.
Primary sources checked: https://docs.rs/aes-siv/latest/aes_siv/ and RFC 5297.
The crate is not independently audited; tests must cover tampering, cross-user
replay, private-record filtering, malformed/oversized tokens and upgrade replay.


### PDF preview — shared display controls (18 October 2026)

Requirement: view PDFs without leaving Cloud Space, including browsers without a
working embedded PDF plug-in. The native iframe cannot provide application-owned
page, zoom, accessible text or render-completion controls. The initial blank preview
fixture contained empty form fields and is NOT evidence of a browser incompatibility;
a separate two-page text fixture is used to validate rendering. Existing authenticated
SDK downloads, current ACL checks and static-site hosting remain unchanged.

Use Mozilla PDF.js's display layer (Apache-2.0), pinned to 6.4.299, served with its
worker, CMaps, fonts and codecs from our own frontend. Primary guidance checked
2026-10-18: https://mozilla.github.io/pdf.js/getting_started/ and
https://github.com/mozilla/pdf.js/releases/tag/v6.4.299 . ICP static-site skill applies
to hosting; no new canister, service, identity or content index. Smallest extension:
a disposable, lazy preview controller, one page/canvas at a time, bounded resolution,
explicit page/zoom controls and cancellation on close/account change. Render text
inertly for accessibility; do not execute PDF actions, embedded scripts or links.
A native iframe remains a smaller alternative, but would leave controls and
render-completion behavior to the browser. PdfCraft stays the editing path. Proof: real PDF in browser, page/zoom,
close/session race tests. Complex PDF/font compatibility remains format-dependent.
