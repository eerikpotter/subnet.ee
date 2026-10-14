# Subnet Cloud API — alpha

Authentication is the signed IC caller's application principal. Ordinary browser
and ICP MCP calls have identical account authority, including ownership actions.
The service cannot authenticate whether a human or AI operated that credential.
`Participant.User` identifies an account, not a verified human. Separate `Agent`
grants are reserved and rejected in alpha. Anonymous callers can read this API
reference but cannot read or change private data. No operation charges or transfers
ICP tokens; storage and operation limits still apply.

## Shared folders and files

For the initial UI, `drive_home` initializes the account and returns account usage,
the personal root and its first child page in one consensus response. No file
content is included. For tools that only need the root, call `drive_initialize` to obtain the account's personal root and register its
legacy file IDs in the folder tree. Repeating initialization preserves IDs and
bytes. `drive_list(null, null, limit)` lists the caller's own root and directly
shared roots. A directly shared child does not reveal its private ancestors or
siblings. `drive_list(opt parent, cursor, limit)` lists authorized live children.
Pages contain at most 50 entries, ordered by stable numeric ID. Cursors are bound
to the account, parent and tree version; Conflict means restart the listing.
Returned capabilities are for presentation; every operation checks live rights.
`drive_browse(parent, cursor, limit)` returns the folder, its readable breadcrumb
path and the same child page in one consensus response. Breadcrumbs stop before
an inaccessible ancestor. The first page also includes a breadth-first `catalog`
prefetch of at most 200 authorized entries below this folder. It contains only
metadata and latest manifests, never file chunks or full history. `complete_folders`
explicitly identifies folders whose children are fully covered; absence from a
partial catalog does not mean a file is missing. Later pages omit prefetch. Use
this for efficient folder navigation.

`drive_get` reads one live entry and its current immutable revision. Missing and
inaccessible IDs both return Inaccessible. Object IDs are stable across rename and
move. Space and owner never change. An entry may reference a parent that the caller
cannot read. Never assume knowledge of that ID grants access.

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
Legacy names retain their existing spelling and IDs.

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
2 GiB total; 20 files per owner; 20 retained revisions per file; 200 objects per
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
