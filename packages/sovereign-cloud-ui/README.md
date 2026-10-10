# Shared Craft cloud UI

The canonical cloud dialogs and browser controller for all five Craft editors.
Authentication, transfer integrity, retries, account scoping and file formats are
provided by `sovereign-cloud`; each web host implements `Editor` for its documents.
`scripts/sync-sdk.py` vendors both packages into each independent app checkout.

An adapter must capture immutable content, use a fresh document ID after New/Open,
mark only the committed snapshot clean, and parse incoming files before replacing
any work. Single-document editors require the shared discard confirmation.

The ICP mark is attributed in `assets/icp/README.md`; its Apache-2.0 license is kept
beside it. This package's source is MIT OR Apache-2.0.
