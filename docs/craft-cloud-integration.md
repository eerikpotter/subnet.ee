# Upstream cloud integration assessment — 24 November 2026

## Requirement

Refresh the eight existing Craft browser applications, retain their native files,
existing ICP accounts, canisters and save acknowledgements, and evaluate whether
ArtCraft's own cloud module can replace application-specific integration code.

## Official ICP guidance

Retrieved the official skills index on 24 November 2026 and consulted
[icp-cli](https://skills.internetcomputer.org/.well-known/skills/icp-cli/SKILL.md),
[static-site](https://skills.internetcomputer.org/.well-known/skills/static-site/SKILL.md)
and [cycles-management](https://skills.internetcomputer.org/.well-known/skills/cycles-management/SKILL.md),
including their relevant deployment, identity, custom-domain and funding references.
This deployment uses existing standard IC mainnet canisters; the user's separate
OpenCloud engine is not its target. No new identity protocol, canister or proxy is
needed for the refresh.

## Supported upstream approach and observed gap

**Proven:** ArtCraft merged its provider-neutral `craft-cloud` contracts, shared
cloud/share UI and local reference adapter on 17 November in
[craft-libs #160](https://github.com/storytold/craft-libs/pull/160).
[RFC-0024](https://github.com/storytold/craft-contrib/pull/27), merged 13 November,
defines the shared provider boundary. An ICP implementation of that boundary is
the preferred direction when adopting upstream's cloud UI. This is evidence of a
usable integration contract, not evidence that every editor already uses it or
that ArtCraft's hosted storage runs on ICP.

**Proven for the reviewed pins:** none of the eight application source trees
consumes `craft-cloud` or `CloudBackend`. Their exact commits are recorded in the
24 November release evidence. Some already use `craft-web-host`, but those
document embedding bridges have different completion semantics. For example,
CAD's current save posts without an acknowledgement ID and Grid's bridge updates
the saved filename after posting. Merely replacing those messages with a network
upload would not establish durable persistence before clearing dirty state.
The [shared web-host contract](https://github.com/storytold/craft-libs/issues/358)
does support acknowledged saves; application adoption must be verified per host.

## Smallest extension for this release

Retain the existing shared ICP SDK, cloud controller and native-format adapters.
Port only the editor hooks moved by upstream, preserving each application's pinned
transport SDK. The new Photo/PDF declarative menus use upstream's command catalog
and submenu model for host requests. They queue work and retain document, busy,
modal and interactive-only guards; the cloud controller acknowledges persistence.
Other applications keep their existing optional host menu/backstage hooks.

SoundCraft's upstream redraw revision also changes for navigation. Its browser
adapter now compares content through the engine's existing document equality and
keeps a separate cloud revision, so local downloads cannot hide unsynced edits.
This avoids repeated portable-audio uploads when only the playhead moves.

Translations now supplied through external `craft-i18n.rev` pins are cached by
exact revision. Builds refuse missing inputs rather than silently losing languages.

## Alternatives, proof and unresolved work

**Proposed/deferred:** implement a single ICP `CloudBackend` adapter and wire each
editor to the standard cloud UI when its document lifecycle supports the contract.
That can reduce custom menus, account dialogs and provider plumbing. It still
needs editor-specific native serialization and revision acknowledgement unless
upstream exposes those uniformly. It does not imply moving existing user data or
changing Internet Identity's derivation origin.

Proof for that later replacement must include save/open with existing files,
concurrent edits during upload, failed writes, account changes during a request,
read-only shares and version conflicts. Confirm asynchronous completion and dirty
state before removing the old adapters. No untested provider replacement is part
of this update. Review build and focused integration evidence separately from
upstream CI; neither proves every unrelated editor feature correct.

The refreshed PdfCraft package includes an OCR worker. Assembly now identifies
the page's explicit Wasm bootstrap and preserves the worker's hashed URLs; the
worker accepts Trunk's forwarded cloud feature without linking any ICP code.
The upstream `xtask models` packaging step supplies its hash-verified OCR models,
licences and attribution beside the editor; recognition does not send documents
to an external service. A macOS-only module qualification was also corrected to
allow the focused native persistence/menu tests to compile on the release host;
that branch is excluded from the browser artifact.
PhotoCraft's frame maintenance also needed a browser-safe clock, while its
filesystem recovery timer remains native-only. Both repairs are checked through
the actual browser build and startup.

WordCraft's new shared font picker initially remained at "Loading fonts…" in the
browser: its renderer's in-memory `FontDb` was not connected to the shared
catalog, and native folder scanning cannot supply browser fonts. The small web
host bridge now uses the catalog's existing `add_bytes` API, deduplicates font
files and refreshes when the renderer's font generation changes. It retains the
upstream picker and existing bundled/imported fonts, with no new font service.

No additional production provider, server, authentication flow or canister was introduced.
The reusable upstream menu model replaces the removed menu rendering hooks; a new
custom menu renderer was unnecessary.
