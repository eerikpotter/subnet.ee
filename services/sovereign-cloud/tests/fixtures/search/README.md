# Synthetic search fixtures

All content is invented test data. `Neptuun kinnitatud` occurs as separate words
in every file, independent of its name. PDF includes real Type1 text and an xref;
WordCraft contains its versioned native body. No personal documents are included.

The Office packages were generated and reopened with `python-docx`, `openpyxl`
and `python-pptx` (bundled workspace runtime), including standard content types,
relationships and namespaces. DOCX includes a table; XLSX has named sheet `Tulud`;
PPTX has an ordinary title/content slide. The canonical extraction unit tests also
use deliberately minimal ZIP/XML fixtures to isolate malformed/hostile cases.

`content_search.rs` uploads these committed fixtures through Candid and checks
actual Wasm timer extraction, signed discovery, locations, revocation and upgrades.
