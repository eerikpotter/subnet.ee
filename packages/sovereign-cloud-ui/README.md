# Shared Craft cloud UI

The canonical cloud dialogs and browser controller for all registered Craft editors.
Authentication, transfer integrity, retries, account scoping and file formats are
provided by `sovereign-cloud`; each web host implements `Editor` for its documents.
`scripts/sync-sdk.py <app>` vendors both packages into one independent app checkout;
use `all` explicitly when every fork should adopt the new SDK version.

An adapter must capture immutable content, use a fresh document ID after New/Open,
mark only the committed snapshot clean, and parse incoming files before replacing
any work. Single-document editors require the shared discard confirmation.

The ICP mark is attributed in `assets/icp/README.md`; its Apache-2.0 license is kept
beside it. This package's source is MIT OR Apache-2.0.


Open and Save As embed the same-origin `/pilveruum/` drive in selection mode.
The frame shows personal/shared folders and the current application's native files,
with light/dark styling selected by the editor. It returns only an account marker
and stable file/folder IDs plus the chosen name, through an origin-, frame- and
channel-checked message. This is a UI selection, never an authorization credential.
The canonical SDK revalidates the actual signed caller, file format and permissions.

Save updates the existing drive file with its expected revision; Save As creates a
new file under the selected folder. Bindings use the account that opened the file,
not its owner. Read-only access permits opening and making a separate copy where
Create is allowed. Permission failures and version conflicts retain the editor's
work and captured retry bytes. Switching accounts clears pending private UI and
callbacks but keeps document bindings, requiring explicit Save As for a new account.
The embedded picker requires the subnet portal routes alongside the Craft host.
