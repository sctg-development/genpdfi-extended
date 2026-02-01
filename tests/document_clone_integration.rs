use genpdfi_extended::{Document, elements, fonts, SimplePageDecorator, Margins};

#[test]
fn integration_document_clone_independent_render() {
    // Load bundled font
    let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
    let fd = fonts::FontData::new(data, None).expect("font data");
    let family = fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };

    // Create doc and add content
    let mut doc = Document::new(family);
    doc.push(elements::Paragraph::new("First page"));

    // Clone and mutate original
    let mut doc_clone = doc.clone();
    doc.push(elements::Paragraph::new("Second page (original only)"));

    // Render both and compare outputs
    let mut out_orig = Vec::new();
    doc.render(&mut out_orig).expect("render original");
    let mut out_clone = Vec::new();
    doc_clone.render(&mut out_clone).expect("render clone");

    assert!(!out_orig.is_empty(), "original output must not be empty");
    assert!(!out_clone.is_empty(), "clone output must not be empty");
    // They should not be identical and original should contain at least as much content
    assert_ne!(out_orig, out_clone, "render outputs should differ after mutating original");
    assert!(out_orig.len() >= out_clone.len(), "original should be same or larger after adding content");
}

#[test]
fn integration_document_clone_decorator_header_behavior() {
    // Load bundled font
    let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
    let fd = fonts::FontData::new(data, None).expect("font data");
    let family = fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };

    // Create a decorator with margins and a header callback
    let mut dec = SimplePageDecorator::new();
    dec.set_margins(Margins::from(12.0));
    dec.set_header(|_| elements::Paragraph::new("HEADER"));

    let mut doc = Document::new(family);
    doc.set_page_decorator(dec);
    doc.push(elements::Paragraph::new("Body"));

    // Clone the document
    let doc_clone = doc.clone();

    // Render both
    let mut out_orig = Vec::new();
    doc.render(&mut out_orig).expect("render original");
    let mut out_clone = Vec::new();
    doc_clone.render(&mut out_clone).expect("render clone");

    assert!(!out_orig.is_empty(), "original output must not be empty");
    assert!(!out_clone.is_empty(), "clone output must not be empty");

    // The original had a header callback; the clone's decorator should have preserved margins
    // but dropped the header callback, so the original output should be larger.
    assert!(out_orig.len() > out_clone.len(), "original with header should be larger than clone without header");
}
