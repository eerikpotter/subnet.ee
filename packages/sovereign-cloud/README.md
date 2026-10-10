# Sovereign Cloud — internal SDK 0.1

An editor-independent Rust client extracted from the working PhotoCraft integration.
The source of truth is `subnet-cloud/packages/sovereign-cloud`. PhotoCraft consumes a
checked, hash-manifested snapshot under `vendor/sovereign-cloud`, so its standalone
fork remains buildable without a relative dependency on the parent repository.
Run `python3 scripts/sync-sdk.py` from the parent after changing the SDK; `--check`
is a required build gate. A future published crate/git release can replace vendoring.

* `protocol`: the existing Candid/serde contract, unchanged for old data and clients.
* `config`: explicit gateway, canister and trusted root key; optional derivation origin.
* `transfer`: create, chunked save/retry, consensus manifests and verified download.
* `browser` feature: shared, storage-aware Internet Identity session adapter and guarded IC transport.
* `launch`: validated service/space/file links; IDs convey no authorization.
* Account/catalog client methods preserve the V1 file transfer API.

The SDK never knows egui, `.pcraft`, editor revisions, menus or document serialization.
PhotoCraft retains those responsibilities and maps SDK progress into its current UI.
`Project` still means a single file in the legacy wire contract. Do not rename wire
fields or repurpose stable-memory IDs. `FileMetadata` and `FileRef` are client vocabulary.

One trusted application family shares the same origin. Subpaths do not change its
principal. The SDK defaults to the current origin and never silently switches existing
accounts to subnet.ee. Alternative origins require explicit setup and data migration.
An app ID is metadata, not an authorization credential.

`BrowserClient::at_endpoint` is the seam for future service/space resolution. V1 file
IDs are only unique within a service: `FileRef.service` identifies that logical service,
not a canister. Mapping it to an endpoint belongs to deployment/resolver configuration.
Per-user canister provisioning, user-owned canisters, grants, encryption, alternate blob stores,
cross-reload resumable uploads and multi-file projects are future work, not implemented.
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
