# Sovereign Cloud file service

Canonical Rust backend for subnet.ee. All current data stays in one canister.
Use code-only upgrades of the existing `cloud` canister; never reinstall to update
code. PhotoCraft’s separately pinned backend snapshot remains independent.
See [API reference](api.md), [development](../../docs/development.md) and
[architecture evidence](../../docs/collaboration-spec.md).

## Boundaries

Principals identify callers and personal viewer state. Space UUIDs identify file
collections: a personal owner or explicit team membership supplies authority.
Teams have Manager/Editor/Viewer roles, separate quota and pending-upload indexes;
founder departure preserves their files. A UUID is an address, never a credential.
Public links are separate read policies with minimal anonymous projections.
All named, public, transfer and search paths recheck current authority.

The browser/ICP MCP authenticate the same account principal; restricted agent
credentials are still deferred. Editor engines do not implement cloud policy.
One shared SDK handles sessions, transport, immutable manifests and verified bytes.

Normal reads seek scoped stable B-trees and load only the necessary path/page.
Writes update affected objects and indexes synchronously. Scoped versions and
idempotent receipts protect retries and concurrent changes. File data lives in
stable memory. No user-owned canister provisioning or routing service is added;
space boundaries leave placement as a later extension.

## Derived content

`packages/sovereign-cloud-text` extracts canonical committed text without identity,
networking or editor dependencies. A stable queue and standard timer/self-call
separate parsing from file commits. Revision-bound sections, postings and source
provenance are rebuildable; query execution never reads file bytes. Current
revision/ACL/expiry/Trash/visibility checks precede snippet publication.
See the API for exact file/format/work limits, partial states and retry behavior.
The PDF crate has one documented lookup-table patch, verified against all upstream
mappings; all other upstream source hashes are checked.

## Stable memory

Existing region IDs remain permanent. New regions for this package:

- 62–67: team records, memberships, memberships by user, usage, file counts and
  pending uploads by space.
- 68: public policies by object.
- 69–76: extraction records, sections, postings, reverse terms, term counts,
  durable jobs, pending revisions and incremental backfill state.

The full layout is defined in the store modules. UUID-schema upgrades preserve
accounts, files, permissions and receipts. Numeric-ID development schemas have no
compatibility layer. Do not downgrade across schema changes; use a forward fix.

## Checks

Canonical unit tests cover live authority, retries, quota, indexes, parser bounds
and failure recovery. PocketIC tests install actual Wasm, exercise all six search
formats and team ownership boundaries, and upgrade populated state. Portal tests
cover anonymous previews, management, selection, search and stale response handling.
Actual local browser journeys complement the fixtures. Neither these tests nor
bounded single-canister work establish 100k-user production throughput.
