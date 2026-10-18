# Sovereign Cloud — internal SDK 0.1

An editor-independent Rust client extracted from the working PhotoCraft integration.
The source of truth is `subnet-cloud/packages/sovereign-cloud`. PhotoCraft consumes a
checked, hash-manifested snapshot under `vendor/sovereign-cloud`, so its standalone
fork remains buildable without a relative dependency on the parent repository.
Run `python3 scripts/sync-sdk.py photocraft` to adopt a canonical SDK change in that
fork. Builds use `--check-pinned` to verify the fork's own snapshot; `--check` compares
it with the current canonical source. Other forks may stay on their existing version.
A future published crate/git release can replace vendoring.

* `protocol` and `id`: file/revision types and one canonical UUID v4 object identity.
* `drive`: shared-drive capabilities, typed commands and opaque paginated queries.
* `config`: explicit gateway, canister and trusted root key; optional derivation origin.
* `transfer`: create, chunked save/retry, consensus manifests and verified download.
* `browser` feature: shared, storage-aware Internet Identity session adapter and guarded IC transport.
* `launch`: validated service/space/file links; IDs convey no authorization.
* Account/catalog client methods preserve the V1 file transfer API.

The SDK never knows egui, `.pcraft`, editor revisions, menus or document serialization.
PhotoCraft retains those responsibilities and maps SDK progress into its current UI.
`Project` describes a file's immutable content revision chain; `DriveObject`
describes its catalog metadata. They use the same UUID key, not separate identities.
`FileMetadata` and `FileRef` are client vocabulary. Numeric revision/upload IDs are
not file IDs. Editor-local document IDs remain inside each host's native adapter.
The canonical shared controller owns conversion to/from cloud object references.

One trusted application family shares the same origin. Subpaths do not change its
principal. The SDK defaults to the current origin and never silently switches existing
accounts to subnet.ee. Alternative origins require explicit setup and data migration.
An app ID is metadata, not an authorization credential.

`BrowserClient::at_endpoint` is the seam for future service/space resolution.
File UUIDs are globally unique and independent of an owner or storage canister.
`FileRef.service` identifies the logical service; endpoint resolution currently
selects one configured canister. Per-user provisioning, user-owned canisters,
distributed placement and alternate blob stores remain deferred. Current sharing
is checked by the backend on every operation, including retries and pagination.
The current server retains its prototype quotas, including 64 MiB per document.

Save retry expects the same immutable bytes and UploadState. Checkpoints must be retained
after a failed request. A lost create response can leave an empty file in the V1 API;
the backend does not have idempotent creation. Revision conflicts are surfaced, not retried
with an overwritten expected revision. Downloaded bytes are not released until SHA-256
matches the consensus manifest. The implementation still buffers the full snapshot.

The SDK is derived from `eerikpotter/photocraft`'s ICP integration, under MIT OR Apache-2.0.

The HTML portal uses the sibling `sovereign-cloud-web` Wasm facade. Both clients use
BrowserSession; stale/expired sessions cannot start subsequent requests. Host UIs must
also discard responses from obsolete sessions and preserve unsaved documents. Existing
Rust AuthClient storage keys are deliberately retained for compatibility.
