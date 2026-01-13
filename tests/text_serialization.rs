// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

use genpdfi_extended::render::Renderer;
use genpdfi_extended::{Position, Size};

#[test]
fn test_embedded_font_serialization_contains_string() {
    use genpdfi_extended::fonts::{FontCache, FontData, FontFamily};
    use genpdfi_extended::style::Style;
    use printpdf::PdfParseOptions;

    let s = "Embedded test: ăâîșț";
    let data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd = FontData::new(data.clone(), None).expect("font data");
    let family = FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };
    let mut cache = FontCache::new(family);

    let mut r = Renderer::new(Size::new(210.0, 297.0), "serialize-test").expect("renderer");
    cache.load_pdf_fonts(&mut r).expect("load fonts");
    let area = r.first_page().first_layer().area();
    let style = Style::new()
        .with_font_family(cache.default_font_family())
        .with_font_size(12);
    assert!(area
        .print_str(&cache, Position::default(), style, s)
        .unwrap());

    let mut buf = Vec::new();
    r.write(&mut buf).expect("write");
    let mut warnings = Vec::new();
    let parsed = printpdf::PdfDocument::parse(&buf, &PdfParseOptions::default(), &mut warnings)
        .expect("parse");

    // Search debug string of serialized ops for a fragment of our string
    let mut found = false;
    for op in parsed.pages[0].ops.iter() {
        let sdebug = format!("{:?}", op);
        if sdebug.contains("Embedded test") || sdebug.contains("ă") {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Expected serialized PDF to contain text from the embedded string"
    );
}
