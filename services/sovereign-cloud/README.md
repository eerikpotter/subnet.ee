# Sovereign Cloud file service

Canonical backend for subnet.ee. Deploy upgrades to the existing `cloud` canister
using the root `icp.yaml`; never reinstall it to update code. PhotoCraft's standalone
backend snapshot remains independent (`scripts/sync-service.py`).

The signed application principal is the authority. A space/file ID or app label is
not a credential. Browser and hosted ICP MCP calls currently have the same account
rights; separate agent credentials and proposal workflows are deferred. See
[API reference](api.md) and [collaboration specification](../../docs/collaboration-spec.md).

## Storage and access

Files retain their owner and space when moved or shared. Principals index owned
objects/projects and received grants; parent IDs index folder children. File IDs
resolve directly. `drive_home` returns the account and initial view in one
consensus update. Folder navigation returns the folder, readable breadcrumbs,
one page and, on its first page, a bounded metadata catalog of up to 200 entries
in that subtree (including its root). The catalog explicitly identifies completely
listed folders; partial folders are never represented as complete. File contents
and revision history are not prefetched. Shared callers never receive private ancestors
or siblings. Every operation checks current grants, expiry and the live hierarchy.
Chunk queries are verified against consensus-backed manifests by the SDK.

Stable memory IDs are permanent:

- 0–8: original counters, projects, uploads, immutable chunks, usage, accounts,
  account/file catalog and native format metadata. Original encodings are retained.
- 9: retired serialized access graph, used only for the one-time migration.
- 10: reserved for a withdrawn prototype; never reused.
- 11–13: attribution, mutation receipts and drive upload staging.
- 14–20: authoritative objects, parent index, grants by root, grants by recipient,
  objects by owner, tree version and migration marker.
- 21–23: projects by owner, pending uploads by owner and by uploader.

Upgrade initializes these indexes once from the bounded previous alpha state.
Normal reads fetch the requested path/page; mutations load only the affected
owner's bounded working set and persist changed records. They do not deserialize
other accounts' files. No user files are kept solely in heap memory. Related
records and indexes change synchronously in the same canister message.

After migration, **do not downgrade to a pre-index binary**: memory 9 is frozen.
Use a compatible forward fix. A snapshot restore requires reviewing every write
since the snapshot because it rewinds user data as well as code.

## Compatibility and checks

Legacy Craft methods remain owner-only, reject trashed ancestors, and retain
native bytes, IDs and history. New drive methods support folder organization,
sharing, soft deletion/recovery, inline files and chunked create/replace. The
current alpha storage, file count and revision limits remain in force. No
per-user canister provisioning or end-to-end encryption is added here.

Unit tests cover ACLs, scope isolation, moves, trash, retries and index persistence.
SDK/PocketIC tests cover original V1 upgrades, all seven native formats and an
upgrade from the pre-index shared-drive Wasm, including grants, lost commit
responses and immutable bytes. Browser QA separately checks actual navigation,
upload feedback, session changes and language switching.
