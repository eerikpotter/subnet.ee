# Sovereign Cloud account and file service

Canonical backend for the subnet.ee workspace. Deploy it as the existing `cloud`
canister, using the root `icp.yaml`. PhotoCraft's standalone backend is a checked
source snapshot maintained by `scripts/sync-service.py`; its build remains independent.

This milestone adds an idempotent account resolver and an owner-private file catalog.
The authenticated IC caller is the authority; neither a space ID nor an app label is
a credential. Account/personal-space IDs are stable within the logical service.

The old V1 Project, Upload and Revision encodings and Candid methods remain supported.
Stable memory IDs 0–5 are unchanged; 6 stores accounts keyed by principal and 7 stores
account/file index entries. On first account resolution, existing V1 files are indexed
without moving their content. This bounded migration relies on the existing 1000-file
service ceiling. Removing that ceiling requires a batched migration/index strategy.

`list_files(space, cursor, limit)` returns only committed files, maximum 50 per page,
with their latest revision. All current V1 files are PhotoCraft format version 1.
`get_file` checks the same caller/space ownership and returns full version metadata.
Both are update calls so catalog and ownership information comes through consensus.
SDK downloads continue to verify query chunks against a consensus-backed manifest.

The next milestone can introduce a generic typed create operation, file metadata
mutations, folders, typed errors and multi-blob revisions. Do not connect other file
formats to the legacy create_project method. The original quotas and upload retention
policy are still in force; this milestone does not implement shared spaces, grants,
end-to-end encryption or user-controlled canister provisioning.

Tests include owner isolation, idempotent accounts, catalog pagination, incomplete
upload exclusion, stable state reopening, full V1 upload/upgrade behavior and a real
old-Wasm upgrade followed by catalog access and an additional revision of the same file.
