use genpdfi_extended::{elements, fonts, Document, Size};
use std::fs;
use std::path::PathBuf;

fn main() {
    // Prepare output dir
    let out_dir = PathBuf::from("examples/output");
    fs::create_dir_all(&out_dir).expect("create examples/output dir");
    // Load a font family from the bundled fonts directory and create a document.
    let family = fonts::from_files(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fonts"),
        "NotoSans",
        None,
    )
    .expect("Failed to load font family");
    let mut doc = Document::new(family);

    // Construct a simple table and add it to the document
    let mut table = elements::TableLayout::new(vec![1, 2]);
    table
        .row()
        .element(elements::Paragraph::new("Left cell with embedded font"))
        .element(elements::Paragraph::new("Right cell with more width"))
        .push()
        .expect("push");
    doc.push(table);

    // Write output PDF to examples/ to make it easy to inspect manually
    let out = PathBuf::from("examples/output/table_embedded.pdf");
    // ensure directory exists
    if let Some(p) = out.parent() {
        let _ = fs::create_dir_all(p);
    }
    doc.render_to_file(&out).expect("render document");
    println!("Wrote example PDF to {}", out.display());
}
