# Deployment and operations

[Project overview](../README.md) · [Development](development.md) · [History](history.md)

Run commands from the repository root. This guide records the deployment configuration
and publishing workflow. Dated rollout measurements belong in the history.

## Environments and canister mappings

Local development uses its own canisters and data. Production configuration is in
[`icp.yaml`](../icp.yaml); the frontend and account/file canister IDs are in
[`.icp/data/mappings/ic.ids.json`](../.icp/data/mappings/ic.ids.json).
Retain the mapping file so updates reuse the existing canisters.

The 2026-10-14 drive upgrade migrated the old serialized access graph into
authoritative stable indexes. Do not downgrade to a pre-index backend: its old
graph is retained only as a migration source and no longer receives writes.
Prefer a compatible forward fix. Restoring a snapshot also rewinds user data;
review writes since the snapshot before considering that recovery path. Release
hashes, snapshots and validation are recorded in
[collaboration progress](collaboration-progress.md).

The production canisters are controlled using the locally configured `efti-nns`
identity. This is an identity name, not a credential. Signing credentials stay
outside the repository.

Standalone Craft deployments have separate accounts and files. Changing a domain
or deploying this workspace does not migrate their data.

## MCP participation

The shared-drive branch includes an operator-approved ICP MCP opt-in. The frontend
presync hook generates discovery using the target environment's real cloud ID;
the next frontend sync publishes it. Deploy the reviewed backend first so public
Candid and `get_api_doc` describe the same implemented API. This source change has
been deployed to production on 2026-10-14. See [MCP setup and alpha permissions](mcp.md) and
[validation status](collaboration-progress.md).

## Custom domain: `www.subnet.ee`

The public origin is `https://www.subnet.ee`; Zone provides the apex
`subnet.ee` → `https://www.subnet.ee/` 301 redirect, including HTTPS.
The `www` CNAME points to `www.subnet.ee.icp1.io`, `_canister-id.www` contains
the frontend ID in the mapping file, and `_acme-challenge.www` points to
`_acme-challenge.www.subnet.ee.icp2.io`. An existing ICP domain registration must
be updated with `PATCH https://icp.net/custom-domains/v1/www.subnet.ee` after DNS
and the new canister's `/.well-known/ic-domains` ownership file validate.

Mainnet-only settings in `icp.yaml` expose `PUBLIC_SOVEREIGN_CLOUD_GATEWAY`
(`https://icp-api.io`) and `PUBLIC_SOVEREIGN_CLOUD_DERIVATION_ORIGIN`
(`https://vigkv-fqaaa-aaaac-bgeha-cai.icp.net`). The SDK reads the certified
`ic_env` cookie: only `PUBLIC_*` canister settings are exposed by this frontend.
The derivation origin is sent only from alternative origins; the official
canister gateway aliases retain their existing identity behavior.
`portal/.well-known/ii-alternative-origins` authorizes `https://www.subnet.ee`
with JSON/CORS response headers, preserving accounts from this frontend's trial.
Accounts belonging to the old, replaced canister are not migrated by DNS.

`icp sync` does not apply settings. For an existing frontend, apply these two
settings with `icp canister settings update frontend -e ic --identity <identity>
--add-environment-variable KEY=VALUE` before syncing. Sync captures the new
settings into the certified cookie. The local environment keeps using its local
gateway and has no production derivation origin.

After an SDK change, adopt its snapshot in the affected editors and rebuild them
before assembly. Ordinary portal updates continue to reuse packaged editors.
Verify the custom origin with:

```sh
python3 scripts/verify.py portal --environment ic --origin https://www.subnet.ee
```

For a portal-only mainnet update, assemble the current portal and reuse its packaged
editors, then sync assets explicitly to `ic`:

```sh
python3 scripts/assemble.py
icp sync frontend -e ic --identity efti-nns
python3 scripts/verify.py portal --environment ic
```

`python3 scripts/verify.py all --environment ic --timings` also downloads and verifies
all registered editor bundles, reporting compressed HTTP transfer sizes and durations.
These durations measure individual HTTP requests, not browser startup or cache hits.
The landing page does not preload editor bundles. Each editor is fetched when opened;
its fingerprinted JavaScript/Wasm uses a one-year immutable cache. HTML and unversioned
portal assets revalidate so updates are picked up. Cache rules must not combine
`no-cache` with the fingerprinted bundles' immutable policy.
