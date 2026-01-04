use genpdfi_extended::fonts::Builtin;
use genpdfi_extended::{elements, fonts, Document, Size};
use std::fs;
use std::path::PathBuf;

fn main() {
    // Prepare output dir
    let out_dir = PathBuf::from("examples/output");
    fs::create_dir_all(&out_dir).expect("create examples/output dir");
    // Load a font family but mark it as builtin-like so the PDF references builtin names
    let family = fonts::from_files(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fonts"),
        "SpaceMono",
        Some(Builtin::Helvetica),
    )
    .expect("Failed to load builtin-like family");
    let mut doc = Document::new(family);

    let mut table = elements::TableLayout::new(vec![1, 1]);
    table
        .row()
        .element(elements::Paragraph::new("Left with builtin"))
        .element(elements::Paragraph::new("Right with builtin"))
        .push()
        .expect("push");
    doc.push(table);

    let out = PathBuf::from("examples/output/table_builtin.pdf");
    if let Some(p) = out.parent() {
        let _ = fs::create_dir_all(p);
    }
    doc.render_to_file(&out).expect("render document builtin");
    println!("Wrote example PDF to {}", out.display());
}
