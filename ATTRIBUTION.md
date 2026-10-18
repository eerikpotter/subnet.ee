# Attribution

The portal layout and CSS illustrations are original code created for this
subnet.ee workspace and licensed under MIT OR Apache-2.0.

The subnet.ee cloud mark in `portal/assets/brand/subnet.png` was supplied by the
project owner and is used unchanged. It is not covered by the portal's code license.

The EFTI mark in `portal/assets/brand/efti.png` identifies the institute in the
portal footer and links to `https://efti.org/`. It is not covered by the portal's
code license.

The public landing page follows the owner's supplied reference, "Eesti suveräänse
nutipilve avaleht.png". The hero uses the owner's "Golden Mist Over Estonian Lakes.png"
and "cloud_of_estonia.png", copied unchanged to `portal/assets/landing/estonian-lakes.png`
and `portal/assets/landing/cloud-of-estonia.png`. The cloud retains its transparency
and floats independently with a CSS animation, disabled for reduced-motion preferences.
The earlier `portal/assets/landing/estonian-cloud.jpg` is retained as an unused
owner-supplied alternative, with original bytes and the matching JPEG extension.
`portal/assets/landing/creative-workspace.jpg` is an
AI-generated background recreated from the reference's lakeside workspace scene;
it is not an original separate photo supplied by the owner. The layered architecture
diagram and tool icons are local SVG artwork. These images load without third-party
requests.

Inter (Rasmus Andersson) and Sora (Sora Project Authors) are bundled locally under
the SIL Open Font License 1.1. Their license notices are in `portal/assets/fonts/`.
Inter files come from the existing PhotoCraft font assets; Sora files come from
[Google Fonts](https://fonts.google.com/specimen/Sora), with the license from
the [Google Fonts repository](https://github.com/google/fonts/tree/main/ofl/sora).

PhotoCraft, PdfCraft, WordCraft, DeckCraft, SoundCraft, CADCraft, GridCraft and FigCraft are independent Crafting Apps by the
ArtCraft team and contributors. The checked-in submodule revisions are the source of
truth. Each application's license, NOTICE and asset attribution are copied into its
own directory in the assembled site. This portal is an independent integration.

The app-card images in `portal/assets/crafts/` are unmodified 256-pixel PNG exports
of the respective Craft app icons, redistributed under their MIT license option.
[Source revisions and artwork credits](portal/assets/crafts/README.md) are recorded
beside the images; original license notices are preserved in `licenses/` there.
These are application icons, not the ArtCraft umbrella brand logo.

The internal Sovereign Cloud SDK is extracted from the ICP integration in
`eerikpotter/photocraft`, preserving its license and V1 protocol.

- `portal/vendor/fflate/browser.js`, `portal/vendor/fflate/LICENSE`: fflate 0.8.2,
  Arjun Barrett, MIT, <https://github.com/101arrowz/fflate/tree/v0.8.2>.
  Pinned npm archive `fflate-0.8.2.tgz`; browser module SHA-256
  `8cc1f687e0159e977addb6b85e274dbd11e622cf151f4fcb7b85d49622ea43e7`.
  Used only to read bounded, allowlisted Office XML parts for inert previews.

## PDF preview

Mozilla PDF.js 6.4.299 (Apache-2.0), https://github.com/mozilla/pdf.js .
The display layer, worker, CMaps, fonts and Wasm codecs are served locally from
`portal/vendor/pdfjs/`. Licenses and package checksum are retained there.
