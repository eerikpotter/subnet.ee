# FigCraft integration

Decision date: 2026-10-15. Status: implemented; local validation recorded below.

## Requirement and evidence

Add `eerikpotter/figcraft` as a pinned Craft editor, with matching Estonian/English
catalog copy, shared navigation, and cloud Open, Save and Save As. Preserve native
designs, embedded images, versions and branches, unsaved work, and account isolation.

ICP skills retrieved on 2026-10-15 before implementation:

- [Internet Identity](https://skills.internetcomputer.org/.well-known/skills/internet-identity/SKILL.md)
  and its [shared-session reference](https://skills.internetcomputer.org/.well-known/skills/internet-identity/references/shared-sessions.md).
- [Static site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md).
- [ICP CLI](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md)
  and its [environment reference](https://skills.internetcomputer.org/.well-known/skills/icp-cli/references/canister-env-vars.md).
- [Stable memory](https://skills.internetcomputer.org/.well-known/skills/stable-memory/SKILL.md).
- [Canister security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md).

The supported approach already used here is certified static hosting, Internet
Identity through the canonical Rust session adapter, ordinary authenticated drive
canister calls and stable structures. FigCraft can reuse all of it on the same
origin. No platform gap was found. The application gaps are a missing format
registration, browser cloud adapter, File-menu extension and launcher dirty state.

The smallest extension registers `figcraft.figcraft`, version 1, vendors the shared
SDK/UI into the fork, and adapts the existing native archive codec. The adapter
captures immutable full-file content, resolves branch tabs to their main file,
preserves other tabs when opening, and only marks an unchanged snapshot saved.
The upstream engine and native codec stay unchanged. Open branch content and a
combined change token are handled within the web adapter. The only shared editor
UI extension is an optional host callback in the existing File menu.
The web host scopes FigCraft's existing `theme::on_dark` helper around the shared
menu controls, because FigCraft keeps that menu dark even in its light theme.
Shared folders, conflicts, permissions and account transitions stay in the existing
SDK/UI/service. No stable schema, memory ID or Candid method changes are required.

A separate component review found no need for app-specific identity, dialogs,
transport or storage. The native archive adapter and small host callback are the
only new runtime components; both are needed to connect the existing systems.
The other seven editor snapshots remain pinned to their existing SDK versions.

Using plain document JSON would omit archive assets and history; a separate storage
service or FigCraft collaboration server would duplicate account and permission
policy. Those alternatives are deferred. This integration provides stored file
revisions and shared-folder access, not new real-time multiplayer infrastructure.

Proof checks: native archive round trips including images/history/branches; changes
during upload; invalid-open preservation; tab identity; shared format isolation and
upgrade persistence; release Wasm build; localized catalog rendering; actual browser
save/reopen against a local canister. Local disposable identities do not verify
Internet Identity issuance or production trust guarantees.

FigCraft's referenced `storytold/craftrules/AGENTS.md` was unavailable (HTTP 404) and
there is no local checkout. The fork's own AGENTS.md remains applicable. No unchecked
platform limitation is inferred from that missing contributor reference.

## Validation

- `python3 scripts/sync-sdk.py figcraft --check`: both snapshots match canonical sources.
- `cargo test -p figcraft-web --features icp-cloud`: four adapter tests passed,
  including embedded PNG bytes, main/branch versions, open tabs, edits during
  upload, invalid input and active edit protection.
- `cargo test -p figcraft-ui-egui host_menu_tests`: host actions appear with and
  without an open design and dismiss the native File menu.
- Focused Wasm Clippy for `figcraft-web --features icp-cloud --no-deps -- -D warnings`
  passed, with an existing toolchain warning for an unknown upstream lint.
- FigCraft formatting, layer, asset and `cargo xtask wasm` checks passed (including
  the standalone web build and all three example plugins). The complete `cargo xtask ci`
  stopped on existing `clippy::nonminimal_bool` errors in unchanged
  `crates/format/src/fig/export.rs` (lines 1714 and 2194); full CI is not green.
- Shared SDK: 6 unit and 10 transfer tests; shared UI: 10 tests; service: 24 tests.
- Four PocketIC tests passed using a freshly built service Wasm and the legacy
  baseline: upgrade persistence, lost replies, all eight registered formats,
  account isolation, shared-folder permissions and conflicting revisions.
- Portal JavaScript/i18n/shell tests passed; the cache/loading/Wasm suite passed
  25 tests, including registry/cache consistency. Python scoped workflow (10),
  localization (3), update workflow (5) and release packaging (1) tests passed.
- `python3 scripts/build.py figcraft` and `python3 scripts/build.py portal` passed.
  The local backend was upgraded with the new format registration. No stable
  schema or Candid changes were made.

- Actual local browser: both catalog languages rendered correctly; created a
  rectangle, saved a native file using the shared picker, observed clean state,
  and reopened the rectangle through the My files launch link. Opening added a
  tab and preserved the initial blank design. Account B listed no files and a
  direct link to account A's design returned "Not found or access unavailable".
  The usual local origin also reached ready on its first FigCraft launch. The
  isolated QA origin needed one initial retry; later cached launches succeeded.
- Browser testing caught the light-theme/dark-menu text contrast mismatch. The
  host now uses the editor's existing dark-control styling; the corrected release
  build and focused Wasm Clippy passed.

- Final browser verification confirmed readable cloud actions in the light theme
  using FigCraft's dark menu. The standard version dialog downloaded and opened
  the corrected package without changing the running editor before acceptance.
- `python3 scripts/verify.py figcraft portal --timings` passed exact deployed
  assets, certification, CSP, runtime configuration, cache headers and missing
  paths. Temporary login fixtures were removed, and the disposable browser
  session was signed out. Screenshots are in `.local/figcraft-qa/`.
- The Wasm sync plugin first exceeded its default 60-second compression limit;
  its raised-limit attempt remained CPU-bound without progress. The existing
  native adapter around DFINITY v0.4.0 `sync-core` completed both local syncs.
  No new deployment tool, dependency or hosting approach was introduced.

Logs are in `.local/figcraft-*.log`. No production deployment is part of these checks.
