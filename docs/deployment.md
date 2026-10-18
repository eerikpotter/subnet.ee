# Deployment and operations

[Project overview](../README.md) · [Development](development.md) · [History](history.md)

Run commands from the repository root. This guide records the deployment configuration
and publishing workflow. Dated rollout measurements belong in the history.

## Environments and canister mappings

Local development uses its own canisters and data. Production configuration is in
[`icp.yaml`](../icp.yaml); the frontend and account/file canister IDs are in
[`.icp/data/mappings/ic.ids.json`](../.icp/data/mappings/ic.ids.json).
Retain the mapping file so updates reuse the existing canisters.

The current backend uses canonical UUID keys and intentionally has no
numeric-ID migration. Its schema guard rejects an older numeric stable layout.
The authorized local and mainnet development deployments were freshly installed
on 18 October 2026. Mainnet's previous numeric-ID data remains in snapshot
`0000000000000003000000000041310d0101`. Code-only upgrades below apply between versions of
the same UUID schema. Deployment over an older layout needs a separate deliberate
fresh installation/new canister, not an attempted in-place conversion.

Historical production state: the 2026-10-14 drive upgrade migrated the old serialized access graph into
authoritative stable indexes. Do not downgrade to a pre-index backend: its old
graph is retained only as a migration source and no longer receives writes.
Prefer a compatible forward fix. Restoring a snapshot also rewinds user data;
review writes since the snapshot before considering that recovery path. Release
hashes, snapshots and validation are recorded in
[release history](history.md) and [earlier collaboration progress](collaboration-progress.md).

The production owner's Internet Identity principal remains unchanged. Release
18 October 2026 used the fresh Keychain-backed CLI session
`codex-subnet-release-20261018`; the older `efti-nns` delegation had expired.
Use a current authenticated session explicitly with `--identity`; these names are
not credentials. Signing credentials stay outside the repository.

Standalone Craft deployments have separate accounts and files. Changing a domain
or deploying this workspace does not migrate their data.

## Backend upgrades with canister-held secrets

Build and use `icp canister install cloud --mode upgrade` for code-only updates to
an already configured backend. Keep the normal Candid compatibility check. A
canister upgrade preserves environment variables; the CLI's broader `icp deploy`
workflow re-applies deployment settings and was observed removing the provider key
because it is intentionally absent from `icp.yaml`. Do not fix that by adding
secrets to source. If deployment settings are deliberately reapplied, configure
mail afterwards through the private controller and verify redacted status. See
[the credential workflow](development.md#canister-held-email-credentials).

### CycleOps monitor and mail secrets

Requirement: configure the backend-held Resend key through its owner while
preserving existing monitoring. Skills consulted on 18 October 2026:
[canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
and [icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md),
including its `references/canister-env-vars.md`. The supported approach is signed
management settings with private visibility and a Keychain-backed identity.

Observed gap: production already has CycleOps V3 (`cpbhu-5iaaa-aaaad-aalta-cai`)
as its second controller; the provisioner's original sole-controller check rejected
it. This is a status monitor, **not a recovery controller**. Removing it would
interrupt monitoring. An arbitrary-controller bypass would weaken secret custody.

Smallest extension: explicit `--allow-cycleops-monitor`, accepting only the owner
plus this exact monitor, and checking its certified controller list is empty and
its module hash is
`dfcf37d4ee18bfe77a12197feaf4686f160d717383f0cf525f9d30e60fec204d`.
All visibility settings remain controller-only; no controllers are changed.
The [reviewed source and published Wasm](https://github.com/CycleOperators/BalanceCheckerVerification/tree/24d22bbb07f2de207c215b70b450df9ac172de11/blackholes/v3)
matched the on-chain hash. Its typed status result omits environment variables,
and its only management call is `canister_status`; it cannot update settings,
read snapshots or upgrade code. The published binary was hash-checked, not
independently rebuilt during this release.

Proof: eight operator tests cover default rejection, explicit monitor acceptance,
changed hash/controller/ID rejection before secret access, unrelated-settings
preservation, private temporary files, cleanup and redacted status. Provision with:

```sh
.tools/mail/bin/python scripts/configure-mail.py --network https://icp-api.io --canister vbfbj-tyaaa-aaaac-bgegq-cai --identity <current-owner-session> --allow-cycleops-monitor
```

The exception is pinned to the reviewed immutable module, not future monitor
versions. Ordinary subnet operators remain within the previously accepted trust
boundary until SEV hosting; this release does not claim SEV deployment.

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
Before a full mainnet fleet sync, check the frontend's cycles balance against its
freezing reserve (`idle_cycles_burned_per_day × freezing_threshold / 86400`).
Leave an additional upload budget: the 18 October fleet attempt spent roughly
0.70T cycles before reaching the existing 30-day reserve. Raw, gzip and Brotli
content are all uploaded; a 221 MB source bundle can transfer substantially more.
Do not reduce the freezing threshold to complete a release. Use an explicitly
approved `icp canister top-up frontend --amount <cycles> -e ic --identity <session>`
and re-run the normal sync after an insufficient-cycles rejection; a zero
`state_hash` means the release is not yet finalized. Confirm a nonzero final hash
and verify the served assets before declaring publication complete.
The official static-site sync plugin generates gzip and Brotli encodings. A fleet
refresh can exceed its default compute-time limit; on 18 October the complete
221 MB local editor bundle also exceeded a 600-second limit inside Brotli. Increase
only this CLI time allowance for a large sync, for example
`ICP_CLI_PLUGIN_COMPUTE_LIMIT_SECS=3600 icp sync frontend -e local`.
This does not change canister limits, runtime permissions or the served encodings.
Verify the custom origin with:

```sh
python3 scripts/verify.py portal --environment ic --origin https://www.subnet.ee
```

For a portal-only mainnet update, assemble the current portal and reuse its packaged
editors, then sync assets explicitly to `ic`:

```sh
python3 scripts/assemble.py
icp sync frontend -e ic --identity <current-owner-session>
python3 scripts/verify.py portal --environment ic
```

`python3 scripts/verify.py all --environment ic --timings` also downloads and verifies
all registered editor bundles, reporting compressed HTTP transfer sizes and durations.
These durations measure individual HTTP requests, not browser startup or cache hits.
The landing page does not preload editor bundles. Each editor is fetched when opened;
its fingerprinted JavaScript/Wasm uses a one-year immutable cache. HTML and unversioned
portal assets revalidate so updates are picked up. Cache rules must not combine
`no-cache` with the fingerprinted bundles' immutable policy.
