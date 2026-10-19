# PDF dependency patch

`lopdf/` is the MIT-licensed crates.io release 0.45.0. Its upstream source hashes
are recorded in `upstream-sha256.json`; the upstream LICENSE is retained.

Only `src/encodings/glyphnames.rs` differs: `Glyph::from_name` uses binary search
over a generated sorted table with exactly the original names, aliases and values.
The unchanged `glyphs!` definition is the source of that table. No PDF parsing,
decompression, font decoding, encryption or other behavior is replaced.

Evidence: the original lookup compiles to a Wasm function with complexity
2,734,911 (Rust 1.95, size profile), above ICP's 1,000,000 validation limit.
The normal `z` size profile also fails (2,734,695). The table removes this enormous
branch function without changing its result or introducing a custom PDF parser.

Run `python3 -m unittest discover -s scripts -p test_pdf_dependency.py` to verify
all mappings and all unchanged upstream files. Canonical extraction tests and the
PocketIC `content_search` test additionally verify actual PDF text and installability.
On an upstream update, re-evaluate whether this patch is still required; never
silently carry it over. Other extracted formats use ordinary unpatched crates.
