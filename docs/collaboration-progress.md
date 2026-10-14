# Collaboration implementation evidence

Published branch: `main` (implementation originated on `feat/shared-cloud-drive`).
The root drive and seven Craft shared-folder integrations are on GitHub and mainnet.
The current demo MVP's
scope is closed; the broader original product vision remains future work.
The current reconciliation below supersedes historical status
statements in the chronological implementation notes; those notes retain the
evidence and limitations of each earlier step. No independent security audit is claimed.

## Complete GitHub and mainnet publication — 15 October 2026

The user authorized publishing all accumulated changes, including the previously
separate illustrative homepage agent workflow. Source release:
[`210bcbf`](https://github.com/eerikpotter/subnet.ee/commit/210bcbf8f1c474917db5ecff63a8f186c66e5d5a).
All seven Craft fork commits were pushed to their `origin/main` before the parent
submodule pointers; all eight remote branches matched their clean local checkouts.
The assembled app manifest records those exact commits with `sourceDirty: false`.
Editor JS/Wasm bytes match the earlier native-browser-tested seven-app release.

The full site is published at <https://www.subnet.ee/>: 172 assets, including the
new homepage demonstration and the simplified drive without upload-administration UI.
Frontend state hash:
`06871feaac2acb1839a6451a8b38da6ef361267e295d3300ca8ba7e601499000`.
The live backend module already matches the tested local artifact:
`34a26aad2ef9a21edfc0fcf00f9a61d6bdb67c5ad0bc611e4d63e751404b27eb`.
No backend upgrade, storage migration or user-file mutation was needed for this publication.

Validation: `python3 scripts/verify.py all --environment ic --origin https://www.subnet.ee`
passes for the portal and all seven editors, including exact downloaded payloads,
certification headers, runtime identity settings, MCP discovery, CSP, cache policy
and missing-asset handling. The finalized asset state was separately read back.
Local ET/EN homepage checks exercised the workflow, image/text previews and return
to the file list; the mainnet demonstration and signed-out drive startup also pass
with no observed browser console errors. All 50 JavaScript and 19 Python tooling
checks, strict cloud UI/backend Wasm Clippy and canonical/vendor snapshot checks pass.

The six editor-wide CI attempts still stop on previously documented Clippy findings
in unchanged upstream code; their formatting stages pass. PhotoCraft's layer check
passes. These are recorded limitations, not a claim that editor-wide CI is green.
The MVP scope and deferred functionality below remain unchanged.

## MVP scope closure — 15 October 2026

Subsequent UX clarification: unfinished upload chunks are a background storage
concern, not a user task. Removed the pending-upload management button/dialog from
the ordinary drive; keep the existing technical abort APIs and defer automatic
cleanup. Do not equate upload expiry with physical byte reclamation. This is a
portal-only simplification, not a new backend lifecycle implementation. The existing
ICP static-site recipe and localized portal assets cover this change; no dependency,
permission model, stable schema or editor rebuild is needed.

Validation: 27 drive/localization JavaScript checks and 3 prerender/localization
Python checks pass; local layout and mainnet ET/EN browser checks confirm the control
is absent. The isolated `.local/upload-ui-release` retains the preceding production
bundle, changing only the drive's two localized pages, JS, CSS, translations and
the translation digest in `apps.json`. Mainnet portal exact-payload/certification/
identity/discovery checks pass; the final manifest was separately rechecked after
its digest update. Frontend state:
`5e2eda9ec09719de9d405ca1fd5c852ea2fef9d6d6b99a83d1a39bf673073b68`.
There was no backend deployment or automatic cleanup implementation.

After the review, the user explicitly deferred proposals and search, said automatic
cleanup is not critical for a demo, chose to retain the existing Trash, and asked to
draw a line under the MVP. The active specification now reflects that decision.
No new feature implementation or deployment was undertaken during this review.

The delivered MVP is the personal/shared drive, basic file/folder operations,
account permissions, content history and recoverable Trash, all seven Craft apps'
shared cloud Open/Save/Save As, and standard ICP MCP. General copy, proposals,
search, automatic cleanup, expanded activity history and richer cloud UI are later
work, not blockers added to this demo's scope. Keep the earlier design as a backlog.

Known limits remain explicit: the sharing form adds grants (narrowing access needs
removing the old applicable grant); Trash has restore but no permanent-delete button;
quota includes Trash and versions; some Craft status dialogs remain English; and
AI has the authenticated account's rights, not separate scoped agent rights.
The test evidence and its boundaries below still apply. Scope closure does not
turn deferred features or unrun browser scenarios into passing functionality.

## Current specification reconciliation — 15 October 2026

Reviewed the [active alpha contract](collaboration-spec.md), the preserved
[original scoped-agent design](collaboration-future-agent-design.md), the current
API, backend, portal, canonical SDK/UI and executable tests. This review does not
silently remove unfinished requirements from either specification.

Architecture evidence: the session's ICP skill index and relevant
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
guidance were consulted. Ordinary signed Candid methods, the existing stable store
and its indexes, and the canonical SDK/picker remain the supported approach.
The gaps below are product workflows, lifecycle rules and regression coverage;
they do not establish a need for another permission service, MCP server or canister.

### Implemented, with evidence

- **Core drive:** personal/shared/Trash views, folder navigation, bounded metadata
  prefetch, file/folder creation, chunked upload with progress, download, supported
  file previews, rename, same-space file/folder move, Trash and restore. IDs survive
  organization changes; content and metadata are stored separately. Owner, parent
  and recipient indexes avoid scanning every user's files on normal browse paths.
- **Human sharing:** direct and inherited Read/Create/Edit/Delete, owner-only grant
  administration, live union/expiry/revocation checks, private-ancestor isolation,
  subtree checks and Create without Edit/Delete. The backend can update a grant
  with its ID and version; the portal's permission-editing UX is incomplete below.
  Source: `services/sovereign-cloud/src/permissions.rs`, `src/store/collaboration.rs`
  and `src/store/read_index.rs`; public contract: [API](../services/sovereign-cloud/api.md).
- **Content safety:** fixed-revision downloads with hash/length validation, staged
  quota reservation, commit-time permissions/head checks, idempotent uncertain
  commits, history and restoration as a new revision. Revisions record the acting
  account. This is content history, not the full activity/audit model below.
- **Craft:** all seven apps share the portal picker and authenticated drive SDK.
  Shared Open/Save/Save As and opening retained versions exist. Local native browser
  roundtrips were performed for all seven; Word additionally exercised shared Edit,
  Read+Create downgrade and draft-preserving Save As. Grid's quota failure preserved
  its draft. The exact production assets were verified. Two-account content writes
  were local fixtures, not production user files.
- **MCP:** the existing ICP connector invokes this same backend. The recorded real
  JSON-RPC transport test includes Bob's allowed create/read, denied Edit/Delete and
  private-sibling access, and revocation. Real II issuance is covered by the separate
  signed-HTTP experiment. The user also confirmed ChatGPT folder/file creation on
  mainnet. These are complementary proofs, not an automated hosted-provider suite.

### Remaining product backlog identified by the review

The following work was identified before the scope-closure decision above. It is
retained for later prioritization, not as a new set of current-demo release gates.

- [ ] **Make permission changes unambiguous.** `portal/cloud.js::share` currently
  creates a new grant even for an existing recipient; rights are a union, so adding
  Read does not remove an older Edit grant. Show effective capabilities and their
  direct/inherited sources, edit/revoke the existing grant by ID/version, and show
  expiry. Explain when an inherited grant still provides access. Acceptance: Edit
  → Read blocks the next save; an overlapping inherited Edit remains explicitly
  visible; stale grant edits and revocation during upload fail safely. Do not add
  deny rules or change the backend's inheritance model to fix the form.
- [ ] **Complete storage and recovery lifecycle.** Expired staging is rejected
  after 24 hours but needs manual abort to release quota. The specified bounded
  cleanup and 30-day Trash retention/deadline are absent. The UI cannot reclaim
  retained file/history capacity, although Trash still counts toward the 20-file
  quota and retained versions toward the 20-revision limit. Add an explicit,
  owner-authorized recovery/cleanup flow, accurate quota guidance and safe bounded
  reclamation. Resolve restore name/parent conflicts in the UI. Include lifecycle
  of mutation receipts and inactive grants: their bounded stores currently fill
  without reclamation. Test quota release, interrupted cleanup, retry semantics,
  retained references and upgrade safety. Existing irreversible owner-only legacy
  deletion endpoints are not a complete Trash/purge experience and are available
  to the owner's AI too; no human-only safeguard is implied.
- [ ] **Finish shared-location clarity.** Before Craft Save As, show the selected
  folder's owner as well as its location. Before moving shared content, explain
  changes to inherited access without leaking inaccessible recipients/ancestors.
  The current picker displays location, but not owner; the move dialog has no
  access-impact preview. Translate the shared Craft status/retry/unsaved-work
  dialogs, which still contain hard-coded English despite the localized picker.
- [ ] **Complete reusable browser regression coverage.** Convert the recorded
  two-account journeys into a repeatable suite/release gate. Cover permission
  editing, inherited overlap, upload-time revocation, history restoration, name
  conflicts and folder organization. In every Craft adapter verify conflict,
  revoked access, rename/move/Trash while open, account switch, history and preserved
  unsaved work, not only native save/open. Check keyboard/focus/Escape, both locales
  and narrow layouts. Existing DOM tests and manual screenshots do not constitute
  that full suite. Add store-saturation/worst-case instruction tests beyond the
  existing 64 MiB single-document commit check. No root automated CI workflow was
  found to run the full cloud acceptance gate.
- [ ] **Add search and general copy.** No search/copy drive API or portal workflow
  exists. Craft Save As only covers new app-document copies. Search must stay inside
  live authorized scope; copying must use new IDs/destination ownership, without
  copying private history or grants. File/folder behavior and resource bounds need
  explicit acceptance cases.
- [ ] **Future proposal workflow — explicitly deferred.** Propose is a
  reserved capability, not an executable workflow. Submission, approval, rejection,
  withdrawal, candidate preview and stale-base handling are absent in the backend,
  portal and Craft UI. The earlier agent identity deferral did not itself defer
  human proposals; the user's subsequent 15 October scope decision now explicitly
  does. The active specification has been updated accordingly.
- [ ] **Complete activity and provenance details.** Content revisions have account,
  time and bytes, and can be restored. There is no separate rename/move/Trash/restore
  activity log, and history does not expose the complete promised operation/base/
  proposal lineage. Add the minimum useful account-attributed activity record using
  the same mutations; do not infer a human/agent operator from a client label.

### Explicitly deferred by the accepted alpha decision

Separate, initially empty agent scopes; agent-specific permission ceilings and
disconnect; trustworthy human-versus-agent attribution; native semantic Craft
agent tooling; user-owned storage canisters; standalone desktop integration.
An MCP connection currently has its user's authority, including that user's shared
access and owner administration. It does not acquire canister controller rights.
The existing future-agent evaluator unit tests are not proof that the live connector
has a restricted agent identity. No new identity restriction is claimed by this review.

### Validation rerun for this review

All **126 existing checks** run here passed: 24 service library tests; 6 SDK unit
and 10 transfer tests; 10 shared UI tests; 50 JavaScript locale/drive/shell/loading/
Wasm tests; 19 Python workflow/localization/discovery tests; and 7 actual-canister
PocketIC persistence/migration tests. The Python and shell checks include supporting
tooling and are not all permission tests. Passing counts do not close the unchecked
acceptance items above.

The service was rebuilt with `RUSTUP_TOOLCHAIN=1.95.0 icp build cloud`; SHA-256 is
`34a26aad2ef9a21edfc0fcf00f9a61d6bdb67c5ad0bc611e4d63e751404b27eb`, matching the
recorded production backend. The seven PocketIC tests used that Wasm,
`.local/baseline-cloud.wasm`, `.local/drive-pre-index.wasm` and PocketIC 16. They
cover actual upgrades, preservation of bytes/ACLs/unfinished uploads, stale writes,
seven-format permission checks, legacy endpoint denial and the 64 MiB commit budget.
Log: `.local/cloud-spec-review-integration.log`. The sandbox denied the first local
listening socket; that run was stopped and the isolated suite rerun successfully
with local-port permission. No production data was used or changed.

MCP/II transport results and seven native browser journeys are the recorded earlier
evidence below; they were not rerun in this specification review. The browser gaps
above remain gaps. The review changed documentation only and did not deploy.

## Active decision, 14 October 2026

The user explicitly approved a smaller alpha: use standard ICP MCP with the user's
ordinary application authority. Agents can perform the same actions as that user,
including administration and deletion. History records the authenticated account;
it cannot reliably identify whether a person or AI operated that credential.

The active contract is [collaboration-spec.md](collaboration-spec.md). Restricted
agent connections are deferred in
[collaboration-future-agent-design.md](collaboration-future-agent-design.md).
The provisional custom MCP/OAuth service and its dependencies have been removed
from the build. No replacement MCP transport or secret custody is needed for this
alpha. Unexported prototype files are retained only under ignored `.local/`.

## Identity boundary: executable evidence

[Reproduction and limits](../experiments/agent-provenance/README.md): II backend
`release-2026-10-13`, PocketIC 16.0.0, ic-agent 0.49.2 and ic-cdk 0.20.3. Disposable
identities obtain real II delegations and make signed HTTP calls. Four checks pass:

- Normal app and full MCP delegations present the same caller, no caller-info
  signer and empty caller-info data. Both can invoke the user's edit endpoint.
- Query-only delegations allow queries and the replica rejects updates.
- A distinct principal registered in the probe can have narrower application
  permissions, accountable attribution and immediate application revocation.
- Disabling MCP in II stops new issuance; an issued delegation still works before
  expiry.

This proves the tested identity boundary, not a hosted MCP client round-trip,
interactive II consent or Claude/OpenAI compatibility. It does not prove that no
other supported ICP mechanism can provide restricted connections.

## Upstream questions: DFINITY only

Published and verified in the authenticated browser on 2026-10-14:

- [dfinity/imcp2 #208](https://github.com/dfinity/imcp2/issues/208): restricted,
  application-verifiable agent identities.
- [DFINITY forum discussion](https://forum.dfinity.org/t/scoped-agent-access-and-attribution-through-icp-mcp/76096):
  recommended ICP identity integration and revocation boundary.

Nothing was posted to ByteSmithLabs. The third-party dependency incompatibility
is an [internal finding](icp-dependency-investigation.md), not an imcp2 defect.

## Implementation and validation

- Seven existing service unit tests passed with Rust 1.95.0 before changes.
- Seventeen service library tests pass, including sharing, creation without Edit/
  Delete, direct-file privacy, grant/create retries, NFC collisions, pagination,
  atomic moves, retained Trash state, legacy-path rejection and quota accounting.
- Two existing PocketIC SDK/migration tests pass against the retained baseline and
  new Wasm, including a lost commit response. These are storage/API checks, not
  browser checks or complete coverage of all seven Craft applications.
- The service builds to release Wasm and the canonical Candid contract matches its
  exports. The final Clippy pass succeeds with warnings denied; `git diff --check` is clean.
- Legacy stable memory IDs 0–8 and existing Candid shapes remain reserved and must
  survive upgrade. Baseline Wasm is retained at `.local/collaboration-baseline.wasm`.
- Ordinary signed Candid endpoints now expose drive initialization, paginated
  listings, current-file reads, folder and inline-file creation, owner-managed
  sharing, rename/move/Trash/restore and API documentation. `api.md` explicitly
  describes unfinished functionality; no custom MCP server/dependency is included.
- The optional `CLOUD_WASM` identity runner passes against the actual cloud canister:
  Alice shares with Bob; Bob's browser and II-issued MCP credentials create/read
  files; Edit/Delete and out-of-scope access are denied; grants/files survive an
  upgrade; revocation denies both credentials and retries. The runner still uses
  direct Candid calls. The separate MCP transport proof below now also passes.
- The portal now has a compact, expandable “Work with AI” setup panel in My files,
  with the official connector address, three steps and the account-authority alpha
  notice. Estonian/English static pages and live switching were checked in a desktop
  browser; switching preserves the expanded panel. Nine JavaScript locale/state
  checks and three Python localization checks pass. This was a static preview,
  not an interactive II connection test.
- Shared-folder portal/Craft UI integration is not implemented yet. The complete spec remains
  unfinished: shared replacement uploads, proposals, recovery listings, activity,
  bounded cleanup, fixed-revision reads and worst-case instruction checks remain.
- Remaining work includes complete file/folder management, transfers, sharing,
  history/proposals, shared UI, hosted MCP/provider authentication, further migration
  checks and browser verification for all seven Craft adapters.

## MCP integration proof, 14 October 2026

[Reproduction and limits](../experiments/mcp-smoke/README.md). DFINITY imcp2-core
0.5.1 at `355d8deb181c5cf0486de5c3ad8be93c5f2dfa76`, unchanged upstream Cargo.lock,
rmcp 2.2.0, ic-agent 0.49.0, PocketIC 15.0.0. Negotiated MCP version: **2025-11-25**.

- `icp build cloud` succeeds using the official Rust recipe. Its public
  `candid:service` matches the canonical contract (apart from CLI output newline).
- Eleven discovery/workflow tests pass, covering invalid/missing IDs, changing
  environments, removing stale identity metadata, and assembly/presync separation.
- The disposable `site-check.py` deployment passes actual presync ID injection,
  cloud-only manifest, certified JSON/CORS/cache headers, local identity fallback
  and missing-path 404. Its temporary network was stopped afterward.
- The MCP smoke test passes discovery, tool listing, actual service Candid and API
  docs, undeclared-canister refusal, signed file creation/read, owner visibility,
  exact bytes, account attribution, retry idempotency, denied Edit/Delete/outside
  access and sharing revocation. Alice owns and shares; Bob acts through MCP.
- The test exposed the expected gateway-alias normalization: both II's browser flow
  and imcp2 derive canister identities using the legacy `ic0.app` spelling. The
  manifest's pointer remains the existing `icp.net` origin. Primary II/imcp2 source
  confirms the normalization; the test checks the resulting provenance.

The MCP test uses a real JSON-RPC byte stream and signed HTTP ingress to the actual
cloud Wasm. Its public web retrieval and delegation issuance use upstream test-only
fixtures. Real II-issued delegation checks are separate, above. No hosted OAuth,
Streamable HTTP, production or Claude/ChatGPT end-to-end result is claimed.

The separate architecture review retains only deployment metadata, public API docs,
existing permission checks and a small setup disclosure in the production path.
The upstream test harness is development-only. Custom MCP hosting, OAuth, agent
secrets and TEE infrastructure remain unnecessary for the accepted alpha scope.

## Required integration gate

Official skill index retrieved on 2026-10-14:
https://skills.internetcomputer.org/.well-known/skills/index.json.
Relevant skills: service-discoverability, agent-web-identity, internet-identity,
canister-security and stable-memory. The supported integration is ordinary Candid
methods, public `candid:service`, `get_api_doc`, correct II derivation origin and an
operator-approved discovery manifest.

Exact skill sources retrieved for this work:
[service-discoverability](https://skills.internetcomputer.org/.well-known/skills/service-discoverability/SKILL.md),
[agent-web-identity](https://skills.internetcomputer.org/.well-known/skills/agent-web-identity/SKILL.md),
[internet-identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md),
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md),
[stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md),
[static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md),
[icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md).

Operator opt-in was approved on 2026-10-14. The preceding request identified
operator authority, acceptance of the ICP MCP App Operator Terms v1.1 (effective
2026-09-02, checked on 2026-10-14), and authorization to add the manifest. The user
replied: “Yep. I authorize to continue and add the MCP capability to subnet.ee”.
This affirmative response covers that request; production deployment remains a
separate action and has not occurred.

The static-site and icp-cli skills were also retrieved on 2026-10-14. The frontend
presync hook now generates the cloud-only manifest using deployed canister IDs.
On mainnet it publishes the existing frontend-canister II derivation origin; local
development retains its visible-origin identity. Ordinary assembly creates neither
discovery file. No application MCP server or production dependency is introduced.

Workspace/repository AGENTS.md and the repository's always-applied Cursor rule now
require skills before architecture choices, a supported-path proof before custom
infrastructure, and explicit evidence for any platform limitation.

## 2026-10-14 — Root drive UX correction and local browser verification

The original specification included a full drive. The earlier portal delivered only
a flat Craft file catalog; that was an implementation gap, not an agreed scope cut.
The old 17 service tests, 2 migration tests and signed/MCP proofs did not establish
browser folder/upload/organization functionality.

Architecture evidence: re-read the current
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
skills on 2026-10-14. Ordinary signed canister methods plus the existing Rust SDK
cover this requirement. The gap was a drive facade/UI and shared chunked transfer,
not another transport or identity service. Reuse the immutable legacy chunk and
revision store; add only stable memory ID 13 for bounded upload staging. Validate
live permissions on every call, expected content heads at commit, and SHA-256 before
publication. Keep Candid and existing storage IDs compatible.

Added `/pilveruum/` and `/en/pilveruum/` with legacy `/tooruum/` redirects. My files,
Shared with me, Trash, breadcrumbs, folder creation, 64 MiB chunked upload, download,
rename/move, trash/restore, sharing, content replacement, history download/restore,
and unfinished-upload cleanup use the canonical service/SDK. No new production
dependencies. Root UI has no permanent-delete action. Trash currently has no timed
purge; proposals, search, organization activity and the complete shared Craft picker
remain unfinished specification items. Native Craft editing of shared files is not
advertised as complete; existing owner app links remain supported.

### Resumed browser run

The interrupted upload had reached the disposable canister: its PNG was visible
after resuming. That existing file was left untouched. The automation's long waits
were localized to `filechooser.setFiles` (and one download completion call), not a
reported backend error. A new synthetic upload and a replacement both completed.
The browser tool did not expose enough diagnostics to identify why its upload
confirmation waited, so this is not evidence of a service timeout or its fix.

Actual portal + Rust/Wasm facade + cloud canister, on a separate managed local
network, with two disposable signing identities:

- Uploaded a 624,000-byte text file, crossing the 512 KiB chunk boundary. Downloaded
  bytes matched the input exactly; SHA-256:
  `2df19861a31bbe886f3c3cdc5e028eb8610f3ce281022e072f867ed4f22bf10c`.
- Renamed the file, moved it into a nested folder, sent it to Trash and restored it
  to that folder. Uploaded a replacement and restored the original revision;
  history retained all three revisions and displayed the acting account.
- Alice shared only the synthetic-data subfolder with Bob, with Read + Create.
  Bob saw that shared root without Alice's inaccessible ancestor. He could download
  and create a folder; Edit, Delete, Share and History controls were absent. Alice
  saw Bob's new folder. After Alice revoked access, Bob's old folder link returned
  unavailable, with an empty listing and disabled write controls.
- Renamed and moved the entire test folder to Alice's root; trashed and restored it
  with both children intact. The unfinished-upload list was empty.
- Switched ET/EN without losing the selected folder. Verified the legacy English
  `/tooruum/` redirect retains both query and folder fragment under `/pilveruum/`.
  Checked a dialog fits the mobile viewport and closes with Escape. Checked widths
  320, 390 and 1280 px without horizontal overflow after the CSS correction.

Issues found and fixed during this run: the list border clipped row action menus;
the mobile navigation overflowed after adding Shared and Trash; file rows wasted a
second line on their action button; tiny revisions displayed as 0 KB; action-menu
accessibility labels did not change language; folders with no available actions
displayed an empty menu. Successful mutations now refresh account usage, and a
history restore retains its request ID on retries. No identity or permission
enforcement was moved into the frontend.

Validation evidence:

- The existing final backend suite passed 20 tests in the preceding implementation
  run. This continuation made no backend changes; backend Clippy with `-D warnings`
  passed again.
- All 3 actual-Wasm SDK/PocketIC integration tests passed again, including legacy
  upgrade, all seven Craft format roundtrips, shared chunked transfers, uncertain
  commit retries, permission denials and revoked/moved access.
- All 19 portal locale/navigation/shell tests passed, including an added assertion
  for the translated action-menu label. All 19 Python workflow tests passed.
- `python3 scripts/assemble.py` succeeded with all seven existing editor packages.
  Test identities and browser fixtures remain outside the assembled production
  bundle. The final browser fixture frontend sync included 42 portal assets.

Scope and review: the browser fixture deliberately omitted the seven large editor
binaries; full-fleet deployment verification and shared Craft editor/picker browser
flows are not claimed. Disposable signing identities test real ingress and ACLs,
but not Internet Identity login issuance. The earlier MCP proof is separate and
was not rerun for these frontend-only fixes. Production subnet.ee was not deployed.
The architecture review found no need for a new production component: retain the
existing signed transport, canonical SDK, immutable revisions and server ACLs. UI
capability checks only select controls; service tests establish enforcement.

The local fixture and its state are preserved under `.local/drive-browser-qa`.
Its `drive-tested.jpg` records the final synthetic-data folder. No permanent delete
or production-account mutation was performed.

## 2026-10-14 — Mainnet root drive and MCP rollout

Explicit user authorization: “paneks siis selle mainneti, saan seda testida”.
The operator terms were rechecked: still v1.1, effective 2026-09-02; the previous
operator acceptance applies. The user renewed the existing `efti-nns` session.

- Existing cloud `vbfbj-tyaaa-aaaac-bgegq-cai` was briefly stopped for snapshot
  `0000000000000000000000000041310d0101` (77,843,991 bytes), then upgraded and started.
  No reinstall, controller change, new production identity or data migration.
- Official recipe-built cloud SHA-256:
  `439e88e0d60d0be69e1c832c524bcea14b68eb87f8a037672a2d5a52c67c16bc`.
  The normal Candid compatibility check passed; published Candid exactly matches
  the tested canonical contract. Management status confirms Running and that hash.
- Synced 169 frontend assets to existing `vigkv-fqaaa-aaaac-bgeha-cai`;
  state hash `35c2df7e0c61854e41fe4a3bef87b898c56a86a8d788062f513870e32e5df886`.
  All seven packaged editor binaries were reused. No test fixtures were published.
- `python3 scripts/verify.py portal --environment ic --origin https://www.subnet.ee`
  passed payload, certificate, identity/environment, MCP discovery, app shell and
  missing-path checks. The public browser loads the new drive and enables sign-in.
  14 loading/Wasm integrity regression tests also passed before frontend sync.
- The root UI now runs at `https://www.subnet.ee/pilveruum/`. The connector is
  `https://mcp.internetcomputer.org/mcp`; private drive browsing needs Actions &
  questions because its metadata reads use consensus update calls. Hosted ChatGPT
  login/discovery/file creation still needs the user's interactive acceptance test.

### Shared Craft picker — implementation decision

Requirement: identical folder navigation and permission behavior in every Craft
popup and the root drive, with editor-appropriate styling. Skills already retrieved
on 2026-10-14: internet-identity, canister-security, static-site, stable-memory.
The standard path remains same-origin shared browser identity and signed Candid
through the canonical SDK. No new authentication, MCP transport or storage service.

Reuse the actual portal drive in an embedded selection mode for Open and Save As,
instead of maintaining a second folder browser in seven editors. A small shared
Rust host passes only stable selection IDs/name across a checked same-origin frame
channel. The editor re-reads capabilities/format via the SDK and performs verified
downloads or version-checked uploads itself. Native bytes never travel in URLs or
frame messages. Native capture/open/dirty-state adapters remain unchanged.

This is proposed until the shared picker, per-account stale-response guards,
read-only opens, revoked saves, preserved drafts and seven real app builds/browser
flows pass. The portal retains its own layout; picker mode adopts the editor's light
or dark theme. Proposals/search/activity remain separate unfinished spec items.

## 2026-10-14 — production MCP confirmation and drive responsiveness

The user confirmed an actual ChatGPT mainnet journey: connected to subnet.ee,
created a folder, and created a file inside it. This supplements the scripted
protocol tests and the hosted connector discovery/identity checks.

Performance decision (updated after the user clarified that writes also must avoid global file scans; validation below):
- Requirement: visible upload progress, faster folder navigation, and bounded
  account/folder reads without parsing unrelated users' metadata.
- Skills consulted this session: [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
  and [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md),
  retrieved 2026-10-14. Standard approach: stable B-tree indexes, distinct memory
  IDs, authorization inside methods, and upgrade persistence tests.
- Observed gaps: sequential ancestor/update round trips; whole AccessGraph JSON
  decoding on reads; sequential upload chunks; progress below the file list.
- Smallest extension: one authorized browse response (folder, readable ancestors,
  page), authoritative per-object stable maps with atomically updated owner, parent and recipient indexes,
  a bounded per-session UI cache with fresh authorization/revalidation, and at most
  four upload chunks in flight. No new server, identity layer or canister.
- Keep consensus metadata updates and verified chunk downloads. Uncertified
  private metadata queries are not substituted just to reduce latency.
- The initially considered read-only projection was rejected: it would leave global
  scans on writes. The implemented path uses owner-scoped working sets (currently
  capped at 200 objects) and writes only changed records. Project ownership and
  pending-upload indexes also avoid unrelated-account scans. The whole old graph
  is decoded once during migration, never during ordinary requests. The original
  memory IDs remain reserved; 14–23 hold the new maps/schema marker.
- Per-user canisters/routing are explicitly out of scope. Shared-root discovery
  uses recipient-principal indexes; authorization still uses the canonical live
  evaluator, and private ancestors are omitted from returned breadcrumbs.
- After this migration, do not downgrade to a pre-index binary: memory 9 is a frozen
  migration source. Use a forward fix or an explicitly reviewed snapshot restore
  that accounts for any writes made since that snapshot.
- Proof required: indexed reads versus existing ACL semantics, rename/move/trash/
  revoke and old-state upgrades; unrelated-user reads excluded; concurrent chunk
  completion/retry/commit tests; stale-response/session/cache browser tests and
  visible progress in a scrolled viewport.

### Bounded metadata prefetch and first-load consolidation — deployed

The user's follow-up requirement was to avoid another empty loading view on the
first visit to a child folder. `drive_home` now combines account/root initialization
and the initial authorized browse response into one signed update. A first browse
page also includes a breadth-first catalog of at most 200 metadata entries in the
requested subtree, including its root. Completely enumerated folders are explicitly
identified; reaching the budget never makes a partial folder appear complete.
Empty folders can still be identified as complete at the entry limit. Larger views
retain their existing 50-entry cursor pagination. These limits do not increase the
alpha storage or object quotas.

The initial home prefetch covers the user's own tree. Shared subtrees are prefetched
when their shared root is opened, using the same live ACL evaluator. No file bytes
or full revision history enter the catalog. The portal keeps at most 400 catalog
entries and 12 recent pages in session memory, with a 30-second freshness limit;
logout, account changes, mutations and access errors clear the cache. A first folder
visit can render prefetched metadata before its background browse completes.
Mutating controls wait for revalidation, and the backend always authorizes writes.

Architecture review: the existing stable indexes, Candid transport and shared SDK
are sufficient. No new backend, identity, dependency, query trust model or stable
memory schema was needed for prefetch. The first uncached request still incurs an
ICP consensus round trip. The current
[certified-variables skill](https://skills.internetcomputer.org/.well-known/skills/certified-variables/SKILL.md)
was also read on 2026-10-14: certified query responses with verified witnesses are a
possible later latency improvement, but were deferred; no security-sensitive
metadata was silently switched to uncertified queries. The bounded catalog is an
intentional tradeoff against an unbounded all-tree response.

Validation completed:
- 23 backend unit tests and strict backend Clippy passed. Tests include unrelated
  owner/corrupt retired-graph isolation, direct shared-subtree privacy, stable-index
  persistence and both 200/250-entry prefetch completeness/budget cases.
- 14 SDK configuration/transfer tests passed, including four concurrent chunks,
  out-of-order completion and draining already-started requests before a retry.
- All three PocketIC SDK/upgrade journeys passed against the final Wasm. The
  pre-index upgrade preserves shared ACLs, IDs, revision bytes and a partially
  uploaded file; the new home/browse response is exercised after upgrade.
- Seven drive JavaScript regressions passed, including initial home without extra
  metadata requests, first-visit prefetch while the response is held, access
  revocation, stale-response rejection and progress before reading upload bytes.
  Locale/static-page checks also passed. `git diff --check` is clean.
- The isolated browser fixture completed 4.1 MB and 32 MB uploads with visible
  percentage/commit state. After the final prefetch build, root/child navigation
  displayed cached contents during background refresh and had no console errors.
  Proof images: `.local/drive-browser-qa/upload-progress.jpg` and
  `.local/drive-browser-qa/prefetched-folders.jpg`. This is local signed browser QA,
  not a measured production latency benchmark or Internet Identity issuance test.

Mainnet release record:
- Before the index migration, the existing cloud canister was stopped, snapshot
  `0000000000000001000000000041310d0101` was created (137,294,602 bytes,
  timestamp 1791980069476049333), then the canister was upgraded and restarted.
- First performance backend SHA-256:
  `436acb3cb9308ca044bc2ee1ec03f9b7181de634c440ec2a2adad69eb13969a6`;
  frontend state hash
  `cd00b74680ff19d9a885676fa259e3a66ed8a62483f5f6a07dbedcc9a19b5185`.
- Final bounded-prefetch backend SHA-256:
  `b88c8978075f4fb89758e5203cb785a3f967e6feeef43acb4dc21f6fd9f9916d`;
  frontend state hash
  `430fa3a0d8be9b39d9cd7452ed12bd2f1c48492269950dd4a4af31a61ccb26a1`.
  Status confirms Running with that module hash. The second upgrade preserves
  the first release's stable schema. No reinstall or snapshot restoration occurred.
- Both releases reused all seven editor packages and synced 169 frontend assets.
  `python3 scripts/verify.py portal --environment ic --origin https://www.subnet.ee`
  passed after each: exact portal payloads, certificate/identity checks, MCP
  discovery, existing app shells and missing-path handling. Editor Wasm bundles
  were not rebuilt or downloaded by this portal-only verification.
- Fresh hosted ICP MCP discovery sees `drive_home` and the catalog contract; an
  authenticated production `drive_home()` returned `Ok` with the catalog. The
  earlier production `drive_browse` smoke check also passed. Only metadata was
  inspected; these checks did not create, modify or delete production files.

Remaining scope: full shared Craft picker integration, proposals and separate
agent credentials remain unfinished as recorded above. This release completes
the root drive upload-feedback/indexing/prefetch work, not those separate features.

## 2026-10-14 — file previews, sharing indicators and context menus

Requirements: double-click/keyboard/menu opening for ordinary files; inline images,
text, audio and video; a subtle sharing marker on files and folders. Relevant ICP
skills, already retrieved this session and reviewed before choosing the approach:
canister-security (live authorization and verified reads) and static-site (certified
assets and CSP). Reuse the canonical signed SDK download and its manifest/hash
verification, native browser media elements and the existing portal. No new service,
viewer dependency, identity or public file URL is necessary. Text is bounded and
inserted as text; SVG is only loaded as an image, never executable page markup.
[MDN SVG image restrictions](https://developer.mozilla.org/en-US/docs/Web/SVG/Guides/SVG_as_an_image)
and [media behavior](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/video)
were checked. Browser codec support remains a constraint; unsupported files retain
download. Media initially downloads and verifies the whole file, within the existing
64 MB quota; streaming/range delivery is deferred.

Sharing status is a derived response field from already-indexed active, unexpired
human grants on the item's ancestry, not another per-row request. Owners see any
effective human sharing; recipients see only their own direct/inherited access,
without other recipient identities or private ancestor names. Do not infer sharing
from owner identity alone. No stable schema change is required; old stored mutation
receipts default the new response field. Validate revocation, expiry, moves, safe
rendering and stale preview/session cleanup before deployment.

The sharing field is Candid `opt` as well as serde-defaulted: the first PocketIC
upgrade test caught that a required added return field prevented the new client
from reading the old pre-index server before upgrading it. The optional form now
passes that actual old-server/new-client journey. Existing clients can ignore it.
The temporary required-field prototype was deployed only to the isolated local QA
canister; replacing that prototype with `opt` required a local-only compatibility
override and matching frontend sync. Production never received the prototype.

The user also requested ordinary contextual-menu behavior. Opening a second
file menu closes the first; clicking outside or pressing Escape closes open menus.
The fix shares one menu lifecycle across file and folder rows.

Implementation and validation:
- Double-click on a row, Enter on its focused row, the visible Open button and
  the menu's Open action use one opening path. Owner Craft project links keep
  their existing app routing. A dedicated modal retains the current folder.
- PNG/JPEG/SVG and other supported image formats use an `img`; audio/video use
  native controls without autoplay. UTF-8/BOM-tagged UTF-16 text is literal text
  in a `pre`, capped at 1 MiB. Unknown formats, decoding failures and larger text
  retain a download action. No HTML injection, embedded file page or public URL.
- Preview object URLs are revoked on close; media is stopped and detached. Session
  changes clear the modal and bytes. Late completions cannot repaint a closed or
  replaced preview. At most one preview download runs at a time; closing discards
  its result, but does not cancel the SDK's already-started download. Stale queued
  requests never start. Resumable/range-based preview streaming remains deferred.
- 24 backend unit tests, strict Clippy, all three actual-Wasm PocketIC upgrade
  journeys, 22 drive/locale JavaScript tests and three static-localization tests
  passed. Indexed browse tests assert both direct and inherited sharing flags.
  JS tests cover inert text/SVG, object URL/media cleanup, late account responses,
  size/format fallback, translated sharing markers and single/outside/Escape menus.
- Actual browser QA uploaded seven synthetic/local test assets into folder
  `Eelvaated` in the isolated local canister: TXT, SVG, PNG, JPEG, WAV, MP4 and an
  unsupported binary. All supported previews rendered; WAV/MP4 decoded to two
  seconds and both played through their controls. The SVG's test script did not
  execute. Unknown binary showed download fallback. The menu test confirmed one
  open menu at a time and closing on outside click/Escape.
- A Read-only grant of that synthetic folder to test account B showed the direct
  folder marker and seven inherited child markers. B could preview the PNG while
  Create remained disabled. Switching back to A closed/emptied B's open preview.
  Test account A was restored. No production file permissions or content changed.
- Visual proof: `.local/drive-browser-qa/file-preview-svg.jpg` and
  `.local/drive-browser-qa/shared-folder-marker.jpg`. A stale browser viewport
  override initially misdirected test clicks; resetting it restored normal input.
  Browser file-chooser confirmation also delayed the batch upload; that elapsed
  tool time is not an upload-performance measurement.

Architecture review: no new component/dependency, stable-memory ID, transport or
CSP exception is needed. Derived sharing flags add no per-item network call and
are not permission checks. Standard browser image/media contexts plus literal text
are sufficient for the requested basic viewer. The shared Craft picker and native
editing integrations are separate unfinished work.

Mainnet deployment reused the existing canisters and all seven packaged editors.
The normal production Candid compatibility check passed without an override.
Backend SHA-256: `34a26aad2ef9a21edfc0fcf00f9a61d6bdb67c5ad0bc611e4d63e751404b27eb`.
Frontend sync: 169 assets; state hash
`71287a0251807db87147777fbff9edeb3e8ea013080bee2b42a39d7638668a3e`.
Management status confirms Running with that backend hash. The production
`verify.py portal --environment ic --origin https://www.subnet.ee` check passed
exact portal/runtime payloads, certification, identity/MCP discovery, existing
editor shells and missing-path handling. No editor rebuild or storage migration.

### 2026-10-14 — Clickable file and folder cursors

Actionable file/folder rows now use the pointer cursor, inherited by their names
and icons. The rule targets the existing keyboard-openable rows; trash rows do
not acquire an opening affordance. This is a CSS-only change using the existing
static-site assembly/sync workflow, with no new dependency or architecture.
Browser computed-style checks passed for both file and folder rows and titles.
Visual proof: `.local/drive-browser-qa/clickable-rows.jpg`.
Mainnet frontend sync completed with 169 assets and state hash
`a3cee7d69cf572e34b4fd7e805a94b0d0c36e7e178855895e7166bcc30d6c92a`.
Production `verify.py portal --environment ic --origin https://www.subnet.ee`
passed exact asset, certification, discovery and missing-path checks.

## 2026-10-14 — Craft shared-drive integration, resumed

The earlier picker decision is now being implemented. Skills consulted this session:
[Internet Identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md),
[canister security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [static site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md).
The supported same-origin session and authenticated canister methods already meet
identity and authorization requirements. The observed gap is the old UI's personal
project catalog, ownership-only bindings and legacy transfer calls. Replace those
with the existing drive API and one embedded portal picker; do not add another
permission system or backend service. Bind documents to the acting account and
stable drive file ID, independently of the file owner's principal. Revalidate
format and permissions at transfer; preserve version conflicts and server-side
revocation checks. Read-only files can be opened and copied with Save As; Create
and Edit remain separate. Validate shared create/open/edit, read-only/revoked saves,
conflicting versions, account switching, and all seven app adapters before release.

Validation in progress: the actual local canister test now covers Bob creating and
reading all seven Craft format kinds in Alice's shared folder, author/owner
separation, revision conflicts, wrong-space references, Read-only downgrade and
revocation. The SDK adds a preflight format/access check without replacing the
server checks. Browser QA has saved and reopened a real WordCraft document as B,
updated A's file (history shows B), and created a second native document in the
shared folder. A cold direct-link launch exposed a host-font initialization race:
WordCraft installs named styles one frame before egui applies the corresponding
fonts. Cloud status windows now wait for the fonts to become available; a native
regression test reproduces that two-frame lifecycle without changing host styles.
The shared UI's 10 tests and strict Wasm Clippy pass. All editors are being rebuilt
with this fix; direct-link and per-editor save/open smoke checks remain pending.

The local browser permission test also confirmed that Read+Create without Edit
can open a native document, blocks its Save with a readable Save As explanation,
preserves the edited text, and successfully creates a new copy in the same shared
folder. The portal sharing form adds grants (union of permissions); the test
removed the older Edit grant before checking the narrower grant. Improving that
existing roster into an explicit per-recipient permission editor is separate UX
work, not a second authorization rule for Craft. Proof images are
`.local/drive-browser-qa/craft-shared-open.jpg` and `craft-readonly-save.jpg`.

Deployment tooling evidence: the pinned `@dfinity/static-site@v0.4.0` plugin
exceeded its configured 600-second compute limit while Brotli-compressing the
173-asset QA bundle; it had not committed a new asset state. DFINITY's v0.4.0
`docs/how-it-works.md` documents canonical gzip/Brotli and native use of the same
sync crates. Reuse the workspace's existing `.local/certified-assets/cli-transport`
helper with the unchanged upstream sync-core and canonical compressors. The only
local test extension is an optional network argument (default remains `ic`);
identity and calls still go through icp-cli. This is deployment tooling, not a
new application transport, identity or permission mechanism. Certification and
production byte verification remain required.

Final local browser validation uses the isolated `drive-browser-qa` canisters and
disposable accounts A (owner) and B (recipient), not production files. WordCraft's
cold launch now opens the saved shared document without the font initialization
panic. The final iframe layout also accepts ordinary pointer clicks in both themes.
PhotoCraft saved/reopened a painted native project; PdfCraft a one-page PDF;
DeckCraft an eight-slide presentation; SoundCraft a 63.1 MB portable audio project
with its tracks and waveforms; CADCraft a DXF line drawing. These checks used B's
Read+Create grant on A's folder. They exercise actual native encoders/decoders,
in addition to the seven-format backend contract test. GridCraft also saved and
reopened a cell value in the shared folder. An additional GridCraft Save As hit
the owner's pre-existing file-count quota; the draft remained intact, Dismiss
attempt allowed another Save As into B's own space, and the full pasted control
text reopened correctly there. SoundCraft visibly reported upload percentage.
Account A was restored after browser QA. Screenshots of each native reopen and
the shared light/dark pickers are retained in `.local/drive-browser-qa/`.

Scoped checks pass: 19 cloud UI JavaScript tests; 16 SDK unit/transfer tests;
10 shared UI tests including the cold-font regression; the actual-canister
seven-format shared permission/conflict test; locale/shell/loading/Wasm checks;
localization/scoped workflow checks; shared UI strict Wasm Clippy; all seven
release Wasm builds; canonical-vendored SDK comparisons; and `git diff --check`.
The separate architecture review found no need for a new identity, permission
store, transport, canister or document format. The embedded portal is the only
new browsing surface; authenticated drive methods remain authoritative.

The first full local asset commit was explicitly rejected for insufficient
simulated cycles; topping up that local canister and replaying the retained public
asset commit succeeded. The final local build is state
`7aa20aac8ce184292a4a9989cda879614f30cbc4f689b08a7953c005a63ef5a7`.
The production candidate is isolated in `.local/craft-release/dist`: it contains
the seven final editor bundles and shared picker, but preserves the published ET/EN
landing pages and landing CSS/JS. The unrelated homepage demo being prepared in
another chat is excluded. Native sync uses the same DFINITY compression cache;
the candidate contains no disposable session fixtures. No backend deployment or
schema migration is part of this Craft release.

Mainnet release completed: 169 assets, state hash
`c1547dc5c8725b8decb7526eb06bfe59e4568deb26012b434e7286b15f15c99f`.
The isolated candidate's `verify.py all --environment ic --origin
https://www.subnet.ee` passed exact page/JS/Wasm bytes for all seven editors,
certification/cache/CSP headers, runtime canister IDs and mainnet trust settings,
Internet Identity/MCP discovery, and missing-asset/session-fixture 404 checks.
This verification uses the ordinary repository verifier with only the unrelated,
unpublished homepage-demo asset assertions omitted in its local release copy.
The published WordCraft started in the browser and correctly requested sign-in
when Open was invoked without a valid user session. Two-account content writes
were tested locally, not against production user files. The backend was unchanged.
