<!--
Copyright (c) 2025 Ismael Theiskaa
Copyright (c) 2026 Ronan Le Meillat - SCTG Development
SPDX-License-Identifier: MIT OR Apache-2.0
Licensed under the MIT License or the Apache License, Version 2.0

-->

![Code](https://tokeisrv.sctg.eu.org/b1/github.com/sctg-development/genpdfi-extended?type=Rust,TypeScript,TSX,C&category=code)
![Comments](https://tokeisrv.sctg.eu.org/b1/github.com/sctg-development/genpdfi-extended?type=TSX,Rust,TypeScript&category=comments&color=abdbe3)
![Documentation](https://tokeisrv.sctg.eu.org/b1/github.com/sctg-development/genpdfi-extended?type=Markdown&label=doc&color=e28743)
[![codecov](https://codecov.io/github/sctg-development/genpdfi-extended/branch/main/graph/badge.svg)](https://codecov.io/github/sctg-development/genpdfi-extended)
![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)
# genpdfi_extended

A modernized fork of genpdfi with improved tests, documentation, and CI.

Overview
--------

`genpdfi_extended` is a Rust library for programmatic PDF generation. It provides a
high-level API (`Document`, `Element`, `Style`, `Paragraph`, `Table`, `Image`, ...)
and lower-level rendering primitives in `render` for advanced usage.

Highlights
----------

- Font management with caching, embedding, and metrics (deterministic tests use bundled fonts in `fonts/`).
- Robust text serialization for embedded fonts: strings are emitted as full text so PDF viewers apply native kerning and text extractors can recover readable text (prevents glyph-id remapping issues when subsetting).
- Optional `images` feature to embed common image formats (PNG, JPEG, etc.).
- Optional `latex` feature to render LaTeX formulas to SVG using `microtex_rs` and embed them as images.
- Optional `mermaid` feature to render Mermaid diagrams to SVG using a headless Chrome instance and embed them as images.
- Executable doc examples (no `no_run`/`ignore`) to improve documentation quality.
- CI workflow that runs tests, generates Cobertura coverage (via tarpaulin) and publishes rustdoc to GitHub Pages.

Quick example (in-memory)
-------------------------

```rust
use genpdfi_extended::{Document, elements, fonts};

let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
let fd = fonts::FontData::new(data, None).expect("font data");
let family = fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };
let mut doc = Document::new(family);

doc.push(elements::Paragraph::new("Hello genpdfi_extended!"));
let mut buf = Vec::new();
doc.render(&mut buf).expect("render");
assert!(!buf.is_empty());
```

Tests and Coverage
------------------

- Run unit tests: `cargo test`
- Run tests including images: `cargo test --features images`
- Generate Cobertura coverage via tarpaulin:

```bash
cargo tarpaulin --features images --out Xml --output-dir .
# produces coverage/cobertura.xml
```

CI and Docs
-----------

A GitHub Actions workflow (`.github/workflows/ci.yml`) is included that:

- builds and runs tests (with and without `images`),
- runs tarpaulin to generate `cobertura.xml` and uploads it to Codecov using the slug
  `sctg-development/genpdfi-extended` with `secrets.CODECOV_TOKEN`,
- builds rustdoc and publishes it to `gh-pages` (GitHub Pages).

Images feature
--------------

Enable `images` to test and use image support:

```bash
cargo test --features images
cargo doc --features images --no-deps
```

Mermaid / Headless Chrome
-------------------------

When using the `mermaid` feature, diagrams are rendered by a headless Chromium instance via the `headless_chrome` crate.
On first execution the crate may download a Chromium binary automatically; this can make the very first run noticeably longer and requires network access. If you prefer to avoid the download, install Chrome/Chromium system-wide and ensure it is available in PATH before running Mermaid examples.

Contributing
------------

- Format: `cargo fmt`
- Lint: `cargo clippy`
- Tests: `cargo test` and coverage with tarpaulin
- Use the bundled fonts in `fonts/` for test determinism

License
-------

See the `LICENSES/` directory for included licenses.

If you want, I can add CI/docs badges to the README and commit & push these changes—what would you prefer?