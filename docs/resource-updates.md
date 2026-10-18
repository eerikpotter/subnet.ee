# Resource-scoped cloud space updates

Requirement (2026-10-15): rename, move, Trash, permanent deletion, uploads and
other file actions must keep the current view usable, with progress on the affected
resource and only changed rows replaced. Preserve loaded pages and unrelated menus.

Status: published to mainnet on 16 October 2026, including inline file and folder
renaming. The validation notes below describe the earlier local proofs; the final
production release and checks are recorded in [history](history.md).

Skills retrieved 2026-10-15 before choosing the implementation:

- [canister-security](https://skills.internetcomputer.org/.well-known/skills/canister-security/SKILL.md)
- [static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
- [icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md)

The supported approach is the existing authenticated SDK, ordinary consensus
canister calls, server-side authorization and certified frontend hosting. The
existing organize receipt contains an ID/version, not a complete directory view.
Follow-up authenticated browse/list/Trash reads provide the current metadata and
permissions. This is sufficient; no transport, identity or canister changes are needed.

Observed gap: the UI used one global busy/loading state, rebuilt every file row
when any metadata changed, and restored identity/home after uploads and purges.
The smallest extension separates navigation, pagination, metadata refresh,
transfers and per-resource operations. Keyed rows retain unchanged DOM nodes;
only inserted, removed or changed entries alter the list. A confirmed mutation
revalidates all already-loaded pages silently. Quota refresh uses the existing SDK
account method through a small portal facade operation, without loading home again.

Alternatives: returning directory snapshots with every mutation would change the
backend contract and still need pagination/revocation handling. Optimistic metadata
writes would require rollback for version conflicts, lost responses and revoked
permissions. The current implementation waits for authoritative metadata while
retaining the visible list. It improves UI continuity, not canister round-trip latency.

Each read is guarded by navigation, read ordering, session and mutation revision.
Concurrent settlements invalidate earlier snapshots, including failed or ambiguous
mutations. A retry retains the original request token. An account change discards
pending results; navigation never returns to the operation's source folder. Initial
navigation may show loading, and a confirmed access revocation removes stale data.
Transient refresh failures leave the visible list intact and report the error.

Proof tests cover delayed rename/verification, row identity, unchanged menu/focus,
loaded pagination, simultaneous success/failure, late stale reads, move/Trash/restore,
new folders, cross-tab notifications, uploads, quota and account/navigation changes.
Browser validation checks the pending indicator, usable sibling actions and stable
layout. Local validation results and scope are recorded in development history.

No new dependencies, persistent browser state, permissions or editor protocol are
introduced. Cross-tab notifications are invalidation hints only. Production network
latency and Internet Identity issuance are outside the local UI proof.

## Filename interaction — 15 October 2026

Implemented and locally proven extension: a single click on an editable file name starts inline rename;
a double click keeps opening the file. Enter commits, Escape cancels, and leaving
an edited field commits. The follow-up requirement extends the same interaction
to editable folder names, with full-name selection even when the name contains a
dot. Read-only folder links and editor pickers keep their existing navigation.
Reuse the same name editor and double-click handler; no separate component is
needed. Proof additionally covers folder rename, double-click navigation and
read-only/picker restrictions. The same skills above and the existing authenticated
`organize` rename operation remain sufficient. The gap is only the name interaction.
Use one delayed click and an inline input, with the existing resource operation
and version checks; no platform or dependency changes. A dialog-only shortcut
would require an extra visual transition. Immediate replacement of the clicked
name would interfere with the second click, so the edit waits 500 ms and a double
click cancels it. Proof: single/double click, keyboard/blur/cancel, unchanged names,
failed saves, permission/picker restrictions, and stale timers after navigation.

Validation: 73 Node tests and three localization tests passed. Native browser QA
confirmed single-click inline editing, basename selection, Enter save, preservation
of unrelated rows and double-click preview. Proof image:
`.local/resource-browser/inline-rename.png`. Local frontend sync/HTTP verification
passed; production remains unchanged.

Folder extension validation: 75 Node tests and three localization tests passed.
Native browser QA confirmed full dotted-name selection, inline save with sibling
rows retained, and double-click navigation into the renamed folder. Screenshot:
`.local/resource-browser/folder-rename.png`. Local frontend sync and HTTP verification
passed; production remains unchanged.
