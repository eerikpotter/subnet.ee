# Subnet Cloud API — alpha

Authentication is the signed IC caller's application principal. Ordinary browser
and ICP MCP calls have identical account authority, including ownership actions.
The service cannot authenticate whether a human or AI operated that credential.
`Participant.User` identifies an account, not a verified human. Separate `Agent`
grants are reserved and rejected in alpha. Anonymous callers can read this API
reference but cannot read or change private data. No operation charges or transfers
ICP tokens; storage and operation limits still apply.

## Object identity and bounded retrieval

Every file, folder, shortcut and personal space has one UUID v4 identity. Candid
and JSON carry it as text; stable indexes use the same UUID's sixteen bytes.
There is no numeric alias or separate public ID. Revisions, uploads, grants and
activity events have their own numeric sequence IDs and must not be used as file
IDs. Links use `/pilveruum/file/<uuid>` or `/pilveruum/folder/<uuid>` (with an
optional `/en` locale prefix); the UUID is an address, never a permission.

All accounts currently use one configured canister. Ownership remains in the
object/space metadata. Multi-canister placement and resolution are deferred; no
per-file location service is required today. This schema deliberately requires a
fresh installation from the previous numeric-ID development schema. Upgrades
within this schema preserve data, permissions, receipts and opaque bookmarks.

`drive_query` seeks ordered name/modified/size indexes in the requested scope.
Each page returns at most 50 matches and examines at most 512 candidates.
**An empty page with `next` is not the end:** continue with that opaque cursor.
A cursor cannot be used by another caller or for different filters/sort/scope.
On Conflict, restart the current listing. Folder versions depend on that folder,
its ancestry and relevant access changes; sibling/account writes do not invalidate
it. Cross-space discovery depends on its source spaces and viewer state, never a
global epoch. Every returned item is checked against current permissions.

Discovery currently accepts up to 256 shared roots from 64 source spaces; larger
views return an explicit request to narrow to a folder. Recursive mutations are
bounded at 4096 objects, combined purge work at 8192, and account aggregate
summaries at 8192. These safety bounds are not a claim of production capacity;
resumable jobs and distributed aggregates remain future extensions.

## Shared folders and files

For the initial UI, `drive_home` initializes the account and returns account usage,
the personal root and its first child page in one consensus response. No file
content is included. For tools that only need the root, call `drive_initialize` to
obtain or create the account's personal root. Repeating initialization preserves
IDs and bytes. `drive_list(null, null, limit)` lists the caller's own root and directly
shared roots. A directly shared child does not reveal its private ancestors or
siblings. `drive_list(opt parent, cursor, limit)` lists authorized live children.
Pages contain at most 50 entries, ordered by canonical UUID. Opaque cursors are bound
to the account, parent and scoped version; Conflict means restart the listing.
Returned capabilities are for presentation; every operation checks live rights.
`drive_browse(parent, cursor, limit)` returns the folder, its readable breadcrumb
path and the same child page in one consensus response. Breadcrumbs stop before
an inaccessible ancestor. The first page also includes a breadth-first `catalog`
prefetch that inspects at most 200 records below this folder, including filtered
records. It contains only
metadata and latest manifests, never file chunks or full history. `complete_folders`
explicitly identifies folders whose children are fully covered; absence from a
partial catalog does not mean a file is missing. Later pages omit prefetch. Use
this for efficient folder navigation.

`drive_get` reads one live entry and its current immutable revision. Missing and
inaccessible IDs both return Inaccessible. Object IDs are stable across rename and
move. Space and owner never change. An entry may reference a parent that the caller
cannot read. Never assume knowledge of that ID grants access.

`drive_details(object)` returns the same authorized live entry, revision count,
readable ancestor locations and active access sources for the signed caller only.
It never returns the full recipient roster or an inaccessible ancestor's name.
Source capabilities are bounded by current effective rights. Ownership is evident
from the entry owner; an empty source list does not remove the owner's rights.

`drive_create_folder` and `drive_create_file` require Read + Create on the parent.
Creation never overwrites a collision. Inline file bytes are a Candid byte vector,
up to 524288 bytes; zero-byte files are supported. `media_type` is descriptive and
must be nonempty. `craft` is optional validated native format metadata: storing
bytes with a Craft type does not convert or validate their native document syntax.
Use actual native serialization for Craft projects.

Names are trimmed, normalized to Unicode NFC, limited to 160 UTF-8 bytes, and
exclude controls, slash, backslash, `.` and `..`. Sibling collisions compare NFC
names after Unicode lowercase conversion, preserving the displayed spelling. This
is not full Unicode case folding; distinct spellings such as ss and ß remain distinct.

Create, grant and organization calls take a caller-scoped 16–80 byte request ID. Reuse the
same ID and payload after an uncertain response; a changed payload conflicts.
Retries recheck authorization. File creation returns the same ID with its current
metadata on retry. Do not generate a new request ID merely because a response was
lost. Request receipts are bounded; limit errors do not silently evict receipts.

## Sharing

Live entries include optional `sharing` metadata with `direct` and `inherited`
flags. These reflect active, unexpired Read grants on the item and its ancestry.
Owners see sharing with other accounts; recipients see only grants to themselves.
No recipient identities or private ancestor names are exposed. This is display
metadata, never authority. Older servers/receipts may omit it; trash entries omit it.

Only the owning account can call `drive_set_grant` or `drive_grants`. The recipient
must have an account in this application. Grant capabilities are explicit Read,
Propose, Create, Edit and Delete; every enabled action requires Read. Create applies
only to folders. Rights from live, unexpired grants on an object and its ancestors
are combined. Ownership supplies all rights. Grants have no deny rules.

For a new grant, omit ID and expected_version. For an update/revocation, provide
both from the existing grant. Set active=false to revoke. Revoked/expired grants
cannot be revived; create a new grant. Reusing an identical request ID returns the original grant result; inspect
`drive_grants` for its latest state if it was subsequently changed. Expiry is Unix nanoseconds; absent expiry means no time limit.
Bob's MCP calls have Bob's rights and lose shared access when Bob's grants change.
Revocation cannot retract downloaded bytes or revoke an II credential globally.

Propose is represented in the permission model; proposal submission/approval APIs
are not implemented in this build. Do not assume a Propose grant permits direct edits.

## Organization and recovery

`drive_restore_preview(object, parent, name)` checks an explicitly trashed item's
proposed destination/name and returns the same versioned access-impact model as a
move. `RestoreReviewed { parent, name, tree_version }` applies that plan through
`drive_organize` with its normal expected object version and request ID. The target
must be a live folder in the same space, outside the item's subtree. A changed
name/destination flow needs Edit as well as Delete; a changed destination also needs
Create there. Changed inherited access or expiry requires the owner. There is no
overwrite. The original `Restore` operation remains compatible.

Undo is a client workflow issuing ordinary rename/move/restore commands against
the versions returned by successful mutations. It does not replay old grants or
overwrite catalog snapshots. Current authority, versions and move impact apply
to the inverse action, including when an agent performs it.

`drive_move_preview(object, parent)` is an authenticated consensus read through
an update method. It validates the same destination, subtree, capability and depth
rules as the mutation, then returns `object_version`, `tree_version`, affected
object/subject counts and up to 100 recipient summaries. A summary's gains/losses
include capability duration changes, using the union of active grants and their
expiry envelopes; file Create is masked and agent authority remains intersected
with its sponsor. Any change requires the human owner. Editors can move only when
effective access remains unchanged, and cannot retrieve others' changed access.

Use `OrganizationChange::MoveReviewed { parent, tree_version }` to bind a user's
review to the current catalog. An intervening catalog change rejects publication.
Successful retries with the identical request return the existing receipt, after
reauthorization. The original `Move(parent)` remains available and enforces the
same owner requirement; it does not imply a human UX review. No additional stable
records, memory IDs or ACL copies are introduced.

`drive_organize` uses an object's expected metadata version. Content revision IDs
are separate from this version. Rename requires Read + Edit. Move requires Read +
Edit + Delete on every descendant and Read + Create on the destination. Moves stay
within one space and reject cycles/depth overflow. Moving changes inherited rights.
Trash and Restore require Read + Delete on the entire subtree. Roots cannot change.
Trash hides the object and descendants without removing their bytes. Restore keeps
the original parent, which must be live, and rejects an occupied name. A separately
trashed descendant stays trashed when its parent is restored. Normal operations
reject trashed ancestors. `drive_trash` provides recovery listings; a separate
organization activity log is not implemented.

`drive_current_chunk(file, revision, index)` requires current Read access and the
current revision ID; a changed head returns Conflict. Metadata is read by consensus
updates. Chunk queries must be checked against the returned byte length and SHA-256.
Chunk indices start at zero; empty revisions have no chunks. Timestamps are Unix
nanoseconds. For shared replacement, immutable revision reads and recovery, use the
chunked drive methods described below. Proposal workflows are not implemented.

Limits: 64 MiB per document through legacy chunked uploads; 256 MiB per owner;
2 GiB total; 20 files per owner; 200 retained revisions per file; 200 objects per
space; 10000 drive objects globally; 64 levels including the root. Published history
is not automatically pruned when limits are reached.

## Legacy owner-only APIs

Existing file/project APIs remain available for the existing Craft clients. They
remain owner-only and reject trashed objects/ancestors. `create_file` takes a validated
FileKind; `create_project` defaults to PhotoCraft. `begin_upload` reserves quota for
1–64 MiB, checks expected_revision and a 32-byte SHA-256, and returns a staging ID.
Only one pending upload is allowed per file. `put_chunk` accepts exact 512 KiB chunks
except the final chunk. Repeating identical chunks is safe; changed bytes conflict.
`commit_upload` publishes only when all bytes match the expected digest and head;
repeating commit returns the same revision. Interrupted staging is not published.

`delete_revision_or_upload` permanently removes a retained revision or staging
upload, including its bytes. It is irreversible and available to the owner's MCP
credential too. `delete_project` permanently removes an empty project; its revisions
and pending upload must already be removed. These legacy methods are not Trash.
There is no human-only deletion safeguard in this alpha. Prefer drive Trash for
recoverable organization changes. Cleanup/retention automation is not implemented.

## Chunked drive transfers and recovery

Use `drive_begin_upload` with a `New` target (parent/name/format) or `Replace`
(file/expected_revision), byte count, SHA-256 and a unique 16–80 byte request ID.
The service reserves the owning space's quota. The maximum file size is 64 MiB;
empty files are supported. Send exact 512 KiB chunks (except the final chunk)
with `drive_put_chunk`, then `drive_commit_upload`. Every step checks current
permissions. Replacement requires Read + Edit; creating requires Read + Create
on the destination. Committing checks the expected content head again and publishes
atomically. Partial uploads are invisible. A repeated begin with the same request
returns the existing upload; repeated commits do not add versions. Preserve request
IDs and bytes when a response is lost. Staging expires after 24 hours.

`drive_pending_uploads` lists the caller's unfinished uploads and those in their
owned space. `drive_abort_upload` releases staged bytes and quota; the uploader or
space owner can discard staging even after access was revoked. Published revisions
cannot be aborted. Expired staging must be discarded to reclaim quota.

For a current download, get the consensus manifest via `drive_get`, then retrieve
`drive_revision_chunk` and verify total bytes and SHA-256. Read is sufficient for
current content. `drive_history` and historical chunk reads require Read + Edit.
To restore old content, download that revision and upload it using `Replace` with
the **current** expected head. This adds a version and retains intervening history.

`drive_trash` returns a paginated list of explicitly trashed items for which the
caller currently has Read + Delete. Its cursor is scoped separately from ordinary
folder listings. Restore with `drive_organize` and the current metadata version.
The original parent must be live and the name available. Restore parent folders
before independently trashed children. No automatic purge is implemented in alpha.

`drive_purge_trash(PurgeRequest { items: vec MutationReceipt { object, version },
request_id })` permanently deletes 1–200 explicitly trashed roots and their entire
subtrees. Only the owning account may call it; shared Delete rights do not authorize
purge. Roots, live items, duplicates and stale versions are rejected before any
deletion. To empty Trash, collect every `drive_trash` page, filter to the caller's
owned items, and submit their IDs/versions together after destructive confirmation.
Objects newly trashed outside those subtrees are not implicitly included.

The operation atomically removes file revisions/chunks, legacy and Drive uploads
targeting deleted files/folders, attribution, grants and catalog indexes. It releases
reserved quota and returns `PurgeReceipt { deleted_objects, freed_bytes }`. Existing
alpha object/file/storage limits bound the work. The same caller/request ID/payload
returns the original receipt, including after upgrade; changed payload returns
Conflict. Prior mutation receipt metadata is retained to prevent delayed requests
from recreating purged content. File contents cannot be restored through the app.

## Browsing, metadata search and personal views

`drive_browser_home()` initializes the caller's space and returns account usage,
saved browsing preferences and the first sorted personal-folder page in one call.
`drive_query(BrowseQuery, opt BrowseCursor, limit)` is the common metadata retrieval
operation. These are consensus **update** calls, even though the query itself
does not mutate files. ICP MCP therefore still needs Actions & questions for this
interface; application Read/Edit permissions remain independently enforced.

Scopes: `Folder(id)` lists immediate children, `Subtree(id)` searches descendants,
`All` covers all currently readable owned/shared descendants, `Shared` lists shared
entry points, `Item(id)` resolves one restricted link without revealing private ancestors,
`Recent` and `Starred` use the caller's private state, and `Trash`
retains the existing Read + Delete requirement. Readable breadcrumbs stop before
private ancestors. Live scopes exclude trashed ancestors, unpublished uploads and
inaccessible items. No result or count confers access for a later action.

`text` is an NFC-normalized, case-insensitive filename substring, up to 160 UTF-8
bytes. `file_type` is empty (any), folder, file, image, audio, video, text, pdf or
a supported Craft app ID. `owner` is an optional principal. Modified bounds are
nanoseconds since the Unix epoch: inclusive `modified_after`, exclusive
`modified_before`. Folder/subtree scope supplies location. This does not search
document contents. No external search database or inference service is used.

Sort fields are Name, Modified (object metadata timestamp), Size (current content
bytes; zero for folders), and Opened (this caller's last explicit open). Names use
normalized Unicode lexical ordering, with object ID as the final tie-breaker;
ordering is independent of interface language. Folder and Shared views keep folders
first in either direction. The server sorts the complete authorized result before
returning 1–50 items. Existing parent/owner/grant indexes supply candidates, bounded
by the current 10,000-object catalog; larger searches return Limit, never partial
success. A future larger deployment can extend retrieval behind this API.

Cursors bind caller, exact filters/order, catalog version and, for Recent/Starred or Opened sorting, viewer-state version.
Grant expiry also invalidates a continuing page. On Conflict, discard accumulated
pages and restart. Cursors contain only a returned item's ID, not hidden names or
sort keys. Every page rechecks current authority. Do not construct or modify cursors.

`drive_star(id, starred)` sets a private flag and requires current Read access.
It is idempotent for the same desired value and never changes file permissions.
`drive_acknowledge_open(id, revision)` records a successful foreground open for the
signed-in viewer. Files require the revision actually opened (current Read or the
existing history permission); folders use null after their listing succeeds. The
root is not tracked. Duplicate acknowledgements of the same revision within one
minute coalesce. Recent displays files only; at most 200 file/folder summaries are
retained. Portal previews acknowledge only after rendering/decoding succeeds, and
the shared Craft controller only after native document decoding succeeds. Activity
failure never prevents document access. Downloads, selections, background metadata
reads and failed opens are not successful opens.

`drive_open(id)` is retained for existing clients: it returns the current entry and
records open intent without asserting a successfully seen revision. New clients
use `drive_get` followed by acknowledgement at the foreground consumer.

`drive_suggested_home()` returns up to 20 files and six folders, derived from current
permission-filtered metadata and the caller's private summaries. Reasons are
UpdatedSinceOpened (a newer content revision after an acknowledged open in the past
30 days), RecentlyOpened (past 30 days), Starred, or RecentlyCreated fallback, in
that priority order; reason timestamps and stable IDs break ties. Renaming does
not count as a content update. No scores, explanations or event log are stored.
Every read revalidates grants, expiry and Trash. Reasons are facts, not permission.

`drive_dismiss_suggestion(id)` hides a currently readable item from this viewer's
Home for seven days; repeated calls during that interval do not extend it. It does
not unstar, delete, change sharing or remove the item from Recent/search. At most
200 active dismissals are retained; older dismissals and expired summaries are
pruned during viewer summary mutations. Intentional stars survive pruning.

`drive_preferences()` reads the caller's private List/Grid and sort settings;
`drive_set_preferences(BrowsePreferences)` replaces them. Starred, Recent and
preferences survive upgrades and are never returned for another account. Revocation,
expiry, Trash and purge hide previously starred/recent items on the next read.
Viewer data uses stable memory regions 24 and 25; existing record encodings are extended with optional, defaulted opened_revision
and hidden_until fields; existing keys and region assignments remain unchanged.


## Optional Subnet email contacts and invitations

These authenticated update methods do not change Internet Identity or provide a
second login mechanism. The caller's principal owns its contact list. A principal
may verify up to ten addresses; each normalized address has one current principal.
Email transfer does not move files, merge accounts or transfer existing grants.

- `email_profile()` returns this caller's contacts, pending verification and whether
  sending is configured. There is no public email/principal lookup.
- `email_request({ email, request_id })` reserves a caller-bound challenge, obtains
  management-canister randomness, and attempts delivery. Request IDs are 16–80
  characters. Retrying the current ID does not send another message. A new request
  replaces the previous challenge; the UI generates UUIDs.
- `email_verify(id, code)` checks an eight-digit code within ten minutes and five
  attempts. It returns `Added`, or `TransferRequired` after successful proof when
  another principal owns the address. Proof is single-use.
- `email_confirm_transfer(id)` explicitly completes a proved, unexpired transfer
  only if the address generation has not changed. Old pending invitations do not
  transfer. Future email shares resolve to this principal.
- `email_remove(email)` removes only this caller's contact; existing ACLs remain.
- `email_share({ object, email, role, request_id })` is owner-only, requires a
  verified sender contact and accepts Viewer/Editor. A bound address receives a
  canonical principal grant and a direct restricted URL. Otherwise it creates a
  seven-day pending invitation. A repeated direct share updates its role. Email
  transport failure is returned as delivery state, separately from the grant.
- `email_invitations(object)` is owner-only and returns recent invitation receipts.
  `email_revoke_invitation(id)` cancels a pending invitation; revoke accepted access
  through the ordinary grant API. Accepted receipts describe past acceptance, not
  current effective access.
- `email_accept(id)` requires the invited verified address and current binding
  generation, then rechecks target/owner authority before creating the grant. A
  forwarded URL conveys no authority. Retrying an accepted invite never restores
  revoked access. Returns the object ID for navigation.
- `email_retry(id)` permits only the sending principal. It reuses the exact payload
  and idempotency key, after a two-minute lease, within 24 hours and five attempts.
  Verification requests have a shorter ten-minute lifetime. No background retries.

Addresses use conservative ASCII validation and lowercase normalization. Dots and
plus tags are preserved. Sending limits: 30/account/day, 10/address/day,
1000/canister/day; verification cooldown 60 seconds. Delivery `Accepted` means the
provider accepted the message, not that it reached an inbox. `Unknown` keeps the
outcome uncertain; do not create another message automatically. Delivery payloads
are erased on acceptance/failure or bounded expiry cleanup. Receipts expire after
eight days; principal grants and verified contacts persist independently.

The backend reads `RESEND_API_KEY`, `SUBNET_MAIL_FROM` and `SUBNET_MAIL_ORIGIN` from
controller-managed canister environment variables. Missing credential disables
sending. Sender is fixed to `subnet.ee <info@subnet.ee>`; destinations in messages
use the configured app origin. Credentials never appear in public API responses.
Only private controllers may read canister status/snapshots/logs. Local configuration
uses `scripts/configure-mail.py`. SEV hosting is a future deployment choice, not
a prerequisite for these account and sharing flows. New stable memory IDs 26–35 hold contacts, address generations,
challenges, invitations/index, delivery jobs, rates, ID and cleanup cursors.


### Activity, storage and version controls

`drive_activity(object: opt text, before: opt nat64, limit: nat32)` is an
account-authenticated update read. Without an object it pages the caller's owned
space; a supplied object gives exact-object history, including an accessible
shared file. Each page rechecks current authorization. The latest 100 successful
mutations per object are retained. Names resolve from the current catalog;
sharing and version-management facts are owner-only. Purging removes its events.
Older objects have no fabricated pre-upgrade history. Open/read telemetry is not
recorded here. Stable memory IDs 36–38 hold event sequence, owner events and the
per-object index; their writes publish atomically with the existing mutation.

`drive_storage()` returns the owning account's current, historical, trashed and
reserved pending bytes, plus files sorted by total retained content size. It is a
projection of existing ownership, revisions and accounting, not another usage
store. Shared recipients are not charged for the owner's files.

`drive_versions(object)` keeps the existing Edit + Read history authority and
adds revision annotations from stable memory ID 39. Existing revisions default to
manual, unnamed and unpinned. `drive_label_revision` is owner-only, versioned and
retry-safe; a nonempty name always pins the revision. `drive_delete_revision` is
owner-only and confirms both object and annotation versions, disallows the current
or pinned version and active replacement uploads, frees bytes exactly once, and
retains retry tombstones. Removing a name/pin is a separate explicit operation.
All validation and encoding precede mutation. Existing file content and stable
IDs remain unchanged by labeling. Ordinary history/download APIs remain supported.

The accepted autosave policy is to keep the latest 20 unpinned automatic
checkpoints while preserving all manual/named/pinned revisions. New uploads accept an optional `automatic` flag (absent means manual). A committed
automatic upload prunes only excess unpinned automatic checkpoints, in the same
mutation as its revision and quota publication. The editor integration uses this
flag; manual uploads never start background pruning. All old records remain manual.


### Organization, requests and recipient controls

`drive_create_shortcut(CreateShortcut)` creates an owned catalog object pointing
to one currently readable non-shortcut target. `drive_shortcut_target(object)`
rechecks access to both; moving or trashing the shortcut never changes the target.
`drive_folder_color(object, opt text)` keeps a caller-private palette choice in
existing viewer state. Color and shortcut metadata contain no file contents.

`drive_request_access(object, message)` returns a uniform acknowledgement without
revealing whether the target exists, belongs to whom, or allows requests. Requesters
share one bounded daily budget across existing and missing object IDs. Owners see
pending requests in `drive_inbox()` and approve/decline with `drive_decide_access`;
approval creates a canonical grant. Duplicate request and decision receipts are
idempotent; requests expire after 30 days. Inbox entries resolve names against live
permissions, retain 100 entries, and support `drive_read_notice` and
`drive_notifications`. Requests remain visible when ordinary notices are disabled.
`drive_notify` sends an in-app reminder to an existing recipient, at most hourly.

`drive_contacts`/`drive_contact` store up to 200 private principal labels per caller.
Labels are presentation only. Email invitations may specify `grant_expires_at`;
invitation acceptance revalidates the requested end time and creates the same
principal grant. Email is never itself an ACL subject.

`drive_hide_shared` removes an item and its descendants from aggregate views only;
direct access remains possible. `drive_block` blocks both directions of sharing
through the normal access evaluator while retaining stored grants; unblocking makes
those grants effective again. `drive_sharing_safety` lists the caller's choices.
`drive_report` stores only report metadata (100/account); controller-only
`admin_sharing_reports` pages 50 reports. Reporting does not imply automated action.
Stable memory IDs 40–45 hold communication, and 46–48 hold recipient controls.
