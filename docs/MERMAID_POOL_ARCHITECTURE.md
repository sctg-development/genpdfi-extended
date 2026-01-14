# Mermaid renderer pool (TypeScript) — Proof-of-Concept

A proof-of-concept TypeScript renderer pool and a Rust example were added to the repository:

- `examples/mermaid_pool/` — TypeScript project (Vite + vite-plugin-singlefile).
- `examples/mermaid_pool_proof_of_concept.rs` — Rust example that loads the bundled helper page and submits the 28 diagrams from `tests/mermaid_render_each.rs`.

Quick start (local):

1. cd examples/mermaid_pool
2. npm ci
3. npm run build
4. cargo run --example mermaid_pool_proof_of_concept --features "mermaid,images"

Notes:
- The bundling step produces `examples/mermaid_pool/dist/index.html` containing a single-file HTML bundle which the Rust example embeds using a data URI.
- Pool concurrency can be configured by passing `?pool=3` query param (default `pool=2`) which the example sets to `?pool=3` by default.
