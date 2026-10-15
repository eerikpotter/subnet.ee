# subnet.ee

[subnet.ee](https://www.subnet.ee/) is an [EFTI](https://efti.org/) public demo
of browser-based creative tools with shared cloud storage on the Internet Computer (ICP).

The creative tools come from the ArtCraft team and community. EFTI develops the
portal and cloud integration: shared sign-in, file storage, versions and opening
files across the workspace.

The demo brings together PhotoCraft, PdfCraft, WordCraft, DeckCraft, SoundCraft,
CADCraft, GridCraft and FigCraft for images, PDFs, documents, slides, audio, drawings,
spreadsheets and interface design. The portal is available in Estonian and English.

## How it works

The Rust/WebAssembly editors run on the user's device. An ICP frontend canister
serves the website and applications; a separate backend stores accounts, files
and revisions. Sign-in uses Internet Identity.

The current demo uses an operator-managed backend. User-owned data canisters
are a direction for future development.

## Repository

- `portal/` — the public site, workspace and shared editor navigation.
- `apps/` — the eight editor forks, pinned as Git submodules.
- `packages/` — the shared Sovereign Cloud SDK, UI and portal Wasm facade.
- `services/sovereign-cloud/` — the account and file backend.
- `registry/apps.json` — editor metadata and supported cloud formats.
- `scripts/` — setup, builds, assembly, updates and verification.

## Development

In an initialized local checkout, update the portal and verify it with:

```sh
python3 scripts/dev.py
python3 scripts/verify.py
```

These commands reuse the packaged editors and update the local frontend.
See the [development guide](docs/development.md) for first-time setup, editor
builds, shared-package changes and choosing the relevant checks.

## Documentation

- [Deployment and operations](docs/deployment.md) — environments, domains and publishing.
- [Updating editor forks](scripts/UPDATING.md) — upstream review and submodule updates.
- [Development history](docs/history.md) — earlier integration milestones and validation notes.

## License and credits

Root code and the shared SDK use [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
The editors retain their own licenses and notices. See [ATTRIBUTION.md](ATTRIBUTION.md)
for application and asset credits.
