Mermaid renderer pool (proof of concept)

This folder contains a small TypeScript project that implements a simple
in-page pool of Mermaid renderers. The pool exposes a minimal API that
Rust code can call using `headless_chrome` and `tab.evaluate`.

Quick start (local)

1. Install dependencies and build:

   cd mermaid_pool
   npm ci
   npm run build

2. After building, the `dist/index.html` file is produced and contains
   an inlined single-file HTML bundle (thanks to `vite-plugin-singlefile`).

3. Run the Rust example to use the bundled page (instructions in
   `examples/mermaid_pool_proof_of_concept.rs`).

Design notes

- Written in TypeScript 5.9.3 and built with Vite 7.3.1.
- Uses `vite-plugin-singlefile` 2.3.0 to embed JS/CSS into a single HTML
  file which can be included into Rust tests via `include_str!`.

The project is intended as a proof-of-concept and not yet production hardened.
