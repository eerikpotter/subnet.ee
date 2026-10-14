# Development history

[Project overview](../README.md) · [Development](development.md) · [Deployment](deployment.md)

## Homepage AI workflow illustration — 14 October 2026

Requirement: adapt the supplied `subnet_agent_workflow_et_v2/index.html` and
campaign image into the homepage between the tools and technology sections.
The tools heading now refers to the cloud drive; the new section demonstrates
reading a brief, understanding it, creating an image, saving it and opening it.

Guidance retrieved on 14 October 2026:
[static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
and [icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md),
selected from the current skill index. The existing `@dfinity/static-site@v0.4.0`
recipe and portal assembly already support the required HTML, CSS, JavaScript
and local image assets. No platform gap was observed. The smallest extension is
a scoped, dependency-free browser illustration using the existing translation
pipeline and brand mark. A separate iframe would complicate language switching
and responsive sizing; a live agent integration is outside this visual task.

The workflow has an accessible animation label and makes no backend or AI calls. It
supports pause/play, selecting steps, opening/closing the image preview, and
stops advancing outside the viewport or in a hidden tab. Reduced motion starts
with the finished image; the static HTML also shows that outcome without scripts.
Copy is provisional. The image retains the supplied artwork and English lettering.
Proof checks cover localized assembly, existing language tests and browser flows;
this illustration makes no claims about real agent authorization or persistence.

Validation: 45 existing JavaScript tests passed with `node --test
scripts/i18n.test.mjs scripts/cloud-i18n.test.mjs scripts/craft-shell.test.mjs
scripts/craft-loading.test.mjs`; the 3 localization and 9 scoped-workflow Python
checks passed. A disposable Node/VM smoke check covered elapsed playback, pause,
manual preview, reduced motion and viewport/tab suspension. Browser checks at
1280, 390 and 320 px covered layout, image loading, preview open/back, step
selection and a language switch preserving the selected stage; no console errors
were observed. Small viewports use a single seekable progress bar. Assembly
keeps all seven existing editor bundles and includes both localized homepages.

The attempted local `scripts/dev.py` sync was stopped during a long-running asset
sync; final certified-canister HTTP verification was not completed. Final visual
QA used the assembled static site. No mainnet publication was performed.

Copy refinement: the heading is followed by one sentence about MCP access and
collaboration in the user's sovereign cloud drive. The agent subtitle identifies
the MCP connection without listing a fixed set of capabilities. Removed the
eyebrow, example badge, duplicate cloud badge, preview caption and image footer;
the image uses the freed space. Updated both languages, removed unused styles
and translations, rebuilt both locales, passed the three localization checks and
verified the heading and open image in the local browser preview.

The demo file list now follows the existing drive's rows: filename, size/date,
`Ava` and the `···` menu. Removed the invented folder heading and table labels.
Open/Enter/double-click and the read actions in the menu work on the two local
sample files. The cursor follows the actual filename bounds, including wrapped
names, and shows a double-click before opening the image. Its motion uses the
same playback clock, so pause and resizing preserve the click target. The real
drive and its authenticated operations are unchanged.
Validated filename targeting at 1280 and 320 px (including a wrapped name),
double-click, Enter, both previews, menu opening/Escape and English metadata/actions.
All 27 existing i18n/drive JavaScript tests and 3 localization checks passed;
the assembled preview reported no browser errors.

Responsive layout: tablet windows remain side by side, with shorter prompt copy
and secondary descriptions/checklists hidden. On phones the compact agent panel
sits above a 260 px cloud panel, keeping both visible within a 478–495 px demo.
A single accessible seek slider replaces the six stage controls below 1100 px;
the current activity stays in the agent window. The slider pauses on seeking and
clamps its final position so the finished preview remains valid. Desktop retains
its original copy and stage buttons. Checked 320×568, 390×844, 768×1024,
1024×768 and 1280×960 layouts, both languages, slider endpoints/intermediate
steps, preview/back and cursor targeting; no horizontal overflow or browser
errors were observed. Localized assembly and the three localization tests pass.

## Public demo and custom-domain source publication — 13 October 2026

Committed the deployed custom-domain configuration, canonical cloud SDK and its
seven editor snapshots, public-demo copy and roadmap, workspace attribution and
EFTI footer mark. The SDK preserves the canister identity on the custom domain
and uses the certified public gateway settings.

Publication checks passed: 32 portal JavaScript tests, 16 Python workflow tests,
13 SDK tests, native SDK Clippy, the browser/Wasm SDK check, all seven snapshot
integrity checks, and secret scanning of the changed files. The custom-domain
HTTP verifier passed for the portal, shared assets, configuration and editor entry
pages. This run did not download every editor's Wasm bundle or repeat sign-in.
Full editor CI passed formatting but stopped at Clippy errors in unchanged
upstream crates; it is not a fully green editor-wide CI result.

## Archive: 10–13 October 2026

The notes below were moved from the project README on 13 October 2026.
They preserve the sequence of integration milestones, earlier design details and
validation results. References to four or five editors, branch locations, CI
failures and deployment state describe those milestones, not the current checkout.
Use the development and deployment guides for current working instructions.

### Initial local validation — 10 October 2026

Initial validation (2026-10-10): all four release Wasm builds succeeded. 25 focused
tests passed: SDK configuration/transfer (8), backend unit tests (5), PocketIC
persistence/upgrade tests (4), and PhotoCraft web-host unit tests (8). Clippy passed
for the SDK (native and Wasm), backend and PhotoCraft web host; PhotoCraft's layer
check passed for all 27 crates. The deployed local site passed `scripts/verify.py`,
including exact asset bytes, certification-header presence, MIME/cache/CSP rules,
runtime trust configuration and missing-file responses.

All four editors were opened from the portal in the browser without console errors;
PhotoCraft's Sovereign Cloud sign-in panel rendered. The portal was checked at desktop
and 390px mobile width without horizontal overflow. At that initial milestone, authenticated
browser save/reopen had not yet been exercised in this new origin; see the shared-account
validation below. Editor-wide functionality
and mobile editor ergonomics are outside this setup's smoke check.

## Shared account and PhotoCraft flow (0.2)

Sign in on the portal, open PhotoCraft and use **File → Sovereign Cloud → Save**. The same
account's committed files appear under **Minu failid**. Opening a row passes a logical
service/space/file reference to PhotoCraft; the backend rechecks ownership and the SDK
checks file integrity. Further saves keep the file ID and add revisions. The catalog
only exposes committed files and reads via consensus-backed updates.

The File menu has one **Sovereign Cloud** section after the local save block: **Save**,
**Save As…**, and **Open…**. One ICP mark appears beside the section heading. Save As
creates a new cloud file and links subsequent cloud saves to it. Open shows the file picker
without leaving the document. There is no separate cloud toolbar button, and the cloud
dialog uses a compact header.

`get_or_create_account`, `list_files` and `get_file` extend the existing API. Accounts
and their file index use new stable-memory IDs 6 and 7. V1 schemas/IDs 0–5 remain intact.
A legacy owner's catalog is indexed once when their account is first resolved (the
old store is bounded to 1000 files). New files update that same index. The account and
personal-space IDs are stable within the logical `cloud` service. Existing file bytes
are not copied. V1 files remain PhotoCraft format version 1; the typed create API described below
adds the other editors. Multi-blob revisions remain future work.

The portal and PhotoCraft share the pinned Rust AuthClient storage adapter. A loaded
session carries a fingerprint of its persisted credentials. Each SDK network call
rejects changed/expired sessions; UI responses are scoped to their originating session.
Cross-tab changes restore identity without reloading the editor. Open documents retain
their original account binding; saving one to another account requires **Sovereign Cloud → Save As**.
Already submitted calls may complete, and logout does not revoke previously issued
IC delegations at the replica. This is one trusted first-party origin, not per-app isolation.

The facade is built separately from the editors. `scripts/verify.py` verifies its served
Wasm/JS too and rejects the temporary browser fixtures. `scripts/prepare-browser-test.py`
creates opt-in loopback-only fixtures with disposable direct-signing identities for UI
QA. Use an isolated canister hostname, never a real user's origin. They are excluded from
normal assembly and must be removed by reassembling/syncing before final delivery. These
fixtures exercise real signed canister calls, but do not test Internet Identity issuance.

Shared-account validation (2026-10-10): SDK tests (9), service tests (6), PocketIC
persistence/upgrade tests (4), standalone backend tests (6), and PhotoCraft host tests (8)
pass. Clippy passes for the SDK, service, portal facade and Wasm PhotoCraft host. All 27
PhotoCraft dependency-layer checks pass; release builds pass for the changed Wasm packages.
The old-Wasm upgrade test preserves file IDs and content, indexes legacy files, appends
a revision and denies another caller access.

Browser QA used two disposable signing identities on an isolated local canister hostname.
A painted PhotoCraft document appeared in the portal, reopened with the same content via
its catalog link, and saved a second revision under the same file ID. Switching accounts
cleared file lists without reloading or losing the document; saving that document to the
other account required an explicit copy. Portal logout propagated to PhotoCraft. Corrupt
persisted delegation JSON recovered to a signed-out state without a browser error. The
file list was visually checked at desktop and 390px width. Real Internet Identity/passkey
issuance and cross-device use remain interactive validation steps; the browser driver did
not expose the provider popup. No mainnet deployment was made.

The final local deployment passed `scripts/verify.py` for the portal and all four editors,
including exact served bytes, runtime configuration, response headers and 404 responses for
both removed browser fixtures. The regular frontend origin was reopened in its signed-out
**Minu failid** view after cleanup.

## Shared Craft navigation

Every `/apps/<id>/` entry point uses the same 40px subnet.ee bar: **← subnet.ee**,
the application's name, and **Minu failid**. `portal/craft-shell.html`, `.css` and
`.js` own the navigation. Assembly renders the name from `registry/apps.json` and
moves each original Trunk entry point to `editor.html` beside its unchanged assets.
A same-origin iframe gives the editor the remaining viewport without covering its
menus. Launch query parameters and fragments are forwarded, including cloud-file
references and renderer options. No other editor is preloaded.

The browser host in each Rust app publishes `data-craft-dirty="true"` or `"false"`
on its document element after both logic and UI updates. PhotoCraft, PdfCraft and DeckCraft
aggregate all open documents; WordCraft reports its session; SoundCraft includes
both dirty state and active recording. This tiny host contract exposes no document
or account data and adds no dependency on the portal to standalone builds.

The common shell reads that state synchronously before navigation. Clean documents
leave immediately; unsaved work opens an Estonian confirmation with **Jätka tööd**
and **Lahku salvestamata**. Cancel/Escape keeps the editor loaded. The shell also
requests the browser's native warning on reload, Back, or tab closure when dirty;
browser restrictions still apply to native unload prompts. Missing/unknown state
uses a conservative save reminder. Explicitly opening a portal link in another tab
does not interrupt the document. The dialog does not claim to save files itself.

To integrate another Craft, register its metadata/build and publish the same dirty
attribute from its web host. The navigation, layout and guard are shared unchanged.
After host changes rebuild the affected apps; shell-only edits need reassembly.

Navigation checks: `node --test scripts/craft-shell.test.mjs` covers both destinations,
cancel/confirm, fresh save/edit state, unknown state, modifier clicks, history restore,
and launch references. `python3 scripts/verify.py` checks served shell assets, editor
entry points, original Wasm/JS assets, CSP and runtime configuration.

File-menu refinement validation (2026-10-10): all 8 web-host tests pass, including actual
menu activation, placement and document guards. The shared UI suite passed 497 tests in
the sandbox; its one GPU-dependent test passed separately with GPU access (3 opt-in tests
ignored). All 27 layer checks, 21 Wasm crate checks and strict Wasm host clippy pass. Native
UI/web all-target clippy passes with the previously documented `collapsible_match` lint
excluded. The release build and local deployment verifier pass. Browser inspection confirmed
the three small menu marks, removed toolbar launcher and all three operation titles in the
signed-out flow; the console had no errors. No cloud upload was needed for this UI change.

Navigation validation (2026-10-10): all four release Wasm builds and scoped web-host
Clippy checks pass, along with 7 shared-shell tests, 8 PhotoCraft web-host tests and
the 27-crate PhotoCraft layer check. Browser QA exercised a painted PhotoCraft image,
a typed WordCraft document, a new PdfCraft PDF and a modified SoundCraft demo. Each
reported dirty state and showed the shared leave confirmation; Cancel preserved the
editor. PhotoCraft local Save cleared the guard and allowed immediate navigation to
My files. Escape, explicit discard, clean navigation, and the 390px shell/dialog
layout were also checked. No mainnet deployment was made.

Cloud-section validation (2026-10-10): 8 web-host tests and 498 UI tests passed (3 opt-in
tests ignored), including keyboard navigation past the heading and a hidden first action.
Native clippy passed with the existing collapsible_match lint excluded; cloud-host Wasm
clippy, all 27 layering checks and 21 Wasm crate checks passed. Release build, local frontend
sync and full served-asset verification passed. Browser verification confirmed the single
section/mark, disabled save actions without a document, enabled actions with a blank document,
and the signed-out Open dialog. No browser console errors or cloud file writes.

The final managed-local deployment passed `scripts/verify.py` for the shared shell,
all four wrapped entry points, their original editor pages, and exact Wasm/JS bytes.
The final PhotoCraft page also rendered on the certified local origin with a clean
dirty-state report. Concurrent asset syncs required rechecking the final served
bundle; the completed server contents match `dist/`.

## DeckCraft

DeckCraft is registered at `/apps/deckcraft/`, with the same shared navigation and
leave guard. Its browser host reports unsaved changes across all open presentations.
The standalone editor's file picker/download flow supports `.deckcraft`, PPTX and
PDF export; Sovereign Cloud stores the editable `.deckcraft` format. The upstream sample
opens by default, and `?blank` starts a new presentation.

`python3 scripts/build.py deckcraft --assemble` builds just DeckCraft and reuses the
other editor outputs. The build selection and portal counts come from
`registry/apps.json`, so adding an editor does not require updating fixed counts.

DeckCraft's ordinary browser download/save path is unchanged from upstream. A local
save-state fix was removed to keep this fork focused on integration. Cloud completion
marks only its captured, unchanged presentation clean. Formatting, asset audit and
the 27-crate layer check pass. Upstream's full baseline CI stops at the existing
`collapsible_match` lint in `deckcraft-matroska`; that unrelated codec is unchanged.

## Shared cloud in all Craft editors

All five web hosts build with `icp-cloud` and vendor the same SDK and UI sources.
Use File → Sovereign Cloud → Save, Save As… or Open…. WordCraft exposes the section
as a File backstage page. My files routes each format/version to its compatible editor.
PhotoCraft stores `.pcraft`, PdfCraft `.pdf`, WordCraft `.wcraft.json`, DeckCraft
`.deckcraft`, and SoundCraft a portable `.scraft.zip` containing session metadata and
lossless planar float sources. Linked videos are rejected by SoundCraft's portable writer instead
of silently dropping media. The service's existing 64 MiB file limit still applies.

Metadata for new formats occupies stable memory 8; memory IDs 0–7 and existing file
bytes are unchanged. Missing kind metadata defaults to the legacy PhotoCraft format.
The old create/list API remains PhotoCraft-compatible; new typed create/list methods
serve the other editors. Ownership is still enforced by the caller, independently of
format metadata. Upload completion marks only the captured, unchanged revision clean.
Single-document editors confirm before replacing unsaved work with a cloud file.

Validation for the shared cloud implementation: SDK transfer/configuration tests,
shared UI tests, service unit tests and isolated PocketIC persistence/upgrade tests
pass. Native format tests preserve a WordCraft document with an inserted picture
and passthrough XML; SoundCraft tests preserve the editable demo and exact float
headroom. New/Open document identities are distinct from edits. Strict SDK, shared
UI (native/Wasm), service and SoundCraft engine Clippy checks pass. WordCraft engine
Clippy exposes a pre-existing `overly_complex_bool_expr` in `cmd/para.rs:277`; it passes
with only that existing lint excluded. Full workspace CI is not claimed.

WordCraft's media-preserving JSON codec is used only by its cloud adapter; ordinary
local JSON open/save remains upstream-owned. SoundCraft's portable bundle is likewise
a separate cloud format, without replacing upstream local session saves.

Browser checks on the isolated local QA origin saved and reopened a DeckCraft sample,
a WordCraft document, a blank PdfCraft document and SoundCraft's synthesized demo
(63.1 MB including its audio). Each appeared in the shared catalog with the matching
editor link, while in-app pickers only listed compatible formats. WordCraft's second
save kept the same file ID and produced revision 2; cancelling replacement of unsaved
text kept the edits, and successful save cleared the shared navigation warning.
These use disposable test identities and real calls to the local replica, not real
Internet Identity/passkey issuance or mainnet.

## Reviewed PhotoCraft update

PhotoCraft's merge `b2fd7337` incorporates upstream `7722172585a0` (0.6.0).
The follow-up includes upstream `2b79c7c25dca` (nine additional test lines) and
the fork's existing `main` merge, preserving both histories.
The update contained 887 upstream commits and 11 conflicted files. Conflicts included
manifests/lockfile, attribution, README, startup and menu plumbing, rather than 11 new
cloud features. Dependency versions from the incoming lockfile were preserved; cloud-only
dependencies were added. The generic menu hook now adds only 10 lines to upstream's
`menus.rs`; its native service initializer and layer checker match upstream exactly.
The service commands and submenu heading renderer live in a separate module. Cloud UI
and transport remain shared, with generated snapshots for independently buildable forks.

The merge passed 13 web-host tests, the host-command guard test, Wasm checks with
`icp-cloud,heif`, and the 29-crate layer check. Scoped Wasm Clippy passes with upstream's
existing `collapsible_match` and `nonminimal_bool` lints excluded. Strict Clippy reports
seven instances of those lints in unchanged upstream code; this is not a full green
upstream CI run. The updater's five offline integration tests pass, including a conflict
that leaves the working app untouched and exact-commit promotion with a backup ref.

The other upstream updates have only been fetched for inspection; they were not merged.
Future reviews should normally select one app at a time. Fetching does not build or deploy
anything. Neither a small diff nor passing tests establishes that upstream code is safe;
see [the update guide](../scripts/UPDATING.md) before running an unfamiliar build.

### CADCraft and GridCraft cloud adapters

The catalog now includes CADCraft (drawings) and GridCraft (spreadsheets). Both use
our shared shell, File → Sovereign Cloud menu, account and versioned file service.
Cloud Save / Save As / Open use upstream's DXF codec (`cadcraft.dxf`, version 1)
and complete JSON workbook codec (`gridcraft.scjson`, version 1), respectively.
Ordinary local saves and exports remain upstream-owned. Cloud files reopen in new
tabs, preserving existing work. GridCraft commits/validates a cell still being typed
before requesting a cloud save. An upload only marks its exact document revision
clean; newer edits and unfinished text remain protected by the navigation guard.

The fork integration is confined to the browser host and an optional File-menu
callback; the storage protocol and UI are generated vendor snapshots. Both apps
are included in `scripts/update-crafts.py`. Rebuild these two apps with
`python3 scripts/build.py cadcraft gridcraft --assemble`; other editor outputs are reused.

## Seven-editor upstream refresh — 12 October 2026

Upstream refresh (2026-10-12): the fork merges retain both upstream history and the
existing subnet integration. Reviewed upstream tips were PhotoCraft `bb17dd555c16`,
PdfCraft `87b448b664e1`, WordCraft `fc52079f587a`, DeckCraft `0ec0cc5e815c`,
SoundCraft `df0c4790b729`, CADCraft `4cb9e8a535b2` and GridCraft `b3acae6b7782`.
GridCraft's cloud actions now live in its new Backstage view. WordCraft uses upstream's
document identity API; SoundCraft keeps its portable cloud format alongside upstream's
new local file handling. CADCraft and PdfCraft retain their new upstream host bridges.
All seven forks retain the same pinned Sovereign Cloud SDK and UI source snapshots.
All seven release Wasm builds pass. The 25 focused tests pass: PhotoCraft web-host/menu
tests (13), WordCraft native-format/document identity (3), SoundCraft portable audio
format (5), CADCraft cloud adapter (1), and GridCraft cloud adapter/Backstage wiring (3).
All seven browser cloud save/open checks pass using a disposable identity on the local
test network, including edited image, text, spreadsheet, CAD and PDF content and the
slide/audio demos. No browser console errors were observed. Temporary session fixtures
and their generated redirects were removed; `python3 scripts/verify.py all` passes for
the final local bundle, including exact asset payloads, certification, cache/CSP headers,
runtime trust and missing-file responses. These checks do not cover Internet Identity
issuance or constitute a deployment of this upstream refresh to mainnet.

Strict editor-wide CI is not green on Rust 1.95.0: it stops at upstream Clippy warnings
in CADCraft `geom/nurbs.rs`, GridCraft's XLSX code/tests, DeckCraft's ISOBMFF codec,
WordCraft `proof/affix.rs`, SoundCraft's video codec and PdfCraft `forms/af.rs`.
PhotoCraft's scoped Wasm Clippy check likewise stops at `drop_non_drop` in upstream
`format/store.rs` when checking the browser target.
These files are unchanged by the cloud integration; the refresh does not suppress
their lints or modify unrelated editor behavior. PhotoCraft's 29-crate layer check passes.

## Mainnet trial — 12 October 2026

The mainnet trial uses two new canisters, separate from all local data and existing
standalone Craft deployments:

* Frontend: `vigkv-fqaaa-aaaac-bgeha-cai`, https://vigkv-fqaaa-aaaac-bgeha-cai.icp.net/
* Accounts/files: `vbfbj-tyaaa-aaaac-bgegq-cai`.

Both are controlled by the existing `efti-nns` identity. Mainnet mappings are in
`.icp/data/mappings/ic.ids.json`; retain them so later updates reuse these canisters.
This trial does not configure the `subnet.ee` custom domain.

The initial mainnet verification passed for all seven editors, including payload
hashes, certified responses, runtime canister IDs/root key, cache headers and 404s.
PdfCraft and SoundCraft also started in the browser. Initial gzip Wasm transfers
were 6.1–12.7 MB and took 3.6–24.8 seconds in one run; these are variable HTTP
measurements, not complete browser startup times. A separate WordCraft Brotli
request transferred 7.4 MB in 7.3 seconds; its conditional repeat returned 304
with no body. The landing page loads no editor Wasm.

## Branch milestones recorded before this split

The current portal and all seven app integrations are published on each fork's `main`.
Earlier integration milestones remain on `codex/shared-account-files` in the root and
PhotoCraft, and `codex/sovereign-cloud` in the other six forks.
The root remote is `https://github.com/eerikpotter/subnet.ee.git`.
The first extraction remains on `codex/subnet-cloud-base`.

## Future work recorded at the time

All seven editors shared cloud storage and compatible format metadata. Shared
spaces, per-app grants, user-owned canisters, encrypted storage, alternate blob
stores and Cloud Engine migration remained later work.

## Homepage AI indicator — 14 October 2026

Published the approved `+ AI` badge and person-with-spark icon beside “Sinu
pilvefailid” using `icp sync frontend -e ic --identity efti-nns`. The release reused
the tested preview bundle: the Estonian and English homepages and `landing.css`
differed from production only by this visual change. The backend was not deployed.
The frontend reported 169 assets and state hash
`c3d736b903f96024ceeb0a9da608c26e5c9ba7b19e3d09cb8786b19cfc17263b`.

`python3 scripts/verify.py portal --environment ic --origin https://www.subnet.ee`
passed exact payload, certification, runtime identity, MCP discovery, cache/CSP
and missing-path checks. The badge was also verified visually at the public origin.


## 2026-10-14 — Shared cloud folders in all seven Craft apps

Open and Save As now embed the same portal drive, with personal/shared folders and
editor-appropriate light/dark styling. Save uses the canonical drive revision;
Create and Edit remain independent permissions. Read-only files open normally and
can be copied where Create is granted. Account changes, permission failures and
version conflicts preserve drafts. Native formats and editor engines are unchanged.
A WordCraft cold-start font race found during direct-file testing was fixed in the
shared UI, with a regression test.

Scoped SDK/UI/API checks and all seven release builds passed. Disposable local
accounts exercised shared native save/reopen in WordCraft, PhotoCraft, PdfCraft,
DeckCraft, SoundCraft (63.1 MB with audio), CADCraft and GridCraft, plus shared edits,
read-only Save rejection, Save As, account switching and quota-error recovery.
Mainnet frontend state is
`c1547dc5c8725b8decb7526eb06bfe59e4568deb26012b434e7286b15f15c99f`.
All seven published bundles passed exact-payload and deployment-configuration
verification. WordCraft startup and signed-out Open were also checked in the
production browser. Existing published landing pages were preserved; another
chat's homepage demo was excluded. No backend deployment or storage migration.
See the full evidence in [collaboration progress](collaboration-progress.md).

## Shared cloud MVP publication — 15 October 2026

Prepared all accumulated shared-drive/backend/SDK/Craft changes and the illustrative
landing-page agent workflow for publication together. The narrowed demo scope and
known limits are recorded in collaboration-spec.md and collaboration-progress.md.
Unfinished-upload administration was removed from the ordinary drive; automatic
background cleanup remains deferred.

The review's 126 checks passed, including seven actual-canister persistence/migration
tests; final portal edits passed 27 JavaScript and three localization checks. Before
publication, all 50 portal/shell JavaScript tests and 19 Python tooling tests passed
again, as did strict cloud UI/backend Wasm Clippy and all seven SDK snapshot checks.
The editor JS/Wasm bytes match the seven-app native browser-tested release.

The six app-wide `xtask ci` runs stop at the same documented unchanged upstream
Clippy warnings (Word proof, Deck codecs, Sound video, CAD geometry, Grid XLSX, PDF
forms). Their fmt stages pass; PhotoCraft's layering check passes. This release
does not claim fully green editor-wide CI or suppress those upstream lints.
