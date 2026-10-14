# Shared cloud drive alpha specification

Active product contract, revised 15 October 2026 following the user's explicit
decision to close the current demo MVP without adding more features.
[Progress and test evidence](collaboration-progress.md) records the actual state.

The MVP is the working shared drive, the seven Craft apps' shared Open/Save/Save As,
account-based permissions, recoverable Trash/content history and standard ICP MCP.
Proposals, search, general copy, automatic cleanup and a complete activity log are
outside this demo. Keep the implemented Trash rather than removing it. Fix defects
in delivered workflows; do not turn the remaining product backlog into new MVP gates.
Known UI/recovery limitations below remain explicit, not claims of completeness.

The alpha uses the existing ICP MCP connector and the signed-in user's app identity.
**Connected AI has the same application permissions as that user.** Separate agent
permissions and trustworthy agent attribution are deferred. The user accepts this
limitation for the alpha demo, including the risk of destructive authorized actions.
Do not represent the earlier scoped-agent design as an alpha security guarantee.

The [future agent design](collaboration-future-agent-design.md) preserves the fuller
permission model for later work. It does not override this alpha contract or require
custom OAuth, an application MCP host, agent key custody, or a confidential subnet
for this release. Native semantic editing of Craft projects also remains a separate
integration milestone.

## Architecture

Start every ICP design and implementation change with the relevant current skills,
as required by [AGENTS.md](../AGENTS.md). The supported route is:

AI client → existing ICP MCP connector → signed Candid calls → cloud service.

The service owns all permission and storage decisions. The canonical Rust SDK and
shared cloud UI/controller serve the portal and every Craft web popup. MCP invokes
the same service methods; it is not another permission database or a separate file
store. A client-supplied user, app, provider, model or `is_agent` field confers no
identity or authority.

Keep a server-resolved actor context so a later supported agent identity can use
this same evaluator. For alpha, it resolves only to the authenticated user principal.
A future restricted connection must not fall back to unrestricted user authority.
Do not advertise or expose agent-grant controls before that binding is implemented.

Preserve existing canister IDs, II derivation origin, legacy file IDs and stable-memory
schemas. An app principal is not an NNS identity or a canister controller. No cloud
connection receives controller rights. Production deployment is a separate action.

ICP MCP discovery requires accurate Candid metadata, `get_api_doc`, the correct II
origin and an operator-approved `/.well-known/ic-architecture` manifest. Publishing
that manifest accepts DFINITY's App Operator Terms; obtain the explicit operator
confirmation required by the service-discoverability skill before creating or
publishing it. Do not silently enable participation during an ordinary build.

## User experience

The product is a shared drive: work on files, with others, and with AI. There is no
Team or Organization setup. A shared folder remains the same folder, not a copy or
a second storage location. Space is an internal ownership/quota boundary with one
owner; collaborators' new files belong to the destination space.

Use three clear views: **My files**, **Shared with me**, and **Trash**. Shared with
me shows directly shared roots and lets users navigate inherited descendants without
exposing private ancestor names or siblings. The views themselves are not folders.

The file browser supports breadcrumbs, folders, file-kind icons, useful empty/error
states, upload/download, rename, move, Trash, restore, sharing and history.
Stable file IDs keep links and open editors
working through rename/move. No drag-and-drop-only operation: every action must be
available from an accessible menu or dialog.

Use one primary action per dialog, concise copy, keyboard navigation, visible focus,
Escape/cancel, predictable loading states and useful conflict recovery. Return focus
to the initiating item. Do not clear drafts or close a save dialog after a failed
request. The portal and embedded picker use the existing Estonian/English system
without translating user filenames or document content. Some shared Craft status,
retry and unsaved-work dialogs remain English in this demo; full localization and
broader accessibility testing remain follow-up polish.

The Craft popup uses the same navigation/controller and backend rules as the portal.
Its colors and typography may match its app. It must not reimplement sharing, scope,
version conflicts or authorization. Show the destination before saving to a
shared folder; an explicit owner label remains a known UX follow-up.
Hide or explain unavailable actions based on service-returned rights;
the backend still checks every request.

## Permissions

Four usable capabilities apply to users and to their AI's actions in this MVP:

- **Read**: current metadata/content, authorized listings and downloads. It does not
  reveal all history, access rosters or private ancestors.
- **Create**: create a new folder or publish a new file within a folder. It does not
  authorize overwriting, editing, renaming or deleting an existing item, including
  one the creator previously created. A collision never means overwrite.
- **Edit**: edit or rename existing items, read retained history and restore old
  content as a new revision. It does not imply Create or Delete.
- **Delete**: move an item to Trash and restore its retained location. A grant on
  a child does not authorize trashing its parent. Permanent deletion is a separate
  ownership action, never implied by this capability.

Read is required for every other capability. Create applies only to folders.
The portal exposes Create, Edit and Delete as separate options in addition to Read.
The existing Propose field is reserved for later use and must not be presented as
a working capability. Do not remove stable fields or change Candid compatibility
merely because the proposal workflow is deferred.

Ownership supplies administration and all file capabilities. Only the owner manages
human sharing and storage administration. No collaborator re-sharing, ownership
transfer or team administration in alpha. An owner using MCP has these same powers;
there is no verified human-only administration channel.

Direct grants add access. Effective rights are the union of current, unexpired
applicable grants on the object and its ancestors. There are no deny rules or hidden
restricted children inside a shared folder. Moving an item outside an inherited
scope can remove access; moving it into a shared folder can add access. An explicit
move-access preview remains a UX follow-up; never expose identities/locations the
caller cannot inspect.

Known sharing UI limitation: adding another grant does not replace an earlier one.
To narrow direct access in the current portal, remove the older grant and add the
narrower grant; inherited rights still apply. The backend already supports versioned
grant edits. A clearer per-recipient editor and effective-rights display are follow-up
work. Do not describe a newly added narrower grant as revoking existing access.

Each grant has a stable ID, subject, root, explicit capabilities, status, optional
expiry and version. Recheck current rights on every operation, including commit,
restore and legacy endpoints. Missing and inaccessible objects
have indistinguishable external errors. A link is a reference, never a bearer grant.

## AI connection and attribution

The Connect AI guidance states prominently:

> Alpha: your AI acts using your subnet.ee account and has the same permissions
> as you, including access to shared files and any owner actions you can perform.
> Separate AI permissions are not available yet.

Direct users to the supported ICP MCP connection instructions and Internet Identity
consent. Do not collect AI-provider API keys, create a subnet.ee OAuth service or
suggest that selecting a folder in a chat limits the credential's authority.

Alice's share to Bob applies to calls authenticated as Bob, including his AI.
Reducing/revoking Bob's share takes effect on subsequent service authorization
checks. It cannot retract bytes already downloaded. Disconnecting ICP MCP through
II prevents new delegations; already issued delegations may remain valid until
expiry. Do not promise an instant per-agent kill switch in this alpha.

Content history records the authenticated account, timestamp and file/revision
with byte size and digest. Expanded operation/base/proposal lineage and a separate
organization activity log are deferred. The operator kind is
**unknown** for ordinary app-principal calls. Display “Bob” or “Bob's account,” not
“Bob manually edited” or “Claude edited.” Unverified client labels may never replace
that attribution or be used for permissions.

A browser dialog or fresh login alone is not proof that a human operated an account.
A verified human-only approval ceremony is not part of this alpha.

## File and folder operations

Persist an object tree with stable IDs, one owning space, parent, name, object kind,
metadata version, timestamps and Trash state. Content revisions are separate from
location/name, so saving content must never undo another user's rename or move.

Names are Unicode NFC, trimmed, bounded to 160 UTF-8 bytes, and exclude control
characters, slash/backslash and `.`/`..`. Compare normalized names consistently for
sibling collisions; specify and test the chosen case policy in the API docs. Preserve
the displayed spelling. Do not let client platform path conventions determine IDs.

Rename requires Read + Edit. Move requires Read + Edit + Delete throughout the
source subtree and Read + Create on the destination. Moves are atomic, within one
owning space, and reject roots, self/descendant destinations and depth violations.
A same-parent move is a no-op. General file/folder copy is deferred; Craft Save As
already creates a new native document with the destination's ownership/inheritance.

Folder Trash/restore checks Read + Delete throughout the subtree. Keep the original
parent and identity, and distinguish descendants independently trashed before their
parent. Restoring the parent must not revive those descendants. The root cannot be
trashed. A restore destination must be live. The current operation restores only
to the original parent and rejects name conflicts. Restore the parent or resolve
the live name collision first; alternate-location/rename-on-restore UI is deferred.

Normal reads/writes reject a trashed object or trashed ancestor. Dedicated Trash
views disclose only currently authorized recovery entries. Permission changes while
an item is trashed still apply to restoration. Restoring content never restores old
grants or silently replaces an occupied destination.

Use optimistic concurrency for metadata, content heads and grants.
Reauthorize at execution and reject stale versions before mutation. Folder operations
must have bounded subtree validation and no partial success. Every retriable mutation
uses a scoped request ID and payload digest: identical retries return the original
result, changed payloads conflict, and a retry must not bypass newly revoked access.

## Content and recovery

Store ordinary binary files and Craft projects with media type, size, digest and
format metadata. Cloud storage support does not imply semantic editing or preview
support for every format. MCP can create text, images and other bytes; it cannot
safely edit a native Craft format just by renaming Markdown or JSON.

Use staged, resumable uploads for large files. Reserve quota before chunks, enforce
chunk count/size and whole-file SHA-256, then atomically publish one immutable revision.
Recheck destination rights, file status and expected head at commit. Downloads read
a fixed revision with verified size/hash; interrupted operations never publish a
partial document. Empty files are supported. Listing is paginated and sorted
consistently, with no unauthorized counts or cross-space cursors.

Retained history supports preview/download where authorized and restore as a new
revision. Restore never erases intervening history or rolls permissions back.
There is no complete activity log in this demo.

Trash is the normal deletion experience. Do not silently prune published history
when storage fills: stop writes with a useful quota/revision-limit error. Incomplete
uploads stop accepting writes after 24 hours. Unfinished chunks are implementation
details, not user-managed files: do not expose a pending-upload list or manual
cleanup control in the normal cloud UI. Authorized abort APIs remain available
for technical recovery. Future automatic cleanup must remove abandoned staging
in the background without interrupting active/retriable uploads or deleting a
committed file. That cleanup is deferred for this demo. Trash currently has
no automatic 30-day purge: do not display or promise that deadline. The portal
offers restore, but no permanent-delete/empty-Trash control yet. Manual Trash purge
is a small possible follow-up, not a requirement to expand this demo. Existing
legacy owner-only permanent-deletion APIs are irreversible and also available to
the owner's AI; the alpha does not claim a human-only safeguard. A future purge
must be explicitly destructive, owner-authorized and reference-aware.

Staging, Trash and retained revisions count toward the owning space's
quota. Initial existing limits are 64 MiB/file, 256 MiB/space, 2 GiB/service, 20 files
per space and 20 revisions/file. Bound folders, tree depth, grants, cursors, pending
operations and request receipts as well. Garbage collection must never reclaim live
or referenced bytes. Backups and controller-level recovery are not implied by Undo.

## Craft app integration

All seven registered Craft web apps use the shared cloud experience. Open/Save/Save
As/history must respect the selected user's live permissions. Saving a
snapshot marks only that unchanged editor revision clean. Conflict or lost access
preserves local work and offers Save As where permitted. Switching accounts cannot
silently save into another account or discard unsaved work.

Stable launch references select files, not trusted identity or authorization. Never
send private bytes in navigation URLs. Rename/move does not break a linked editor;
Trash/revocation produces a recoverable error without overwriting metadata or
marking an unsaved draft clean. Native project encoding remains in the app adapter.
Standalone desktop integration and headless native-agent editing are separate work.

## MVP verification and known coverage boundaries

A working UI mockup, library tests, or a signed-identity probe alone does not prove
MCP-to-cloud functionality. Record actual commands, artifacts and limitations.

1. **Identity evidence**: real II-issued app/MCP delegations against a disposable
   local canister, signed HTTP ingress, verified certificates, observed caller-info,
   query-only restrictions and revocation behavior. A separate-principal control
   demonstrates the future boundary without claiming connector support for it.
2. **Core sharing**: Alice owns a folder/file, shares with Bob, and Bob's actual
   signed calls exercise each capability. Test denial, inheritance, direct file
   privacy, expiry/revocation, conflicting grants and Alice-only administration.
3. **Organization and content**: nested create/rename/move/Trash/restore, cycle and
   cross-space rejection, subtree atomicity, name conflicts, stable links, immutable
   revisions, stale writes, retries, interrupted uploads, quotas and recovery.
4. **Real MCP demo**: connect through the selected standard ICP MCP implementation,
   authenticate with II, discover the cloud interface, create a file, list/read it,
   and verify the same bytes/history in the UI. Repeat under a collaborator's account
   and demonstrate denial outside their share. Record the negotiated MCP/client
   versions. Do not describe direct Candid calls alone as this test.
5. **Shared UI and Craft**: native shared save/open has been checked in all seven
   apps. The canonical SDK/backend tests cover format, conflict and permission
   boundaries; browser negative-path evidence is strongest in Word, plus Grid's
   quota recovery. The full per-app matrix of conflict, history, permission loss,
   Trash, rename/move while open and account switching is not complete. Expanding
   it into an automated browser suite is follow-up work, not a claim made by this
   demo. Retain existing locale, keyboard/menu and light/dark picker checks.
6. **Persistence and security**: upgrade old fixtures preserving stable IDs/bytes,
   enforce authorization on every legacy/new endpoint, verify no partial mutation
   after errors, bound memory/instruction use, and review every bypass path. Actual
   private files and production canisters are not test fixtures.

The recorded evidence combines unit/contract tests, actual local canister upgrades,
separate II and MCP transport experiments, manual native browser journeys, published
asset checks and the user's mainnet ChatGPT confirmation. It is not one automated
hosted-provider end-to-end suite or an independent security audit. Store-saturation
and exhaustive worst-case operation testing remain follow-up beyond the existing
64 MiB commit-budget check. See the progress document for exact commands and limits.

## Deferred beyond this demo

Proposals (submit/approve/reject/withdraw and candidate review), search (requires its
own access-aware design), general file/folder copy, automatic staging/Trash cleanup,
a complete organization activity log, richer permission editing and UX polish.
Separate agent credentials/scopes/provenance and native semantic Craft agent editing
remain the already accepted future milestones. The earlier detailed design is
preserved in the future-design document rather than treated as finished functionality.

The current MVP scope is closed for the demo. Known limitations stay visible; fixing
an actual defect in delivered functionality is appropriate, but additional features
need a separate work package. Do not advertise deferred restrictions or coverage.
