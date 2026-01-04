This directory contains runnable examples that reproduce the PDFs generated in the integration tests.

Examples:
- `basic_and_structural.rs` - creates a simple PDF with text and performs a basic structural parse.
- `image_positions.rs` - (requires `--features images`) creates PDFs with embedded images at known positions and prints observed transformation matrices.

Usage:
- Build and run the basic example:
  cargo run --example basic_and_structural

- Build and run the image example (images feature required):
  cargo run --example image_positions --features images

Notes:
- Examples write PDFs to `examples/output/` for inspection.
- Optionally run the Python validator `tests/scripts/validate_pdf.py` (requires PyMuPDF + Pillow) to perform visual checks. You can install deps with `python3 -m pip install -r tests/scripts/requirements.txt`.
- To capture debug diffs from example runs, set the env var `GENPDFI_SAVE_DIFFS=target/test-diffs` before running the example.
