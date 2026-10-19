# Development history

[Project overview](../README.md) · [Development](development.md) · [Deployment](deployment.md)



## Git publication and mainnet release — 19 October 2026

The user authorized publication of the completed team-space/public-link/search
package and all eight reviewed Craft updates. Pushed all eight fork `main`
branches before root `4f1b707` and the fixture/provenance attributes follow-up
`0d5fd46` to `eerikpotter/subnet.ee`. No standalone Craft canister was changed.

Consulted the official `icp-cli`, `agent-web-identity`, `stable-memory`,
`cycles-management` and `static-site` skills on 19 October. The supported path is
the existing pinned Rust recipe and static-site v0.4.0 sync, using a fresh owner
web delegation in Keychain. No new deployment component was needed. The explicit
code-only backend command preserves private environment settings; frontend sync
uses `ICP_CLI_PLUGIN_COMPUTE_LIMIT_SECS=3600` for the eight-app compression work.

The owner authenticated `codex-subnet-release-20261019`; its principal matched the
existing controller. Mainnet cloud `vbfbj-tyaaa-aaaac-bgegq-cai` upgraded normally,
including the Candid compatibility gate, from
`c6104cf1cfb371bdbb18dda4fc25d8e6c5d3606967ef7dd8aefcaca8b6f9b1a3` to
`c0c16e070a1b80565b6697ff273ad1289c7934f5d0d4ef88428e5ae4ecc2a79d`.
No reinstall, schema reset or settings update. The deployed module hash and public
Candid match the release artifact; `email_profile` reports the provider enabled.
No mail was sent. Existing owner/CycleOps controllers were retained.

Preflight balances were 1.263T cycles for cloud and 3.253T for frontend; this
release did not convert ICP or purchase additional cycles. The prior-source
populated upgrade rehearsal and 344 local HTTP checks passed. Frontend sync
completed with 460 assets and the nonzero final state hash
`bda7974d1f1f7442cd90eadda412be172e7c89b1b3099e725314bee5e952c359`.
`verify.py all --environment ic --origin https://www.subnet.ee --timings` passed
all 344 HTTP payload/certification/runtime-trust/cache checks, including all eight
updated editor packages. The real production browser rendered signed-out Cloud
Space with the updated navigation. No production test accounts or files were
created. Detailed save/open and permission journeys were exercised locally.
The 36-item feature review and its served local copy now state the actual released
scope and remaining limitations. Release artifacts and bounded logs are retained
under `.local/releases/2026-10-19/`.

## Craft upstream integration and author-profile inspection — 19 October 2026

Updated all eight fork `main` branches from exact reviewed upstream commits using
`update-crafts.py check/prepare/promote`. Review worktrees isolated conflicts and
backup refs retain each previous fork HEAD. The initial update was local; the user
subsequently authorized GitHub and mainnet publication. Existing
team/public-link/content-search work was preserved; editor builds use
their independently pinned SDK snapshots, without silently adopting that new SDK.

Pinned fork / upstream pairs:

- Word: `dcd51979cb05` / `117aea31a8e6`.
- Grid: `202ab729d5cd` / `66ce6be4743f`.
- Deck: `ea5c467f4031` / `010941ec60f8`.
- PDF: `6087df68e236` / `43550c448db3`.
- CAD: `cd7fa0239795` / `0b32c2e6a14c`.
- Fig: `87141a7899df` / `a0d1162976fa`.
- Photo: `55434cae468f` / `d95fdf3181c0`.
- Sound: `9338597cef16` / `25d52c646aea`.

Manifest, build-script and dependency deltas were reviewed before execution.
Conflicted lockfiles keep the reviewed parents' external package pins; Cargo
canonicalized the merged graph without opportunistic upgrades. PDF's authoritative
asset ledger was merged and its generated attribution was regenerated and checked.
Cloud menu hooks coexist with the upstream QAT, backstage, host bridge, inbox and
new-document changes. Word's start screen now reports no document to the cloud
adapter; a regression checks capture/open/late-save behavior. CAD's menu loop keeps
the upstream category name needed by the existing cloud hook.

A real browser launch found a separate upstream Deck regression: recovery instance
initialization called `std::process::id()` in Wasm and trapped before the editor
opened. The narrow platform guard keeps OS-process/time entropy native-only,
leaving desktop recovery and document counters unchanged. Five native recovery
tests passed. This is a startup compatibility repair, not a replacement of the
editor's local save/export workflow.

All eight updated browser apps completed cloud save/open against the local canister:
Word retained an autosaved text edit; Grid a cell edit; CAD a line; Fig a rectangle;
Photo a brush stroke/layer; PDF an existing two-page document copied with Save As;
Sound a 63.1 MB nine-track session; Deck an eight-slide presentation. Browser cache
updates were accepted through the shell. Final `verify.py all` passed 344 HTTP
payload/certification/trust/cache checks. Logs and screenshots are retained in
`.local/craft-updates/`; no additional editor-wide test reruns followed.
All eight cloud-feature Wasm checks and release builds were exercised. Focused
adapter tests passed for Word (1 host + 2 codec), Grid (2), CAD (1), Fig (4) and
Photo's actual cloud-menu interaction (1). The updater's five offline tests passed.
Sound's full `xtask ci` passed (1,149 tests; one ignored). Its first attempt lacked
local socket permission; rerunning with local socket access passed. PDF's initial
non-UTF-8 filename fixture was similarly blocked by sandbox permissions; the exact
binary test passed with local filesystem access. The full rerun passed all eight
PDF CI steps (3,761 tests; 15 ignored).

Full CI was also attempted for Word, Grid, Deck, CAD, Fig and Photo. Their unchanged
upstream code fails newer Clippy `collapsible_match` / `nonminimal_bool` warnings
in proof, XLS/XLSX, Matroska, NURBS, several Fig UI modules and Photo app directories,
respectively. Those unrelated failures were not rewritten in these integration
forks. Passing cloud tests/builds is not a claim that every upstream CI gate passed.
The user then explicitly narrowed future update validation: root AGENTS.md and
`scripts/UPDATING.md` now override inherited full-CI-before-commit defaults for
these integration forks. Build, affected adapter/menu tests and browser startup form the routine gate;
cloud save/open is added for changed hooks, lifecycle or serialization, or a
concrete unresolved integration risk. Shared tests run once when changed. Broader editor
tests require an explicit request or a concrete unresolved concern.

A release-specific `workflows` rehearsal now accepts an optional previous
UUID-schema artifact. Built the previous source from `a1f1cbb` (backend/SDK sources
unchanged since the deployed release); its Candid exactly matches live mainnet.
The populated previous-to-current upgrade preserved content, sharing, manual
versions, activity, labels and retry receipts. This was a source rebuild, not a
byte-identical reproduction of the old production module. It adds no numeric
legacy migration. Verbose logs stay in files and the agent sees bounded summaries.

Profile inspection found existing display names/initials, not a shared Craft
account. All hosts already share `Cloud`/`BrowserSession`; the proposed connection
is an optional current-principal author ID through that adapter, alongside existing
labels. No additional account service, authentication flow or profile UI is needed.
This update does not yet persist principal IDs in comment formats. The precise
boundary and required account-switch/format roundtrips are in
[collaboration-spec.md](collaboration-spec.md#editor-author-identity--inspection-19-october-2026).

## Local team spaces, public links and content search — 19 October 2026

Implemented the approved three-part package in the canonical Rust service, SDK
and portal. One data canister remains; no editor engines or independently pinned
fork snapshots changed. No Git push or mainnet deployment was performed.

Team membership uses space UUIDs with Manager/Editor/Viewer roles, multiple
Managers, last-manager protection, member departure, external-sharing settings
and space-based quota/pending accounting. Founders have no special authority after
leaving. Members currently require an existing Subnet account ID; email team
invitations and enterprise administration remain future additions.

Public UUID URLs now support anonymous current-content preview/download and
bounded folder listings. Owner/Manager policy has optional expiry, revocation,
folder inheritance and a team external-sharing override. Public projections omit
private ancestry, owner, authors, recipients and old revisions. Every chunk checks
live policy and the SDK verifies the immutable manifest. Public exposure is included
in reviewed move audience changes; it does not create named access or global search.

Content search extracts canonical committed bytes into a revision-bound derived
index. TXT/Markdown, WordCraft, PDF text, DOCX, XLSX and PPTX are supported. A stable
queue, standard CDK timers and isolated self-calls keep parsing out of file commits;
post-upgrade backfill is incremental. Exact normalized token AND queries use scoped
postings, bounded candidates and live authorization before returning snippets.
Current limits and explicit partial/unsupported/failed states are documented in
`services/sovereign-cloud/api.md`; there is no OCR, ML, semantic search or external
extraction service. The 100k-user target has not been load-tested.

Actual Wasm validation rejected upstream lopdf’s giant glyph-name match (function
complexity 2,734,911; the z profile also failed). The pinned dependency has only
that lookup replaced with a sorted binary-search table. All 4,495 names/aliases
and the hashes of every unchanged upstream file are checked. The standard Rust
recipe then installed and indexed PDFs successfully. Browser QA caught PdfCraft’s
ordinary PDF bytes carrying a Craft association; the extractor handles this and
version 2 backfills affected files automatically. The real Wasm test now includes
that exact association. Office fixtures were generated and reopened independently
with python-docx/openpyxl/python-pptx, then passed through the canister indexer.

Validation:

- 77 canonical service unit tests; 6 extraction tests including XML entities,
  ZIP/PDF expansion, repeated XLSX values, output and term limits.
- 12 shared SDK transfer tests, including verified anonymous reads/revocation.
- 195 portal JavaScript tests, including team creation/admin retries, public link
  settings, anonymous preview, bounded content pages and stale detail responses.
  A final selection regression prevents ordinary selection from falsely marking
  rows as cut (optional boolean needed an explicit false).
- 6 PocketIC flows: 4 shared adapter contract tests, populated team upgrade,
  and actual six-format timer/index/authorization/upgrade flow. PDF association
  and independently generated Office fixtures were retested in final Wasm.
- Python: 4 localization, 10 scoped-build and 2 dependency-provenance tests.
  Backend clippy and `git diff --check` clean. Portal build and local verification
  pass; the eight existing editor packages were reused.
- Real browser: Manager creates/administers a team; Editor sees it and creates a
  folder without management controls; anonymous folder/text preview; six uploaded
  content formats found by words absent from their names; PDF details reports
  searchable. Evidence: `.local/public-link-proof.png`,
  `.local/content-search-proof.png`. Browser fixture upload automation was slow;
  subsequent canister operations and all six uploads completed.

Existing local `cloud` was upgraded code-only using the controller Keychain
identity, preserving files and private email settings. Frontend synchronized at
`http://frontend.local.localhost:61304/`; disposable browser fixture pages removed
before final verification. The 36-item HTML review and its local served copy now
separate delivered core flows from remaining group/SSO/OCR/sharing-policy features.

## Mainnet cloud drive release and Git publication — 18 October 2026

Published all nine `main` branches after explicit authorization: root release
`e4a6c10d7e84929a4ae6b2edaf13428bbcfc9b38` in `eerikpotter/subnet.ee`
and the eight Craft commits pinned by that root. No standalone Craft deployment
was changed. Existing validation is recorded in the two following milestone entries.

The authenticated owner session was renewed with Internet Identity and stored in
Keychain as `codex-subnet-release-20261018`. Mainnet cloud
`vbfbj-tyaaa-aaaac-bgegq-cai` contained the previous numeric-ID schema. Under the
agreed disposable-development-data policy, it was stopped, snapshotted and freshly
installed with the UUID backend, then started. No legacy conversion or ID aliases.
Snapshot `0000000000000003000000000041310d0101` (232,090,483 bytes) remains on the
canister and was verified in its snapshot list after installation; previous
snapshots were retained. The old module was
`0a62f6a6a8216b4298a53f1266284adb9880281a65f2fe9de70eaf125b947089`.
Restoring this snapshot restores the old program and data together and requires
matching old frontend assets; it also discards subsequent writes.

The official Rust recipe built backend module
`c6104cf1cfb371bdbb18dda4fc25d8e6c5d3606967ef7dd8aefcaca8b6f9b1a3`;
its deployed hash and public Candid exactly match the release artifact/contract.
Public `get_api_doc` succeeded and anonymous `drive_browser_home` was rejected.
Resend settings were provisioned through the owner from Keychain, with no secret
in repository files or command arguments. Application `email_profile` returned
`enabled = true`; no email was sent during this release.

The existing second controller is the immutable CycleOps V3 status monitor, not
an alternate owner. Its live hash matched the upstream published binary and its
controller list was empty. Its reviewed typed response omits environment variables
and it has no management mutation methods. Added an explicit hash-checked exception
to the operator helper, retaining owner-only configuration and private visibility
without removing monitoring or accepting arbitrary additional controllers.
Eight operator tests passed. Details and the primary source are in
[the deployment review](deployment.md#cycleops-monitor-and-mail-secrets).

The first frontend attempt used the official static-site v0.4.0 sync. It
planned 456 assets, prepared 259 changed assets (197 already current), and started
session 26. After uploading roughly 378 MB of raw/compressed content, `upload_chunks`
was rejected with `IC0207` because the canister reached its freezing reserve.
It requested at least 3,227,141,030 more cycles for that next call; that minimum
does not cover the remaining release. The balance was 663,177,243,643 cycles against
22,047,166,183 idle cycles/day and a 30-day reserve. No freezing or access-control
setting was reduced. That attempt left `state_hash` at zero and was not a finalized
frontend release.

Read the official `cycles-management` skill and checked the existing owner ledger:
0.031T cycles was insufficient for a full retry. Requested explicit approval to
convert at most 1 ICP and deposit 2T cycles into the existing frontend before
retrying the standard sync. The user approved that exact budget. Converted 0.966 ICP (total debit
**0.9661 ICP**, including the ledger fee), receiving 2,000,486,000,000 cycles;
deposited exactly **2T** to frontend `vigkv-fqaaa-aaaac-bgeha-cai`. The subsequent
canister balance was 4,068,771,315,644 cycles, above its then-current 30-day reserve
of 661,414,985,490 cycles. Failure evidence:
`.local/mainnet-frontend-sync.log`; funding receipts and retry output are in
`.local/mainnet-cycles-mint.json`, `mainnet-frontend-top-up.log` and
`mainnet-frontend-sync-funded.log`. The local deployment was not stopped or reset.

The funded official sync **completed successfully**, publishing 456 assets with
final state hash
`55742e3e12db9695de8327f7bd3decba54265351219c5e47bb930458e5a63fb7`.
A separate canister call confirmed the same hash. Both canisters were running;
the frontend retained its existing static-site module and had approximately 3.27T
cycles afterwards. Controllers and freezing thresholds were unchanged.

`python3 scripts/verify.py all --environment ic --origin https://www.subnet.ee --timings`
passed: 342 asset responses matched the local release byte-for-byte, including all
eight editor bundles and the local PDF runtime. Certification headers, MIME,
cache policies, CSP, mainnet trust configuration, both languages' UUID deep links,
Internet Identity discovery, MCP discovery and missing-asset responses also passed.
Browser smoke testing at the public origin confirmed the signed-out Home view,
enabled sign-in controls and navigation to all eight application launchers, with
no captured warning/error messages. Screenshot: `.local/mainnet-release.jpg`;
verification output: `.local/mainnet-final-verify.log`; sanitized canister evidence:
`.local/mainnet-final-status.json`. Authenticated daily file flows were tested
locally as described below, not repeated on production accounts during this release.

## Daily file workflows and accurate 36-group review — 18 October 2026

Completed the existing daily workflow slice without changing the backend schema,
SDK, shared editor adapter or any editor-native source. Browser verification found
and fixed a portal JSON/Candid boundary regression: `drive_list` still parsed a
UUID parent as a number, breaking upload collision checks and destination pickers.
Email retry/revoke had the inverse error (a numeric job parsed as an object UUID).
Their serialization is now a small host-testable module in the portal facade;
revision/operation parsing is named explicitly. No alternate file ID or alias layer.

Previews now include a locally hosted, pinned Mozilla PDF.js display controller:
one page/canvas at a time, page/zoom/fit controls, inert accessible page text, local
fonts/codecs and cancellation on close/session change. The initial PDF fixture had
empty form fields and was not a valid visual-content test; a separate generated
2-page text PDF proved rendering and page navigation. Office remains a bounded,
simplified text/value preview. PDF, Office and media controls retain arrow-key
ownership. File menus are positioned within the viewport and scroll internally
when tall, including bottom rows and phone layouts. Cut-file dimming applies to
content, leaving action menus opaque and above neighboring controls.

Real browser flows against the running local canister used disposable accounts
on isolated localhost origins (no real-user session injection):
- Uploaded PDF/DOCX/XLSX/PPTX/PNG through the ordinary queue; verified literal DOCX
  markup, worksheet gaps/names, slide order and PDF page navigation.
- Account B: inaccessible UUID link -> request; A: explicit Viewer decision;
  B: readable file; A: revoke; B: inaccessible again.
- WordCraft using the unchanged common adapter: edit -> autosave -> reopen with
  saved text; new unsaved document -> persisted local draft -> second editor
  window -> explicit restore as copy, without overwriting the cloud file.
- Activity feed records content and sharing changes; storage separates current
  and historical bytes; version UI identifies the automatic checkpoint.
- 390px browser viewport: final-row/cut-file menu stays inside the viewport and
  readable; its details action opens current metadata; the two-column grid has no
  horizontal overflow and loads the PNG thumbnail. This is not a physical-phone
  touch or comprehensive screen-reader audit.

Validation: `node --test scripts/*.test.mjs` **185 passed**; portal facade
`cargo +1.95.0 test --lib` **2 passed**; facade wasm32 clippy `-D warnings` passed;
localization Python tests **4 passed**; portal release build and assembly passed,
reusing all eight packaged editors. Native `cargo test` for the browser binary
cannot import its wasm32-only SDK; the request boundary library is tested on host
and the actual facade is compiled/linted for wasm32. An accidentally overlapping
Trunk invocation failed; the final build was serialized and passed. Existing
backend/PocketIC and eight-editor checks from the preceding entry were not rerun
because their implementation did not change.

Local frontend sync succeeded with 455 assets, state hash
`208610388e64e46fd1dddb464e69a958bb7c77a225a9af71714e6cf9d0aad340`.
The deployed asset verification passed, including every local PDF runtime, font,
character map and codec (payload, certification, MIME and revalidation).
No backend reinstall, settings update, mainnet deployment, email send or Git push.
Proof images: `.local/daily-pdf-preview.jpg`, `.local/daily-draft-recovery.jpg`,
`.local/daily-mobile-menu.jpg` and `.local/daily-feature-review.jpg`.
The canonical HTML review remains `docs/cloud-drive-feature-review.html`, mirrored
at `.local/drive-feature-review/index.html` for the existing port 61305 review.

All 36 groups retain concrete boundaries: 29 available core flows, 1 partial
accessibility/mobile group, 5 separate future groups and 1 explicitly deferred
content-search group. No content index, OCR, ML or AI search was added. 100k users
is a design target, not a demonstrated capacity claim. The storage overview still
aggregates one account; large-account pagination/incremental accounting and broad
screen-reader/editor-mobile audits remain explicit follow-ups.


## UUID identities and scoped catalog operations — 18 October 2026

The authorized technical milestone keeps one data canister and one canonical UUID
v4 per object/space, with no numeric alias or migration layer. The development
schema guard refuses numeric-ID state before opening differently typed maps.
Rename/move/content/grant operations prepare changes to the relevant object/path,
then maintain canonical records and derived indexes together. Name collisions,
ordered listing, owner counts and sharing discovery use scoped stable indexes.
Folders, spaces, ACL roots and viewer policy have independent revision scopes;
there is no canister-wide cursor epoch.

Bounded search examines at most 512 candidates and returns 50 matches. Continuation
is opaque and authenticated; private filtered positions cannot be forged or read.
Empty pages retain a next-page action. Suggestions inspect bounded windows, and
opening/starring no longer scans all personal metadata. The per-object size index
also follows replacement and current-revision removal. UUID URLs use certified
SPA rewrites in both languages. The SDK and shared UI absorb cloud identity;
all eight editor-specific `cloud.rs` adapters and the `Editor` trait are unchanged.

Validation:

- 68 canonical service unit tests; Clippy `--lib --tests -- -D warnings` clean.
  Tests include 100,000 unique UUID allocations, same-owner 200/5,000-object
  storage-operation comparison, private empty-page continuations, malformed and
  cross-user bookmarks, sibling/account isolation, and same-schema reopening.
- 19 SDK tests and 12 shared UI tests; the shared Wasm adapters compile in their
  statically linked editor hosts. No editor engine suite is required by this change.
- 15 PocketIC flows across `adapter_contract`, `browse`, `persistence`, `recovery`,
  `workflows`, `trash` and `email`: populated same-schema upgrades, immutable bytes,
  permissions/revocation, lost-response retries, activity, UUID shortcuts and purge.
  No historical backend binary is used. Email's three tests pass with the official
  October launcher; the generic 16.0.0 test binary failed to produce its HTTP v2
  request. Provider responses are mocked and no real mail is sent.
- The 256 MiB purge test advances empty replica rounds before code upgrades;
  immediate management chunk upload after a large batch hits heap-delta throttling.
  This proves accounting/persistence, not sustained write throughput.
- Portal: all 180 tests passed, including the UUID language-switch check. Python: 10 scoped
  workflow and four localization tests. Canonical standalone service snapshot also
  compiles; its dependencies and API document are now included in snapshot checks.

Local UUID-schema reinstall completed for `4caro-hl777-77775-aaaba-cai`, using the
sole Keychain controller and code-only install. The user authorized disposable test
data/no migration. Resend settings were verified configured before and after;
provider credentials were not printed or added to files. No mainnet deployment.
Browser verification on an isolated local test origin used the actual backend and
assembled release Wasm: create a UUID folder, switch language, reload its deep link,
launch WordCraft into that folder, save through the shared picker, and reopen the
saved document with its text intact. The file deep link survived reload. A second
disposable account received an unavailable/access-request view without the file
name or contents. No editor-specific adapter change was needed. Screenshot:
`.local/uuid-adapter-browser-proof.jpg`.

The official static-site plugin exceeded its 60-second default and then a
600-second compute allowance while Brotli-compressing the 221 MB editor bundle.
`ICP_CLI_PLUGIN_COMPUTE_LIMIT_SECS=3600 icp sync frontend -e local` completed:
252 assets, state hash
`b3d46f662de295bb1d445da36bb56312ab229bb4278300cdcee234fdfa5e484e`.
`python3 scripts/verify.py portal wordcraft` passed against the actual local
canister, including all four certified UUID file/folder rewrites, portal assets,
all editor entry shells, WordCraft HTML/JS/Wasm bytes and trust/missing-asset checks.
The source checks also confirm unchanged native editor code in all eight forks.

This is not a 100,000-user capacity claim. Existing product quotas, explicit
subtree/aggregate bounds and single-canister hosting remain. Multi-canister routing,
resumable large jobs and deployment throughput benchmarks remain separate work.

## Folder navigation, details and recovery — 18 October 2026

Delivered the authorized next milestone locally: an on-demand paginated sidebar
folder tree, a caller-scoped details/access panel, Undo for rename/move/Trash, and
restore with a different name or destination. The details and recovery controllers
are separate portal modules; the tree reuses catalog browsing. Shared Rust types,
Candid and the browser facade expose details and reviewed restore through ordinary
authenticated methods. No stable schema, memory IDs, canisters or routing tables
were added. Details filter inaccessible ancestor names and show only the caller's
live access sources, never another recipient's email address.

Undo issues ordinary version-checked inverse commands using successful mutation
receipts. It retains exact requests across ambiguous responses and does not repeat
successful members of a partial batch. Access changes require confirmation; an
unrelated newer edit or revoked authority cannot be overwritten. Reviewed restore
reuses the move-impact planner, checks name/destination and tree versions, requires
Edit + Delete, and requires Create when changing parent. Its retry path rechecks
the same authority. Plain Restore keeps its existing permission contract.

Validation completed:

- `cargo test --manifest-path services/sovereign-cloud/Cargo.toml --lib --quiet`:
  52 tests passed, including hidden ancestors, stale Undo, revoked rights,
  destination/name conflicts, cross-space rejection and access-impact checks.
- `node --test scripts/cloud-i18n.test.mjs scripts/drive-transfers.test.mjs
  scripts/download-archive.test.mjs scripts/i18n.test.mjs`: 113 tests passed,
  including lazy pagination, session changes, details refresh, restore confirmation,
  partial Undo and exact retries after lost responses.
- `python3 -m unittest discover -s scripts -p test_localize.py`: four tests passed.
- `cargo test --manifest-path services/sovereign-cloud/Cargo.toml --test recovery
  --quiet`, with `POCKET_IC_BIN=/tmp/subnet-trash-tools/pocket-ic`,
  `RECOVERY_BASELINE_WASM` pointing to `.local/recovery-baseline.wasm` and
  `PHOTOCRAFT_CLOUD_WASM` pointing to the current release Wasm: populated old-to-new
  upgrade, content/grants preservation, restore elsewhere, a second upgrade,
  receipt replay and revoked access passed.
- `python3 scripts/build.py portal --assemble`, backend release build/Candid
  generation, formatting, `git diff --check` and `python3 scripts/verify.py` passed.

Browser checks used the actual portal/backend with a disposable account on a
separate loopback origin: folder expansion and navigation, file upload/details,
rename/move/Trash Undo, restore to root with a new name after the original parent
was trashed, and ET/EN panel translation. Proof:
`.local/cloud-navigation-details.png`. Logs: `.local/navigation-*.log` and
`.local/recovery-upgrade-test.log`. The existing HTML feature review and its served
copy now mark these four capabilities delivered.

Installed locally with `icp build cloud -e local` and code-only
`icp canister install cloud --mode upgrade -e local --identity
subnet-cloud-local-keychain`; the local network was not restarted. Mail status
remained configured. The native certified-assets transport synced final asset hash
`6a3a49ed12e4414f49cdd43daaec188b194651a8e923c6fbc352b94f62ec9e0f`.
Mainnet was not deployed. Email account copy now reads “E-post seotakse sinu subnet
kontoga.” Email anti-abuse limits were retained; clearer cooldown/daily-limit
messages remain recorded follow-up work.

Limits: one most recent action/group per account/tab, no redo or persisted Undo,
no ACL/permanent-delete Undo, and no details for trashed objects. The tree covers
My files; Shared with me retains its existing navigation. No mobile viewport
browser proof was obtained. Activity history, sharing conveniences, previews and
revision-retention policy remain later milestones.

## Email enabled on the current local runtime — 18 October 2026

The user explicitly deferred SEV hosting as a prerequisite and authorized finishing
email on the current environment. Updated the official local launcher to
`v16.0.0-2026-10-08-03-33`; its normal defaults support the standard HTTP v2 adapter.
Tests no longer enable beta features or depend on the custom experimental launcher.
The current backend is configured through its sole Keychain controller and mail
is enabled. No standalone provider key file was reintroduced.

**Local data incident:** stopping the September launcher lost the local canister
state. The filesystem backup was taken after shutdown and proved incomplete; a
portable canister export should have been made before stopping. No recoverable
local copy was found. The user was informed and confirmed that only test data was
stored there. Recreated the local backend and frontend from source/builds, using
the same IDs and port 61304. This recreated the application, not the lost files.
Mainnet and source were untouched. The incomplete backup is retained under
`.local/email-runtime-backup/`. Development instructions no longer claim retaining
the cache directory is a sufficient backup.

Verification messages now link to `/pilveruum/#email`, preserve that destination
through login and explain how to resume the pending code. Failed/expired delivery
receipts no longer offer an unusable retry button. A full sharing integration test
found and fixed re-sharing after revocation: email sharing updates a live direct
grant, but creates a fresh grant after revocation/expiry. It never resurrects the
old grant or makes old invitations grant access again.

Validation: 49 backend unit tests, three actual Wasm/PocketIC email tests, 113
portal/locale/shell/loading tests, six provisioner tests, four localization tests,
and Clippy with warnings denied. Integration coverage includes verified-recipient
file links, pending invitation verification and explicit acceptance, revoked access,
address transfer with old ACL preservation, future sharing to the new address owner,
populated pre-email upgrades, and controller-only key rotation/removal. Served
portal/trust/cache verification passed. Evidence is `.local/email-finish-*.log`.

The ordinary `icp deploy` re-applied environment settings and removed the mail key.
Re-provisioned it from Keychain and verified that the standard code-only
`icp canister install --mode upgrade` preserves it. Development/deployment guides
now use this path for an existing configured backend. The frontend sync needed the
existing native DFINITY adapter after the standard Wasm compressor hit its time
limit, and local test cycles after creating the fresh canister. Final asset state:
`f02e366d4b5989e25c977b36d71d179ccc2011f42b729d7d1ac037f0942f4ff0`.

Browser checks used the actual portal and backend with a disposable test principal
on a separate loopback origin; the user's login storage was untouched. The account
menu, enabled email-entry dialog and authenticated `#email` deep link render
correctly, with no observed console errors; proof:
`.local/email-account-dialog.png`. No verification code was sent from that test
account to the user. Prior authorized real Resend delivery remains verified by the
user's receipt; a further send to the provider's test mailbox was rejected by
automatic approval review because that destination was not authorized. It was not
retried or bypassed. Provider responses in the automated integration suite are
mocked. Bounce webhooks and notification preferences remain a separate later step.

## Subnet email contacts, sharing and owner-managed credentials — 18 October 2026

Implemented optional multi-address contacts within Subnet, without requiring
Internet Identity email attributes. Eight-digit, caller-bound verification codes
prove mailbox control; moving an existing address requires an explicit second
confirmation. Transfers do not move files, old grants or old pending invitations.
Email sharing resolves a current binding to the ordinary principal ACL or creates
a revocable invitation. Delivery receipts and retries remain separate from access.
Stable memory IDs 26–35 extend the existing single-canister architecture.
See the [contract and remaining limits](cloud-drive-roadmap.md#email-sharing--implementation-contract-18-oct-2026).

The portal provides **Minu e-postid**, verification/resume, transfer/removal,
email-or-principal sharing, pending invitations and delivery retry/revoke controls.
The shared Rust facade exposes the typed methods; Craft editor binaries were not
rebuilt for this email-only stage. Main local backend upgrade and frontend sync
succeeded on the existing port-61304 deployment, preserving local canister IDs and
data. Browser inspection found that deployment signed out; no user authentication
or address binding was fabricated to claim a manual end-to-end test.

Validation: 49 backend unit tests; Clippy with warnings denied; 110 JavaScript
locale/portal/shell tests; four Python localization tests; two real PocketIC email
tests, including a populated pre-email Wasm upgrade and controller authorization;
six operator-provisioning tests; and the served portal/trust/cache verification.
Test evidence: `.local/email-service-tests.log`, `email-clippy.log`,
`email-js-tests.log`, `email-upgrade-test.log`, `email-final-verify.log`.

One explicitly authorized real test email was accepted by Resend, and the user
confirmed receiving it. It was a delivery test tied to a test principal/request,
not a verification of the user's account. No address binding was created. The
disposable PocketIC runtime needed the official beta-feature configuration for
HTTP pricing v2; the main local runtime has no live provider key and sending stays
disabled until a compatible runtime is configured. The disposable replica and its
historical checkpoints were removed after the test; the user's main state remains.

The API key is provisioned into the backend's environment variables by the owner's
signed standard ICP management call. `configure-mail.py` supports explicit local
or remote backend targets, rotation, removal and redacted status; no app admin role
or custom secret API was added. The script requires the sole private controller
and controller-only status/snapshot/log visibility. A regression test captures the
CLI's merge semantics: omission during sync cannot remove a key. Removal uses the
explicit remove-variable operation and validates the result. The real local
redacted status and explicit removal commands passed. Remote provisioning was not performed.

The supplied `resend.rtf` was moved into native macOS Keychain and deleted. The local
controller identity also moved into ICP CLI's Keychain storage; its old plaintext
identity and project PEM were removed. Provisioning uses a private system temporary
configuration outside the project and cleans it on success/error. A value-based
scan found no real key in project source, assets, logs or retained replica state
(build caches, tool installs, Git internals and editor checkouts were excluded).
This does not claim production SEV confidentiality or isolation from processes
already authorized as the same macOS user. Provider bounce webhooks, preferences
and a shared cross-project mail service remain deferred.

The HTML feature review now reflects implemented account/mail flows, the successful
delivery check, and the remaining local-runtime/production-configuration work.

## Reliable transfers and organization — 18 October 2026

Implemented the seven next-stage groups from the HTML feature review: folder upload,
per-item upload/copy outcomes and retry/cancel, explicit file collision choices,
hierarchical folder ZIPs and multi-file copy, standard clipboard/internal drag,
reviewed access-changing moves, and new Craft documents in the current folder.
The same single canister, canonical catalog and permission evaluator remain.
See [architecture and limits](cloud-drive-roadmap.md#reliable-transfers-and-organization--implementation-contract-18-oct-2026).

The shared backend move planner compares effective rights and expiry durations,
including redundant grants and agent/sponsor intersections. Preview and mutation
use the same rules; changing access requires the owner even through the older
direct move call. Reviewed mutations bind to a catalog version, and committed
retries still return their receipt. No new stable memory layout is introduced.

The portal's extracted job controller retains each request and resolved destination.
SDK cancellation drains in-flight chunks and aborts uncommitted staging; a committed
result remains completed. Failed/cancelled folder dependencies cannot be recreated
implicitly by their children. The browser clipboard uses standard copy/cut/paste
events, so macOS system commands work. Account changes clear private queued work.
The hierarchical ZIP encoder is checked with Python's independent zipfile reader.

Validation: 37 service unit tests; 17 SDK tests; 10 shared UI tests; 116 combined
JavaScript tests, followed by 97 portal/queue/locale tests after the final dependency
fix; four localization tests; Candid equality; focused Clippy/Wasm checks; the actual
pre-suggestions populated-state PocketIC upgrade; all eight SDK snapshots and release
builds. Logs are `.local/next-*.log`. Final portal-only assembly follows the fleet
assembly, preserving all eight rebuilt packages. No production deployment or Git
publication is part of this work.

Final local sync published 244 assets with state hash
`62231c2e894666ce1f3753b5949c32ba91a759ebfb93f61297f95f4df174b2b1`
on both local networks. `python3 scripts/verify.py all --timings` passed for the
portal, all eight editors, trust/discovery configuration and missing-asset handling
on the user’s port-61304 environment. Logs: `.local/next-main-dependency-sync.log`
and `.local/next-final-verify.log`.

Real local browser checks on the isolated port-51488 canisters: nested folder upload
and per-item outcomes; Cmd+C/V and destination copy; a shared-folder move's access
loss preview; denied access for the former recipient while a retained shared sibling
stays visible; and new-in-folder save with explicit ownership. All eight rebuilt
native editors saved and reopened actual changes: Word text, Grid B1, a ninth Deck
slide, a second Photo stroke, PDF drawing annotation, Sound Kick mute, a second CAD
rectangle, and a new Fig rectangle in Transfer QA. Proof: `.local/drive-browser-qa/
next-*-reopened.png`, `next-new-in-folder.png`, `next-transfer-complete.png`, and
`next-move-access-preview.jpg`.

The all-editor checks are manual native round trips, not an exhaustive automated
editor matrix. Folder-picker empty directories, persistent reload recovery,
recursive folder copy and unified download jobs remain outside this milestone.
The deployment target's SEV properties are not established by localhost tests.
Email UX and supported II verified-attribute/Resend integration are recorded as the
next proposal, not as delivered email functionality. The HTML review remains at
`http://127.0.0.1:61305/` with all 31 feature groups and their remaining differences.

## Suggested Home and familiar file workflows — 18 October 2026

Completed the milestone begun on 17 October. Home derives file/folder suggestions
from the canonical authorized catalog and bounded private viewer summaries, with
truthful reasons and seven-day dismissal. Successful foreground opens are recorded
after portal preview/native decoding; selection, downloads and background reads do
not create a new successful-open signal. Background refresh preserves displayed
file and folder order; explicit refresh reranks and coalesces while already running.

Sharing now has Viewer/Editor presets, Advanced capabilities, versioned direct-role
changes and inherited read-only rows. Restricted file/folder links survive sign-in
and confer no permission. Single-file Make a copy reuses verified download/staged
upload, explicit name/destination, durable retry IDs and quota refresh. All data stays
in the same platform-owned canister, with one catalog and permission evaluator.
Architecture and accepted/deferred scope: [drive roadmap](cloud-drive-roadmap.md).

Validation passed: 36 service unit tests, 16 SDK tests, 10 shared UI tests, 110 combined
JavaScript tests (73 portal tests rerun after final refresh/quota fixes), four Python
localization tests, strict service/facade Clippy, release backend/facade Wasm,
exported Candid comparison, all eight canonical SDK snapshot checks and eight editor
builds. The populated PocketIC test upgrades an actual pre-suggestions schema and
preserves files/chunks, grants, viewer state, pending upload and retry receipts;
new metadata survives another upgrade. Same-Wasm deployment is not that proof.

Real browser flows used disposable accounts on the separate port-51488 QA network:
stable selection geometry/double-click, inert text and image preview, truthful Home
reasons, private dismissal versus Recent, explicit-name copy into a folder, Viewer
copy, Viewer-to-Editor change, revocation, independent copy after source revocation,
and signed-out restricted-link return. ET/EN and list/grid were checked. All eight
current Craft builds saved and reopened native data through the common picker, and
Home confirmed each successful open: Word paragraph, Grid cell, Deck sample slides,
Photo stroke, one-page PDF, Sound's 63.1 MB sample session, CAD rectangle and Fig shape.
Screenshots: `.local/drive-browser-qa/suggestions-*-reopened.jpg` and
`suggestions-home-desktop.jpg`; detailed notes: `.local/suggestions-browser-proof.txt`.

These are manual native round trips, not a complete automated per-editor conflict,
offline and account-switch matrix. The IAB viewport override did not resize the hidden
test tab, so no new 390 px proof is claimed for this milestone. Clipboard delivery
was observed through the UI result; exact restricted URL construction is covered by
unit tests, not a successful system-clipboard readback. Existing editor-wide lint
limitations remain as previously recorded; targeted shared packages pass. SEV-enabled
hosting is expected later and is not established by localhost evidence.

Commands/evidence: `cargo test --manifest-path services/sovereign-cloud/Cargo.toml
--lib --locked`; shared SDK tests; `RUSTUP_TOOLCHAIN=1.95.0 cargo test --manifest-path
packages/sovereign-cloud-ui/Cargo.toml --lib`; Node/Python logs under
`.local/suggestions-*.log`; `python3 scripts/sync-sdk.py all --check`; `python3
scripts/build.py all` (resumed explicit remaining apps); `python3 scripts/assemble.py
all`. The upgrade test uses `SUGGESTIONS_BASELINE_WASM=.local/suggested-home-baseline.wasm`
and the current service Wasm via `PHOTOCRAFT_CLOUD_WASM` (absolute paths), then
`cargo test --manifest-path services/sovereign-cloud/Cargo.toml --test browse
suggested_home_upgrade -- --nocapture`. Rust 1.95.0 is required by the shared UI.

The backend was upgraded on the original local network at port 61304 with its state
preserved. `ICP_CLI_PLUGIN_COMPUTE_LIMIT_SECS=900 icp sync frontend -e local`
published 243 local assets (state hash
`e46775d3e5f3ca9964d9d83172cf61be408ee3ba62a679e2a52f5f5ab7412289`).
`python3 scripts/verify.py all --timings` passed for the portal, all eight editors,
exact payloads, runtime/discovery configuration, caching and missing assets.
QA account-fixture assets were removed by syncing the clean frontend and their
HTML/JS endpoints were confirmed to return 404;
their previous local build remains in ignored QA storage. No source commit, push,
mainnet deployment or user data reset was requested or performed.

The requested feature reassessment is in the roadmap and the standalone
[HTML review](cloud-drive-feature-review.html), with 31 filterable feature groups,
official Google references, next-milestone acceptance scenarios and explicit data
boundaries. Browser checks exercised its stage filters, search and architecture tab.
The owner will provide project-wide email support later; no separate provider was
selected. Broader implementation remains proposed, not authorized by this review.

## Inline folder name editing — 15 October 2026

Editable folders now use the same single-click name editor and double-click opening
as files. Folder names are selected in full, including dots; Enter/blur saves and
Escape cancels. Read-only folder links and picker navigation are preserved. No new
backend operation or component was needed. Validation: 75 Node tests, three Python
localization tests, native browser rename/navigation and local HTTP verification.
Only the local frontend was updated. Proof: `.local/resource-browser/folder-rename.png`.

## Inline filename editing — 15 October 2026

Editable file names now enter inline rename on a single click. Double-click still
opens the file; Enter or blur saves, Escape cancels, and F2 is available from a row.
The basename is selected without the extension. Read-only files, Trash, folders
and editor picker navigation retain their existing behavior. Rename uses the
existing authenticated, version-checked operation and resource progress.

Validation: 73 Node tests and three localization tests passed. Native browser QA
verified single/double click, basename selection, Enter commit and unchanged sibling
rows using isolated in-memory test data. The local frontend was synced and HTTP
verified; production was not deployed. Decision: [resource updates](resource-updates.md).

## Resource-scoped file updates — 15 October 2026

File actions now keep the cloud space visible and usable: pending state belongs to
the affected resource, metadata refresh is silent, and unchanged rows retain their
DOM nodes. Rename, move, Trash/restore/purge, folders and uploads preserve loaded
pages. Uploads and purges refresh storage usage through the existing SDK account
method instead of restoring identity/home. Read ordering and session guards reject
stale results; refresh failures retain the current list unless access was revoked.
Architecture evidence: [resource updates](resource-updates.md).

Validation: 68 Node tests across cloud UI, locale and shared shell/loading, three
Python localization tests, portal Wasm build/assembly and `git diff --check` passed.
New regressions cover pending rename, unchanged menu/focus/rows, loaded pagination,
concurrent success/failure, out-of-order reads, move/Trash/restore, creation,
cross-tab updates, upload progress/quota and account/navigation changes.

Native browser QA at 1280×720 used the actual portal code with an isolated in-memory
fixture: pending rename kept other file actions enabled; completion replaced exactly
one of three rows. Screenshot: `.local/resource-browser/pending-rename.png`.
The fixture does not access user files or claim a live Internet Identity proof.
`python3 scripts/dev.py` synced only the local frontend and `python3 scripts/verify.py`
passed at `http://frontend.local.localhost:61304/`. No new backend method, editor
package or production deployment was required.

## Permanent Trash deletion — 15 October 2026

The portal now offers owner-only **Kustuta jäädavalt / Delete permanently** on
trashed rows and **Tühjenda prügikast / Empty trash** for all owned entries across
all pages. Both require confirmation, preserve file names across language changes,
leave other owners' shared items alone, and refresh quota after success. The shared
service removes nested content, revisions, grants and targeted pending uploads in
one version-checked update with persistent retry receipts. No stable IDs or existing
method contracts changed. Architecture evidence is in [Trash deletion](trash-deletion.md).

Validation: 28 service unit tests, 16 SDK tests, 54 Node locale/shared-shell tests,
three Python localization tests and service Clippy with `-D warnings` passed. The
portal and service Wasm builds passed. The new PocketIC test upgraded the old indexed
storage baseline, purged a full 256 MiB account in one update, verified ownership,
anonymous rejection, zero usage, inaccessible content, subsequent file creation and
retry behavior after another upgrade. `git diff --check` passed.

Commands: `cargo +1.95.0 test --manifest-path services/sovereign-cloud/Cargo.toml --lib --locked`,
SDK `cargo test --locked`, `node --test scripts/cloud-i18n.test.mjs scripts/i18n.test.mjs
scripts/craft-shell.test.mjs scripts/craft-loading.test.mjs`, localization unittest,
`python3 scripts/build.py portal --assemble`, and the service `--test trash` with
`PHOTOCRAFT_CLOUD_WASM`, `PHOTOCRAFT_BASELINE_WASM=.local/drive-pre-index.wasm` and
`POCKET_IC_BIN=/tmp/subnet-trash-tools/pocket-ic` (use absolute Wasm paths).

Native browser QA checked the actual portal code with a separate in-memory visual
fixture at 1280×720 and 390×844: owned/shared controls, individual/all confirmations,
cancel focus, and ET/EN switching. Mutations were disabled in that visual fixture;
actual destructive behavior was verified against disposable PocketIC data, not a
real user's browser session. No production deployment was performed.

Local deployment: `RUSTUP_TOOLCHAIN=1.95.0 icp deploy cloud -e local` passed the
Candid compatibility check and upgraded the existing local backend, followed by
`python3 scripts/dev.py` and `python3 scripts/verify.py`. The portal/shared-site
verifier passed at `http://frontend.local.localhost:61304/`; existing editor packages
were reused. The visual fixture is isolated under `.local/trash-browser/` and is
not part of the deployed assets.

## ICP MCP connection wording — 15 October 2026

The English and Estonian connection guide now presents both access modes and
identifies the browsing restriction as a limitation of Subnet's current API.
The MCP guide explains that metadata reads use consensus updates; folder grants
still restrict what the connected account can change. This is a copy change.

Checked the current ICP skill index and the service-discoverability,
agent-web-identity, internet-identity and canister-security skills linked in
[the setup decision](mcp.md#setup-decision--2026-10-15), plus the official connector
instructions, on 15 October 2026. The existing connector supports both modes;
the documentation gap was presenting only one. No integration extension was needed.

Validation: 50 existing Node locale/shared-shell tests and three Python localization
tests passed; `git diff --check` passed. No deployment was performed.

## Responsive landing images — 15 October 2026

Requirement: keep originals in Git and generate a few image sizes during the build,
especially to reduce the mobile hero download. The old page preloaded the same
2.8 MiB landscape PNG and 276 KiB transparent cloud for every viewport; a 32-pixel
footer logo referenced an 812 KiB PNG.

Architecture evidence, retrieved 15 October 2026:
[static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
and [icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md).
The supported approach is already in use: `@dfinity/static-site@v0.4.0` runs the
Python assembly build and serves certified static files. No hosting gap was found.
The observed gap is in our assembly: it copied originals without encoding variants
or adding responsive markup. The smallest extension is Pillow in that existing
build, native `srcset`/`sizes`, matching responsive preloads and content-hashed
WebP URLs. Generated images use a separate `/images/` cache rule so the mutable
`/assets/*` rule cannot conflict. No new canister, runtime image API or CDN is needed.

Three widths balance payload and build complexity; the browser also considers
pixel density. A single 96-pixel footer mark covers its 32-pixel display size up
to 3x density. WebP preserves the cloud's alpha channel. Extra AVIF/fallback sets
are deferred: the site's modern browser baseline supports WebP. Originals remain
unchanged and the campaign's explicit download still returns its original PNG.
References: [responsive images](https://developer.mozilla.org/en-US/docs/Web/HTML/Guides/Responsive_images)
and [Pillow's WebP encoder](https://pillow.readthedocs.io/en/stable/handbook/image-file-formats.html#webp).

Proven locally: four image-build tests cover dimensions, preserved transparency,
no upscaling, deterministic URLs/cache invalidation, matching preloads in both
locales, original bytes, missing inputs and required layout hints. Three existing
localization tests and ten scoped-workflow tests also pass. `python3
scripts/assemble.py` succeeds, reusing all seven packaged editors. `git diff
--check` passes and Git originals are unchanged.

Browser QA at 390, 768 and 1440 CSS pixels loaded the 640-, 1280- and 1672-pixel
landscapes respectively, with no horizontal overflow or console errors. Both
languages and the lower banner/footer were visually checked. A 390-pixel iframe
at the browser's native 2x density loaded the 1280-pixel landscape (315,788 bytes)
and 640-pixel cloud (24,966 bytes), each exactly once via the matching preload.
At 1x the landscape is 88,472 bytes versus the original 2,885,880: about 97% less;
at 2x it is about 89% less. The 32-pixel footer logo uses a 96-pixel WebP of 5,008
bytes versus 831,844 (99.4% less). The shared subnet PNG falls to 24,070 bytes.

Review: the build uses one encoder dependency and the existing assembly/hosting;
there is no runtime sizing code or new service to maintain. Original campaign
downloads are preserved. Local static-preview tests establish selection and byte
savings, not production load time or live certified cache headers. The extended
`verify.py` checks those headers after deployment. No canister sync or mainnet
publication was performed for this change.

Local rollout on 15 October 2026: ran `python3 scripts/dev.py` at the user's
request, followed by `python3 scripts/verify.py`. The local canister serves all
13 generated WebP files with correct payloads, media types, certification and
non-conflicting immutable cache headers. Portal routes, shared assets, runtime
trust and missing-asset checks passed. The deployed landing page also loaded its
generated hero images in the browser without console errors. Local URL:
`http://frontend.local.localhost:61304/`. Mainnet publication remains deferred.

## Homepage AI workflow illustration — 14 October 2026

Requirement: adapt the supplied `subnet_agent_workflow_et_v2/index.html` and
campaign image into the homepage between the tools and technology sections.
The tools heading now refers to the cloud drive; the new section demonstrates
reading a brief, understanding it, creating an image, saving it and opening it.

Guidance retrieved on 14 October 2026:
[static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
and [icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md),
selected from the current skill index. The existing `@dfinity/static-site@v0.4.0`
recipe and portal assembly already support the required HTML, CSS, JavaScript
and local image assets. No platform gap was observed. The smallest extension is
a scoped, dependency-free browser illustration using the existing translation
pipeline and brand mark. A separate iframe would complicate language switching
and responsive sizing; a live agent integration is outside this visual task.

The workflow has an accessible animation label and makes no backend or AI calls. It
supports pause/play, selecting steps, opening/closing the image preview, and
stops advancing outside the viewport or in a hidden tab. Reduced motion starts
with the finished image; the static HTML also shows that outcome without scripts.
Copy is provisional. The image retains the supplied artwork and English lettering.
Proof checks cover localized assembly, existing language tests and browser flows;
this illustration makes no claims about real agent authorization or persistence.

Validation: 45 existing JavaScript tests passed with `node --test
scripts/i18n.test.mjs scripts/cloud-i18n.test.mjs scripts/craft-shell.test.mjs
scripts/craft-loading.test.mjs`; the 3 localization and 9 scoped-workflow Python
checks passed. A disposable Node/VM smoke check covered elapsed playback, pause,
manual preview, reduced motion and viewport/tab suspension. Browser checks at
1280, 390 and 320 px covered layout, image loading, preview open/back, step
selection and a language switch preserving the selected stage; no console errors
were observed. Small viewports use a single seekable progress bar. Assembly
keeps all seven existing editor bundles and includes both localized homepages.

The attempted local `scripts/dev.py` sync was stopped during a long-running asset
sync; final certified-canister HTTP verification was not completed. Final visual
QA used the assembled static site. No mainnet publication was performed.

Copy refinement: the heading is followed by one sentence about MCP access and
collaboration in the user's sovereign cloud drive. The agent subtitle identifies
the MCP connection without listing a fixed set of capabilities. Removed the
eyebrow, example badge, duplicate cloud badge, preview caption and image footer;
the image uses the freed space. Updated both languages, removed unused styles
and translations, rebuilt both locales, passed the three localization checks and
verified the heading and open image in the local browser preview.

The demo file list now follows the existing drive's rows: filename, size/date,
`Ava` and the `···` menu. Removed the invented folder heading and table labels.
Open/Enter/double-click and the read actions in the menu work on the two local
sample files. The cursor follows the actual filename bounds, including wrapped
names, and shows a double-click before opening the image. Its motion uses the
same playback clock, so pause and resizing preserve the click target. The real
drive and its authenticated operations are unchanged.
Validated filename targeting at 1280 and 320 px (including a wrapped name),
double-click, Enter, both previews, menu opening/Escape and English metadata/actions.
All 27 existing i18n/drive JavaScript tests and 3 localization checks passed;
the assembled preview reported no browser errors.

Responsive layout: tablet windows remain side by side, with shorter prompt copy
and secondary descriptions/checklists hidden. On phones the compact agent panel
sits above a 260 px cloud panel, keeping both visible within a 478–495 px demo.
A single accessible seek slider replaces the six stage controls below 1100 px;
the current activity stays in the agent window. The slider pauses on seeking and
clamps its final position so the finished preview remains valid. Desktop retains
its original copy and stage buttons. Checked 320×568, 390×844, 768×1024,
1024×768 and 1280×960 layouts, both languages, slider endpoints/intermediate
steps, preview/back and cursor targeting; no horizontal overflow or browser
errors were observed. Localized assembly and the three localization tests pass.

## Public demo and custom-domain source publication — 13 October 2026

Committed the deployed custom-domain configuration, canonical cloud SDK and its
seven editor snapshots, public-demo copy and roadmap, workspace attribution and
EFTI footer mark. The SDK preserves the canister identity on the custom domain
and uses the certified public gateway settings.

Publication checks passed: 32 portal JavaScript tests, 16 Python workflow tests,
13 SDK tests, native SDK Clippy, the browser/Wasm SDK check, all seven snapshot
integrity checks, and secret scanning of the changed files. The custom-domain
HTTP verifier passed for the portal, shared assets, configuration and editor entry
pages. This run did not download every editor's Wasm bundle or repeat sign-in.
Full editor CI passed formatting but stopped at Clippy errors in unchanged
upstream crates; it is not a fully green editor-wide CI result.

## Archive: 10–13 October 2026

The notes below were moved from the project README on 13 October 2026.
They preserve the sequence of integration milestones, earlier design details and
validation results. References to four or five editors, branch locations, CI
failures and deployment state describe those milestones, not the current checkout.
Use the development and deployment guides for current working instructions.

### Initial local validation — 10 October 2026

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

WordCraft's media-preserving JSON codec is used only by its cloud adapter; ordinary
local JSON open/save remains upstream-owned. SoundCraft's portable bundle is likewise
a separate cloud format, without replacing upstream local session saves.

Browser checks on the isolated local QA origin saved and reopened a DeckCraft sample,
a WordCraft document, a blank PdfCraft document and SoundCraft's synthesized demo
(63.1 MB including its audio). Each appeared in the shared catalog with the matching
editor link, while in-app pickers only listed compatible formats. WordCraft's second
save kept the same file ID and produced revision 2; cancelling replacement of unsaved
text kept the edits, and successful save cleared the shared navigation warning.
These use disposable test identities and real calls to the local replica, not real
Internet Identity/passkey issuance or mainnet.

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
see [the update guide](../scripts/UPDATING.md) before running an unfamiliar build.

### CADCraft and GridCraft cloud adapters

The catalog now includes CADCraft (drawings) and GridCraft (spreadsheets). Both use
our shared shell, File → Sovereign Cloud menu, account and versioned file service.
Cloud Save / Save As / Open use upstream's DXF codec (`cadcraft.dxf`, version 1)
and complete JSON workbook codec (`gridcraft.scjson`, version 1), respectively.
Ordinary local saves and exports remain upstream-owned. Cloud files reopen in new
tabs, preserving existing work. GridCraft commits/validates a cell still being typed
before requesting a cloud save. An upload only marks its exact document revision
clean; newer edits and unfinished text remain protected by the navigation guard.

The fork integration is confined to the browser host and an optional File-menu
callback; the storage protocol and UI are generated vendor snapshots. Both apps
are included in `scripts/update-crafts.py`. Rebuild these two apps with
`python3 scripts/build.py cadcraft gridcraft --assemble`; other editor outputs are reused.

## Seven-editor upstream refresh — 12 October 2026

Upstream refresh (2026-10-12): the fork merges retain both upstream history and the
existing subnet integration. Reviewed upstream tips were PhotoCraft `bb17dd555c16`,
PdfCraft `87b448b664e1`, WordCraft `fc52079f587a`, DeckCraft `0ec0cc5e815c`,
SoundCraft `df0c4790b729`, CADCraft `4cb9e8a535b2` and GridCraft `b3acae6b7782`.
GridCraft's cloud actions now live in its new Backstage view. WordCraft uses upstream's
document identity API; SoundCraft keeps its portable cloud format alongside upstream's
new local file handling. CADCraft and PdfCraft retain their new upstream host bridges.
All seven forks retain the same pinned Sovereign Cloud SDK and UI source snapshots.
All seven release Wasm builds pass. The 25 focused tests pass: PhotoCraft web-host/menu
tests (13), WordCraft native-format/document identity (3), SoundCraft portable audio
format (5), CADCraft cloud adapter (1), and GridCraft cloud adapter/Backstage wiring (3).
All seven browser cloud save/open checks pass using a disposable identity on the local
test network, including edited image, text, spreadsheet, CAD and PDF content and the
slide/audio demos. No browser console errors were observed. Temporary session fixtures
and their generated redirects were removed; `python3 scripts/verify.py all` passes for
the final local bundle, including exact asset payloads, certification, cache/CSP headers,
runtime trust and missing-file responses. These checks do not cover Internet Identity
issuance or constitute a deployment of this upstream refresh to mainnet.

Strict editor-wide CI is not green on Rust 1.95.0: it stops at upstream Clippy warnings
in CADCraft `geom/nurbs.rs`, GridCraft's XLSX code/tests, DeckCraft's ISOBMFF codec,
WordCraft `proof/affix.rs`, SoundCraft's video codec and PdfCraft `forms/af.rs`.
PhotoCraft's scoped Wasm Clippy check likewise stops at `drop_non_drop` in upstream
`format/store.rs` when checking the browser target.
These files are unchanged by the cloud integration; the refresh does not suppress
their lints or modify unrelated editor behavior. PhotoCraft's 29-crate layer check passes.

## Mainnet trial — 12 October 2026

The mainnet trial uses two new canisters, separate from all local data and existing
standalone Craft deployments:

* Frontend: `vigkv-fqaaa-aaaac-bgeha-cai`, https://vigkv-fqaaa-aaaac-bgeha-cai.icp.net/
* Accounts/files: `vbfbj-tyaaa-aaaac-bgegq-cai`.

Both are controlled by the existing `efti-nns` identity. Mainnet mappings are in
`.icp/data/mappings/ic.ids.json`; retain them so later updates reuse these canisters.
This trial does not configure the `subnet.ee` custom domain.

The initial mainnet verification passed for all seven editors, including payload
hashes, certified responses, runtime canister IDs/root key, cache headers and 404s.
PdfCraft and SoundCraft also started in the browser. Initial gzip Wasm transfers
were 6.1–12.7 MB and took 3.6–24.8 seconds in one run; these are variable HTTP
measurements, not complete browser startup times. A separate WordCraft Brotli
request transferred 7.4 MB in 7.3 seconds; its conditional repeat returned 304
with no body. The landing page loads no editor Wasm.

## Branch milestones recorded before this split

The current portal and all seven app integrations are published on each fork's `main`.
Earlier integration milestones remain on `codex/shared-account-files` in the root and
PhotoCraft, and `codex/sovereign-cloud` in the other six forks.
The root remote is `https://github.com/eerikpotter/subnet.ee.git`.
The first extraction remains on `codex/subnet-cloud-base`.

## Future work recorded at the time

All seven editors shared cloud storage and compatible format metadata. Shared
spaces, per-app grants, user-owned canisters, encrypted storage, alternate blob
stores and Cloud Engine migration remained later work.

## Homepage AI indicator — 14 October 2026

Published the approved `+ AI` badge and person-with-spark icon beside “Sinu
pilvefailid” using `icp sync frontend -e ic --identity efti-nns`. The release reused
the tested preview bundle: the Estonian and English homepages and `landing.css`
differed from production only by this visual change. The backend was not deployed.
The frontend reported 169 assets and state hash
`c3d736b903f96024ceeb0a9da608c26e5c9ba7b19e3d09cb8786b19cfc17263b`.

`python3 scripts/verify.py portal --environment ic --origin https://www.subnet.ee`
passed exact payload, certification, runtime identity, MCP discovery, cache/CSP
and missing-path checks. The badge was also verified visually at the public origin.


## 2026-10-14 — Shared cloud folders in all seven Craft apps

Open and Save As now embed the same portal drive, with personal/shared folders and
editor-appropriate light/dark styling. Save uses the canonical drive revision;
Create and Edit remain independent permissions. Read-only files open normally and
can be copied where Create is granted. Account changes, permission failures and
version conflicts preserve drafts. Native formats and editor engines are unchanged.
A WordCraft cold-start font race found during direct-file testing was fixed in the
shared UI, with a regression test.

Scoped SDK/UI/API checks and all seven release builds passed. Disposable local
accounts exercised shared native save/reopen in WordCraft, PhotoCraft, PdfCraft,
DeckCraft, SoundCraft (63.1 MB with audio), CADCraft and GridCraft, plus shared edits,
read-only Save rejection, Save As, account switching and quota-error recovery.
Mainnet frontend state is
`c1547dc5c8725b8decb7526eb06bfe59e4568deb26012b434e7286b15f15c99f`.
All seven published bundles passed exact-payload and deployment-configuration
verification. WordCraft startup and signed-out Open were also checked in the
production browser. Existing published landing pages were preserved; another
chat's homepage demo was excluded. No backend deployment or storage migration.
See the full evidence in [collaboration progress](collaboration-progress.md).

## Shared cloud MVP publication — 15 October 2026

Prepared all accumulated shared-drive/backend/SDK/Craft changes and the illustrative
landing-page agent workflow for publication together. The narrowed demo scope and
known limits are recorded in collaboration-spec.md and collaboration-progress.md.
Unfinished-upload administration was removed from the ordinary drive; automatic
background cleanup remains deferred.

The review's 126 checks passed, including seven actual-canister persistence/migration
tests; final portal edits passed 27 JavaScript and three localization checks. Before
publication, all 50 portal/shell JavaScript tests and 19 Python tooling tests passed
again, as did strict cloud UI/backend Wasm Clippy and all seven SDK snapshot checks.
The editor JS/Wasm bytes match the seven-app native browser-tested release.

The six app-wide `xtask ci` runs stop at the same documented unchanged upstream
Clippy warnings (Word proof, Deck codecs, Sound video, CAD geometry, Grid XLSX, PDF
forms). Their fmt stages pass; PhotoCraft's layering check passes. This release
does not claim fully green editor-wide CI or suppress those upstream lints.

Publication completed: source commit `210bcbf8f1c474917db5ecff63a8f186c66e5d5a`
and all seven Craft commits are on their GitHub `main` branches. The complete
172-asset site is live at `https://www.subnet.ee/`, frontend state
`06871feaac2acb1839a6451a8b38da6ef361267e295d3300ca8ba7e601499000`.
The existing backend module matches the tested build; no backend upgrade was needed.
Full mainnet verification of the portal and all seven editors passed, as did the
homepage demonstration and signed-out drive browser checks. See
[publication evidence](collaboration-progress.md#complete-github-and-mainnet-publication--15-october-2026).


## 2026-10-15 — User-controlled cached Craft versions (local)

All seven editors share a browser package cache and explicit update dialog. Each
package has verified asset sizes/hashes and a content-derived build ID. Cached A
continues to open after A is removed from the server; downloading B selects it for
future launches without replacing the current editor. Restart checks the live
draft state, including again after asynchronous cache reads. Live-tab leases
protect older packages during cleanup. Missing/evicted caches fetch the current
server package. The server retains only one package per app.

Release notes come only from matching published ArtCraft GitHub releases, with an
inert-text excerpt and source link. The refresh command checked all seven current
versions and found no exact published tag matches, so the initial snapshot is empty.
No subnet-authored notes or unadopted upstream changes are shown. Notes-only changes
do not change the package identity.

Validation: 66 JavaScript tests and 25 Python tooling tests passed (Python via the
existing `.tools/images/bin/python` environment). Assembly passed without rebuilding
any native editor. Native browser smoke checks reached ready for all seven apps
from virtual cached URLs with no console errors; the English and Estonian version
dialogs were checked. WordCraft exercised A-to-B switching after physical A asset
removal, preservation of a second tab's unsaved document, rejection of a restart
with a dirty document, and latest-package recovery after clearing Cache Storage.
The A/B/C versions used for these scenarios are disposable test fixtures, not new
ArtCraft releases. Proof is under `.local/craft-cache-qa/`.

The regular local Wasm sync plugin was stopped after over six minutes without
progress output. The existing native adapter around DFINITY v0.4.0 sync-core
completed the local frontend sync: 202 assets, state
`4cc8bf2707dcdea3664839f69897a0aec88dd88b0ea0bfa6c5b9e0476d3cdfc0`.
Full local `python3 scripts/verify.py all` passed exact asset bytes, certification,
CSP, runtime identity configuration, immutable/mutable cache rules and missing-path
checks. WordCraft also reached ready from a cached URL on the actual local ICP
origin and opened its normal signed-out cloud dialog. This does not repeat the
previous release's authenticated seven-app collaboration matrix or establish
compatibility with future upstream format changes.

No new deployment tool or dependency was added. Backend data/schema and app engines
are unchanged. Publication followed in the combined release recorded below.


## 2026-10-15 — Version popup refinement and combined publication

Removed the misleading Keep current footer action. The shared version popup now
closes on a click outside its bounds, Escape, or the accessible corner close button.
Clicks on the popup's own padding keep it open. Only available-update actions remain
in the footer. Verified all three dismissal paths in native GridCraft on the local
ICP frontend; the popup opens without footer controls when the app is current.

The combined release includes responsive landing images, the shared cached-version
launcher, ArtCraft-only release notes and this popup refinement. All 66 JavaScript
tests and 25 Python tooling tests passed again. The seven pinned app source commits
and editor engine binaries remain unchanged. No backend upgrade is required.
Local frontend state after the popup change:
`79cf05b79c6b219e85ac1bc97ce45076e8cda60b2098ed09184962e9139d3b56`.

Published source commit `ba0e063e722a6ec05f14de30d446338af5f08c37` to
`origin/main` and deployed the complete frontend to mainnet with the existing
DFINITY certified-assets sync adapter. The frontend now contains 203 assets;
production state hash:
`ecf088800f24b6bc77e003287e5a03ecfa1439b91d43d279917fb489fe569c17`.
The backend and its data were not upgraded.

`python3 scripts/verify.py all --environment ic --origin https://www.subnet.ee --timings`
passed for the shared site and all seven editor packages, including payloads,
release metadata, cache headers, runtime configuration, certification and missing
asset handling. Native browser checks on mainnet confirmed the homepage's responsive
WebP images loaded, GridCraft reached ready from its cached package, and its version
popup closed on an outside click with no Keep current action. Both pages reported
no console errors. Local checks additionally covered Escape, the close button and
clicks inside popup padding. These checks do not repeat the prior authenticated
collaboration matrix because this release changes neither app engines nor the cloud
backend.

## 2026-10-15 — Optional AI setup in the cloud drive menu (local)

Moved Work with AI from the footer of every drive listing to its own `#ai` view
in the left menu. The page presents a recommended separate Internet Identity with
ordinary folder sharing and an alternative using the owner's existing identity
with its full account permissions. Expandable instructions explain signing in as
the agent, copying its Subnet principal, switching back to the owner and granting
Read/Create/Edit/Delete as needed. The current account's principal can be copied
directly from setup. Copy is disabled during session restoration or stale identity
state. Both languages include inheritance, revocation and account-attribution
boundaries. See [MCP setup](mcp.md) for the skill evidence and decision.

Validation: `node --test scripts/cloud-i18n.test.mjs scripts/i18n.test.mjs
scripts/craft-shell.test.mjs scripts/craft-loading.test.mjs` passed 50 tests;
`python3 -m unittest discover -s scripts -p test_localize.py` passed 3 tests.
`python3 scripts/assemble.py` passed, reusing all seven editor packages;
`git diff --check` passed. Browser checks on the assembled localhost site confirmed
the separate menu view, exclusion from Shared and Trash, expandable steps, language
switching with the route and expansion preserved, and responsive layouts at desktop,
768px and 390px widths without horizontal overflow. The browser reported no console
errors. Screenshots are under `.local/ai-setup-qa/`.

No backend or authentication protocol was changed, and no mainnet deployment or
Git push was performed. Hosted MCP acceptance with two real Internet Identities,
principal comparison and shared access/revocation remains an interactive check;
the automated account-change checks use fixtures.

## 2026-10-15 — Cloud space terminology (local)

English product copy now uses **cloud space**, with **your own sovereign cloud
space** in the fuller positioning. Estonian remains **pilveruum**. Updated portal
navigation, titles, metadata, homepage copy, AI setup and shared picker text, plus
the canonical Rust dialog labels and the terminology guidance in development.md.
Existing Sovereign Cloud menu references still match the editors' actual menu.
Canonical Rust label changes await deliberate SDK adoption and editor rebuilds;
this portal assembly reuses the existing seven editor packages.

Validation: the 50 JavaScript checks and 3 static localization checks passed,
assembly and `git diff --check` passed, and the local browser confirmed the new
English sidebar, breadcrumb, account label, title and description. No Cloud drive
wording remains in portal or canonical shared UI source. Preview:
`.local/ai-setup-qa/cloud-space-en.jpg`. No deployment or Git push was performed.

## 2026-10-15 — Complete AI setup and cloud space release preparation

Adopted the canonical cloud UI snapshot in all seven editor forks and rebuilt
every production web bundle. The landing page, its animated workflow, buttons,
instructions and FAQ use cloud space; a refreshed browser check and a scan of
portal source and assembled text found no remaining cloud drive wording. Each
compiled editor contains Cloud space and no Cloud drive string. Estonian remains
pilveruum. Editor source commits are pinned by this repository's submodules.

The shared UI's 10 library tests and strict Wasm Clippy check passed, alongside
the 50 portal JavaScript tests and 3 localization tests recorded above. All seven
release builds passed and all SDK snapshots match canonical sources. Each app's
`cargo xtask ci` passed formatting but stopped at existing Rust 1.95 Clippy
warnings in unchanged engine code: PhotoCraft affinity, PdfCraft forms, WordCraft
proof/docbin, DeckCraft matroska/isobmff, SoundCraft video, CADCraft geom and
GridCraft xlsx. The complete editor CI suites are therefore not green; these
snapshot commits change only dialog strings and their integrity manifests.
Logs and commit receipts are under `.local/ai-cloud-space-*`.

The mainnet preflight state matches the previously published frontend:
`ecf088800f24b6bc77e003287e5a03ecfa1439b91d43d279917fb489fe569c17`.
Publication and production verification are recorded below when complete.

## 2026-10-15 — AI setup and cloud space published to GitHub and mainnet

Published `16281fc8067cb5b6737a37c8b24f48153a5e97c8` to the root
`origin/main`, after pushing all seven editor snapshot commits to their respective
`origin/main` branches. The complete assembled release pins clean editor commits.
The existing DFINITY v0.4.0 certified-assets sync adapter updated 47 assets,
retained 156 current assets and finalized all 203 assets on the existing frontend.
The backend, identity settings and data were not upgraded. An independent
consensus-backed `state_hash` call confirmed the reported production state:
`45d80386895e353584632b0f08af8ff7162dd0ccbd6b4c4a12624aab1452fdf3`.

`python3 scripts/verify.py all --environment ic --origin https://www.subnet.ee --timings`
passed for the shared site and every editor: exact deployed bytes, release
metadata, certification, identity configuration, cache rules and missing-asset
handling. Browser checks on mainnet confirmed cloud space wording across the
landing page, both AI setup options, preserved expanded steps when switching
languages, and AI setup hidden in Shared and Trash. GridCraft reached ready on
mainnet and opened its signed-out Cloud space dialog. The checked pages reported
no console errors. Screenshots are under `.local/ai-setup-qa/mainnet-*`; deployment,
verification and state receipts are under `.local/ai-cloud-space-*`.

The full editor CI limitations recorded above remain. This release does not repeat
authenticated sharing or two-identity hosted MCP acceptance; it changes frontend
guidance, navigation and dialog wording. Cached editor versions remain under user
control and can adopt the new packages through the existing version menu.

## 2026-10-15 — Add FigCraft with the shared cloud integration

Added the `eerikpotter/figcraft` fork as the eighth Craft, with Estonian/English
catalog copy, original interface-design artwork, shared navigation and package
caching. Registered `figcraft.figcraft` version 1 in the shared SDK and backend.
The new optional web adapter saves native archives including images, versions
and branches, preserves existing tabs on open, and keeps later edits dirty when
an earlier upload finishes. The editor engine and native format remain unchanged;
the only general editor UI extension is a small File-menu host callback.

The decision and detailed validation record is [FigCraft integration](figcraft-integration.md).
Native adapter/menu tests, SDK/UI/service tests, four PocketIC migration and
permission tests, portal tests and release builds passed. FigCraft formatting,
layers, assets and standalone Wasm/plugin checks passed. Full upstream CI stops
on existing Clippy errors in the unchanged `.fig` exporter. The cloud service was
upgraded only in the local environment to accept the added format.

Browser checks verified cloud save/reopen of a synthetic design, clean saved
state, preserved tabs, account isolation, both catalog languages and the native
File menu in light mode. A scoped use of FigCraft's existing dark-popover styling
keeps the cloud menu readable. The standard version updater adopted the final
bundle, and `verify.py figcraft portal --timings` passed against the local canister.
Temporary login fixtures were removed and the disposable session was signed out.
Proof and logs are under `.local/figcraft-qa/` and `.local/figcraft-*.log`.

The fork commit `c49c108f09dec11d2223b15347e95fd81673432d` was pushed to
`eerikpotter/figcraft` on `main` after the user authorized continuation; the remote
SHA was verified. The other seven editor commits and binaries were reused.
The subnet integration is committed locally. No production deployment was
performed. Existing unrelated workspace changes remain untouched.

## 2026-10-15 — Refine the cloud space workspace

Removed the workspace's bottom promotional section, footer slogans and ICP sidebar
label. The header now holds the subnet identity and account actions. Navigation
separates files/shared work from applications/AI, with Trash and language/home
utilities below. Mobile uses one header row and an explicit navigation panel;
Escape, outside interaction, keyboard focus and route selection dismiss it.

The app launcher reuses the landing page's original icon language in compact cards:
four columns on a wide desktop, horizontal cards on intermediate screens and two
compact columns on phones. Eight tools fit the tested 390 × 844 phone viewport.
The signed-out drive offers sign-in and application browsing; file actions appear
with a signed-in account. Account-ID copying now lives in the account menu, and
the sharing instructions identify its new location. Existing AI connection copy
edits in the working tree are preserved.

Architecture review: the requirement is frontend navigation/layout, with no new
identity, storage or transport behavior. The session's
[static-site skill](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
(retrieved 2026-10-15) supports the existing certified-assets hosting unchanged.
There is no platform gap. The smallest addition is a lightweight workspace
navigation controller, independent of the cloud client's startup. Existing
session, permissions, file operations and picker protocol are reused. No Craft
source or binaries, canonical SDK, backend or stable schema changed.

Validation: 36 JavaScript locale/drive/navigation tests and 13 Python localization
and scoped-workflow tests passed. Assembly reused all eight packaged editors.
`python3 scripts/verify.py portal --timings` passed the local served-byte,
certification, configuration and cache checks, including the new controller and
SVG sprite. Native browser checks covered desktop, 390-pixel and 320-pixel views,
ET/EN switching, navigation dismissal, signed-in/out states, the account menu and
an actual FigCraft picker open after switching its language. The narrow-screen
check caught and fixed account-menu positioning. Test sessions used the isolated
local canister hostname; temporary fixture pages were removed from the final sync.
Screenshots and logs are in `.local/workspace-ux-qa/` and `.local/workspace-ux-*`.
This is a local frontend update; no production deployment or Git push was made.

## 2026-10-15 — Craft artwork and quieter application cards

Requirement: remove the "LOO MIDAGI UUT" eyebrow, keep Alpha and the ArtCraft
provenance together, and give the compact application cards more of the original
Craft character. The redundant "Choose a tool" instruction is also removed.
The attribution text itself now links to ArtCraft, alongside
the subnet cloud credit; there is no detached "About the project" link.

The cards use eight unmodified upstream app-icon PNG exports, with the existing
small tool glyphs next to their categories. Source revisions and original MIT
license notices are preserved in `portal/assets/crafts/`. The 256-pixel exports
cover the 48–72 CSS-pixel display sizes through 3× density and total 240,078 bytes;
no editor checkout is read during portal assembly and no Craft source is changed.

Architecture evidence: the [static-site skill](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md),
retrieved 15 October 2026, supports the existing certified static-asset hosting.
There is no platform gap. This is a portal markup, CSS and local-image change;
an image service, new dependency or larger app banners would add complexity or
scrolling without meeting an additional requirement.

Validation: three localization and ten scoped-workflow tests passed. Portal
assembly reused all eight packaged editors. Local certified-asset sync and
`verify.py portal --timings` passed, including the eight app-image payloads.
Browser checks at 320, 390, 1063 and 1280 CSS pixels showed all images loaded,
both locales translated, and no horizontal overflow or browser warnings/errors.
The 1280 × 720 view fits all eight cards and their provenance. Screenshots are
in `.local/workspace-ux-qa/art-apps-*`; sync/verification logs use
`.local/workspace-art-*`. No production deployment or Git push was performed.

## 2026-10-15 — Refresh control and uploads in the file area

Requirement: make My files and Shared with me equally restrained on desktop and
mobile, keep manual refresh as a quiet fallback, and accept dropped files in the
existing content surface rather than a separate upload zone. The toolbar now uses
an accessible refresh icon, and the file panel contains a small drag hint. Only
an active file drag reveals the full-panel upload cue and the target folder name.
Touch users retain the upload button; coarse pointers do not show a drag hint.

Architecture evidence: the [static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
and [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
skills, consulted from the session's 15 October 2026 retrieval. Existing certified
static hosting, CloudPortal upload/checkpoint logic and server-side capability
checks meet the requirements. The gap was the portal interaction: native browser
file-drop events now call the existing upload function, with the same limits,
progress and destination snapshot. No new upload service, dependency or Craft
fork change is required. Folder-tree uploads are explicitly rejected rather than
silently importing only part of a drop.

Only an authenticated, validated writable folder accepts a drop, including shared
Create-only folders. Shared overview, trash, read-only folders, editor pickers,
pending navigation and changed sessions cannot receive uploads. Text/link drags
are left alone; file drops outside the panel cannot navigate the browser away.
Nested drag events, cancellation, navigation and focus loss clear the overlay.

Background refresh retains the visible rows and all loaded pages and avoids
restarting identity restoration. Identical responses preserve DOM focus. Foreground
refresh is deferred briefly so it does not consume the click that reactivates a
window. These changes are integrated with the concurrent work on per-file
operations and incremental row rendering; those changes have their own validation.

Validation: 71 Node locale, drive, shared-shell/loading and workspace tests pass,
including drop permission boundaries, multiple files, nested drag events, directory
rejection, unchanged/paginated refresh, late responses and the foreground-click
race. Three Python localization and ten scoped-workflow tests pass. The portal
runtime builds in release mode; assembly reuses all eight editor packages.
The final local certified-asset sync and `verify.py portal --timings` pass.

Browser QA used an isolated disposable local account: multi-file drops succeeded
in the root and an open subfolder, including after integrating per-file updates;
the quota and list reflected the committed uploads. Shared overview does not offer
an upload target. Desktop and 390/320-pixel layouts and both languages were checked
without horizontal overflow or browser errors. Test pages were removed from the
final sync, the test session was signed out and temporary tabs were closed. Logs
and screenshots are in `.local/file-area-qa/`. No production deployment or Git push.

## 2026-10-16 — Drive UX mainnet release

User-authorized publication of permanent Trash deletion (one entry or all owned
entries), resource-scoped progress and incremental rows, and single-click inline
renaming/double-click opening for editable files and folders. The assembled portal
also publishes the previously validated workspace navigation, Craft artwork and
file-area refinements described above. Existing editor packages were reused.

The release follows the session's ICP CLI, static-site, canister-security and
stable-memory guidance recorded in the feature specifications. The cloud canister
was stopped briefly, snapshotted, upgraded in place with the normal Candid check,
and restarted. Snapshot: `0000000000000002000000000041310d0101` (214,459,446 bytes).
Earlier snapshots were retained. Running backend SHA-256:
`0a62f6a6a8216b4298a53f1266284adb9880281a65f2fe9de70eaf125b947089`.
Published Candid exactly matches the source and `get_api_doc` includes the new
`drive_purge_trash` method. Both canisters are running.

The standard v0.4.0 sync plugin reached its 60-second compute limit during Brotli
compression, before committing assets. The existing native adapter around the
same unchanged DFINITY sync-core and canonical compressors completed the sync:
55 assets encoded, 187 reused, 242 total. Finalized frontend state, separately
read back after publication:
`e88e96d921346118d9b04b03633ef1fb2221b03f100e37ce1e8669a9f5c8741d`.

Release checks: 28 backend unit tests and 79 JavaScript tests passed before
publication. The earlier local 256 MiB upgrade/purge/re-upgrade and authenticated
browser proofs remain the functional validation. `verify.py portal --environment
ic` passed on both the certified canister gateway and `https://www.subnet.ee`:
exact portal and shared-shell bytes (including the portal Wasm), certification
headers, CSP, cache rules, runtime identity settings, discovery and missing paths.
Production browser smoke checks passed in ET/EN with no observed console warnings
or errors. Production user files were not mutated as a test. Editor engines were
not rebuilt or downloaded again for this portal release.

The candidate Wasm, asset hashes, snapshot receipt, before/after status, deployment
and verification logs, test output and browser screenshot are retained under
`.local/releases/2026-10-15-drive-ux/` (preparation began before local midnight).
The source remains in the working tree; no Git commit or push was requested.

## 2026-10-17 — Familiar cloud drive browsing and selection

Implemented the first drive milestone in the canonical Rust backend/SDK/facade
and shared portal. Authenticated browse queries cover folders, Shared, Recent,
Starred, Trash and metadata search, with complete authorized sorting before
pagination. Private stars, recent opens and view preferences use new stable-memory
regions; existing file records, transfers, permission evaluation and editor formats
remain canonical. The session's required ICP skill evidence and implementation
boundaries are recorded in `cloud-drive-roadmap.md`.

The owner's visual references guide a distinct default Home (welcome, search,
real folder shortcuts and recently opened files) and My files folder browser.
The sidebar has New and storage usage; type, owner, modified-date and location
filters expose existing queries. Compact sortable columns, list/grid, breadcrumb
navigation and muted gold stars complete the familiar browsing surface.

Selection uses click, Cmd/Ctrl, Shift-range and keyboard navigation. Its action
bar replaces controls in reserved space, so rows do not move on selection or
double-click. Actions include principal sharing, same-space move, recoverable
Trash/restore, stars, single-item rename and download. Multi-file downloads produce
one ZIP; partial failures are reported. Batch mutations retain per-item request
IDs, skip successful retries, group selected descendants and stop on account
changes. Existing backend authorization remains authoritative.

Validation: 104 Node tests (locale, portal flows, shared shell/loading, workspace
and ZIP interoperability), four Python localization tests and ten scoped-workflow
tests pass. The Rust service's 33 unit tests and 16 SDK tests pass; service/facade
Clippy and release Wasm builds pass. PocketIC upgraded the actual pre-milestone
Wasm and re-upgraded the final build, preserving file bytes, grants, staged uploads,
retry receipts and private viewer state. Logs are under `.local/drive-*`.

Browser QA used disposable accounts on the isolated port-51488 network. It covered
cross-account private state, nested metadata search, sort/pagination behavior,
range/toggle selection, menus, rename focus, three-file move/Trash/restore, Home,
filter chips and list/grid. Three selected files arrived in one ZIP and Python's
standard reader verified its entries and CRCs. Actual desktop and 390px taps left
row positions/heights unchanged, with no mobile horizontal overflow. Native Save
As/reopen passed in all eight packaged Craft apps; GridCraft's shared picker was
checked again after the Home refactor. Editor packages were reused, not rebuilt.

The user's existing local environment at port 61304 received the backend upgrade
and final frontend assets without resetting data. Test fixture pages were removed
from the QA canister. No production deployment or Git push was performed. Local
disposable-identity checks do not prove Internet Identity issuance or SEV hosting.
Search remains metadata-only; Recent records explicit portal/shared-picker opens.
ZIP buffering has a 256 MiB browser bound; folder archives and streaming are deferred.

Final `python3 scripts/verify.py portal --timings` passed against the existing local
gateway, checking exact shared assets, certification, cache rules, discovery and
missing/test paths. The separate architecture review retained only the browse/
selection and bounded archive modules: no replacement transport, permission layer,
search service or editor-specific picker was needed. Packaged native editors still
emit their existing eframe viewport/theme warnings; no portal error was observed.


## 2026-10-19 — Subnet account name and editor profile connection (local)

Added one optional display name to the existing principal-owned account record,
with a caller-only bounded update and the existing account-menu entry. Five
existing author profiles receive principal and name via the shared UI adapter;
no new editor profile UI, comment engine, account table or II integration.
Runtime IDs are excluded from persisted preferences. Names in existing native
comments remain snapshots; live resolution of other authors is not implemented.
Viewer remains read-only; Editor can save comments as part of the document.

Focused checks: account name ownership/bounds/stable-memory reopening, five host
profile/account-switch/preference tests and existing affected adapters; Word's
actual comment/reply + cloud-adapter save/open with distinct account names.
Verbose results are under `.local/profile-identity/`. Full Craft engine suites
were not run. No mainnet publication is part of this change.

Validation completed: 13 focused editor adapter/profile/comment tests, one account
storage test, 12 canonical shared-UI tests and 119 existing portal localization
checks passed. All five changed release browser packages and the portal built.
The local backend was upgraded without reinstalling data; the local frontend was
synced, preserving the three unchanged editor packages. The scoped published-asset
verifier passed for the portal and all five changed editors.

Browser proof: the disposable local test account's name was saved in the existing
Subnet account menu. The updated WordCraft package displayed that name in its
existing Options → General profile without entering it in WordCraft. Screenshot:
`.local/profile-identity/word-account-name.png`.
The same browser check also changed the account name while WordCraft stayed open: the existing profile and derived initials refreshed without reloading the editor. Temporary QA tabs were closed.


## 2026-10-19 — Daily workspace A/B/C (local)

Implemented creation intent and revision-aware first autosave in the canonical
cloud UI, with optional creation hooks in Word/Grid/Deck. Empty launches do not
create files; changing document identity cancels the launch destination. Shared
host chrome receives a small versioned presentation signal; saved state follows
the acknowledged document revision, not a local save flag.

Portal return context uses the non-reserved `workspace_return` parameter and
same-origin allowlisting. Selection restores only after fresh authorized reads,
scoped to account and route, with bounded pages and IDs. The unified share view
keeps person grants and explicit public-link policy writes distinct; copying a
link and closing the view never grants access. Home filters collapse without
hiding the search scope. Search copy describes file-text availability without
exposing index management. Morphology and folder overview pages are deferred.

Validation reused the passing canonical creation/autosave/revision tests and
Word/Grid/Deck adapter tests. All eight release browser packages built. Latest
focused portal/host pass: 29 checks initially passed; two old copy assertions were
updated and their two cases then passed. No editor engine suites were run.
Logs: `.local/workspace-ux-implementation/`. Local frontend sync completed (460 assets, state hash
`1e3c8e3da3cb4f1c475ba177200ea80fa04464d71471dff38fb039a6757be688`).
`python3 scripts/verify.py all` passed exact deployed assets, trust/cache and
missing-asset handling. Final browser journeys are recorded below.

Browser results: Word test text, Grid cell value 4 and Deck title MVP survived
cloud reopening in the original team space. Return restored the selected file;
opening a content-search result returned to the same query and selection. A
saved-vs-unsaved departure was checked with Deck: cancel retained work, autosave
completed and return succeeded. The five other updated hosts started with the
shared presentation signal. Search returned TXT/WordCraft/PDF/DOCX/XLSX/PPTX
snippets. Selection did not change absolute row position (569.992 px before and
after); initial viewport changes/browser scrolling were distinguished from layout.

Browser checks caught and fixed two gaps: Deck's native default sample required
its documented `blank` launch flag (now declared as registry `createQuery`, with a
focused test), and the mobile search input needed its own row at 390 px. Public
link expiry and explanatory sharing text use disclosures. Existing role/policy
mutation regressions passed after the layout change.

Word's optional host title service consumes the same versioned presentation signal
as the outer shell, removing the misleading local “Not saved” status for cloud
files without manufacturing a local path or changing save flags. One focused UI
test passed (722 unrelated cases filtered out); only Word was rebuilt for this
additional host presentation hook. Its final deployment and browser checks passed.

Final local publication: 460 assets, state hash
`91210a9c694f2c1a75d1f851ad272622f44c5e26d647c85f12d1ae12a5a7d93b`.
The final scoped verifier (`python3 scripts/verify.py wordcraft`) passed the changed
portal/Word assets and trust/cache/missing-asset checks; the other seven unchanged
packages reuse the earlier successful full artifact verification. Word browser
build d9cc4bf2 displayed the same saved filename/status in native and outer chrome.
Evidence lives in `docs/workspace-ux-evidence/`. UX and 36-feature HTML views were
updated. No Git push, mainnet deployment or data reset was performed.

Limits: automatic initial filenames are provisional and can be renamed through
the existing file workflow. Names and contents remain explicit separate search
modes; no morphology, OCR or semantic ranking was added. Editor-local menus,
local autosave controls and general editor translations remain upstream UI; this
work unifies the cloud flows without replacing those editors.

## 2026-10-19 — Name existing cloud documents without duplicate files

Shared adapter: cloud Save As names an editable object bound to the current
account; an explicit Save a copy action creates a new object. Unbound, read-only
and other-account documents retain the new-copy picker. Naming uses existing
version-checked metadata mutation, then the existing content replacement flow;
no backend/schema change or provisional-name storage is introduced. Reopened
files follow the same behavior. Session/document changes cancel pending naming.
Native editor-local Save As/export remains separate from the cloud menu.

Portal lists, preview headings, details, rename fields and the shared editor title
show the native document stem. Storage/download names retain their suffix. The
WordCraft detail type is “Tekstidokument”. Canonical shared UI is vendored into
all eight hosts; no native editor source changes were needed for this fix.

Focused checks: shared UI 15/15; naming/identity helpers 6/6; selected portal
rename/picker/details flows 14/14; new native-document rename flow 1/1. JS module
syntax and wasm32 controller check pass. Full editor suites were not run.
Logs: `.local/workspace-ux-implementation/naming/`.

Browser proof (local WordCraft build 0608d1f3): empty team folder → type → autosave
`Nimetu 8a2cf25a` → cloud Save As `Nimekontroll`. Exactly one row remained, UUID
`acc6e4db-4e16-4d6b-84a7-d939098c76c1` unchanged. Explicit Save a copy then created
`Nimekontroll koopia` with distinct UUID `a0ac09d0-2d4d-4f5a-84dd-237a99fb6592`.
Original reopened with its name and captured text intact. Screenshot:
`docs/workspace-ux-evidence/named-document.png`. No mainnet publication or Git push.

Final local validation: all eight browser packages built, pinned SDK snapshots
verified, and `python3 scripts/verify.py all` passed against the deployed frontend.
State hash: `93c7d6d567637e233bbd98706136c10581f4c6f25d2d6f877d8eba057ee4d8da`.
Fresh cached-version startup was checked in PhotoCraft, PdfCraft, DeckCraft,
SoundCraft, GridCraft, FigCraft and CADCraft; WordCraft used the full journey above.
CADCraft's existing dirty blank document prevented in-place restart; the empty
QA document was discarded through its normal leave confirmation, then the new
version started successfully. No user file was discarded.
The renamed original still has one content version; details show “Tekstidokument”.
Download was verified as `Nimekontroll.wcraft.json`. Final screenshots include
`docs/workspace-ux-evidence/named-file-details.png`.


## 2026-10-19 — Rename from the workspace and WordCraft titles

Added a capability-gated title button to the common shell and a lifecycle-owned
intent listener to the shared adapter. WordCraft's existing interactive
`file.rename` command forwards to the same controller via one optional service.
Native fallback and parameterized native commands remain intact. Rename preserves
the bound UUID, folder and permissions; read-only bound documents cannot create
an accidental copy through this action. Unbound work uses the first-save picker.

Checks: canonical UI 15/15, shell 12/12, two focused Word title/host-fallback tests
(722 unrelated tests filtered out), wasm controller check and pinned snapshots.
Local Word build ae1cc242: outer title renamed the synthetic Nimekontroll file to
Pealkirja kontroll; native Word title then renamed it to Pealkirjast nimetatud.
Both use the existing metadata mutation and preserve UUID
acc6e4db-4e16-4d6b-84a7-d939098c76c1. Logs: `.local/title-rename/`.


Alpha publication validation: all eight release browser packages built; local
`verify.py all` passed for the portal, all eight editors, trust/cache headers and
missing-asset handling (460 assets; state hash
c0a05664445fa75c1e1d9938cac57217122de22fd7ceeda9663f88f273d27735).
All eight updated cached browser packages started. Word used the complete rename
journey; the other seven used startup/capability checks, without repeating cloud
save/open. CAD's generated empty dirty drawing required normal leave confirmation
before reopening the updated package. No user document was discarded.

GitHub: root implementation 443505e and all eight referenced Craft commits pushed
to their existing origin/main branches. Mainnet cloud upgraded in place to module
4517275c416c19b18d16a3e16b9ba255208cedbbdad4f5bc4b2bb996f581cdb4,
with snapshot 0000000000000004000000000041310d0101; status Running verified.
With explicit user authorization, 0.4811 ICP including fee funded conversion;
1T cycles was deposited to the backend. Frontend publication outcome is recorded
separately after the sync and deployed-artifact verification finish.


Mainnet frontend sync completed: 461 assets, state hash
f4d727568fa9caaeb3ba5768dfd68ba16abb41a4ed720ec40a4e47088561452f.
The extra discovery asset is environment-specific; local and mainnet discovery
configuration are intentionally distinct. No editor, account or file data was
reset. Publication used the existing owner session and existing canisters.

Final verification: `python3 scripts/verify.py all --environment ic` passed
345 published-file checks plus the shared trust/cache/missing-asset assertions.
The custom-domain browser check at https://www.subnet.ee/pilveruum/ reached the
signed-out workspace with no console errors. Authenticated rename/save tests ran
locally; the mainnet smoke check did not create or modify production user files.
Screenshot: `docs/workspace-ux-evidence/alpha-mainnet.png`. The 36-group feature
review now distinguishes shipped autosave/title behavior and account integration
from the remaining comment-service and large-storage work. No full Craft suites
were rerun for this release.
