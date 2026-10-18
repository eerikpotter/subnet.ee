# Permanent Trash deletion

Requirement (2026-10-15): delete individual trashed files/folders and empty all
owned Trash entries, with an explicit warning that restoration is no longer possible.

Status: implemented; unit, browser UI and replica proofs passed locally. Published
to mainnet on 16 October 2026; see the dated release in [history](history.md).

Skills retrieved 2026-10-15 before architecture/implementation:

- [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
- [stable-memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md)
- [icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md)

The [static-site skill](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
was also retrieved on that date before the local frontend sync. The existing
certified-assets recipe and runtime canister configuration are unchanged.

Supported approach: authenticated ordinary canister update calls, authorization
inside the method, and the existing `ic-stable-structures` maps/indexes. No async
boundary is needed. The existing legacy deletion methods reject trashed ancestry,
require separately deleting each revision, and do not cover folders or shared
staging. That observed application gap requires one new owner-only update method;
it does not require a new service, identity system, dependency or stable-memory ID.

Smallest extension: `drive_purge_trash` accepts explicitly selected IDs and metadata
versions. The portal uses it for both a single row and a fully paginated snapshot
of owned Trash. All validation, accounting arithmetic and encoding finish before
stable writes. The method removes each selected subtree, versions, grants, targeted
staging and indexes atomically; deduplication avoids double charging when a parent
and independently trashed child are both selected. A retry receipt survives upgrades.
Prior request metadata remains as tombstones to prevent delayed creates from
recreating deleted content. This is logical deletion through the app, not a claim
of cryptographic erasure from node storage or controller backups.

Alternatives: chaining legacy deletes can leave partially emptied folders and
cannot access Trash; a server-side unqualified "delete everything now" could delete
items the user had not confirmed. A resumable purge job adds state and partial
failure semantics. Current per-owner limits (200 objects including root, 20 files,
20 revisions/file, 256 MiB reserved storage) bound the single update. Revisit batching
before raising these limits. The endpoint remains callable by an AI holding the
owner's normal account credential; no human-presence guarantee is introduced.

Proof tests cover owner isolation despite shared Delete permission, live/root and
stale-version rejection without partial deletion, nested/independently trashed
children, legacy/Drive pending uploads, shared uploader cleanup, quota release,
retained live bytes, retry payload binding, stable reopening, and more than one
Trash page. UI tests cover cancellation, translations, complete snapshot collection,
account changes, error/retry behavior, and storage display refresh.
An actual PocketIC upgrade from `.local/drive-pre-index.wasm` preserved 256 MiB of
legacy content, purged it in one update, verified zero usage and inaccessible bytes,
then upgraded again and returned the identical retry receipt. This establishes the
current storage-bound case locally, not production latency or erased backup bytes.

Review: reuse the existing maps, quota accounting and graph diff/index writer.
The only added contract is the purge request/receipt; the portal facade translates
it without making authorization decisions. Automatic expiration, background cleanup
and editor changes remain outside this requirement. Execution results and browser
validation limitations are recorded in the dated development history.
