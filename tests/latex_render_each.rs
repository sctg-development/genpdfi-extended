// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

const LATEX_FORMULAS: &[(&str, &str, f32)] = &[
    ("1. Einstein's Equation", r#"E = mc^2"#, 12.0),
    ("2. Pythagorean Theorem", r#"a^2 + b^2 = c^2"#, 12.0),
    (
        "3. Quadratic Formula",
        r#"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#,
        12.0,
    ),
    (
        "4. Gaussian Integral",
        r#"\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}"#,
        12.0,
    ),
    ("5. Euler's Formula", r#"e^{i\pi} + 1 = 0"#, 12.0),
    ("6. Golden Ratio", r#"\phi = \frac{1 + \sqrt{5}}{2}"#, 12.0),
    ("7. Circular Area", r#"A = \pi r^2"#, 12.0),
    ("8. Sphere Volume", r#"V = \frac{4}{3}\pi r^3"#, 12.0),
    (
        "9. Sine Addition",
        r#"\sin(a + b) = \sin(a)\cos(b) + \cos(a)\sin(b)"#,
        12.0,
    ),
    ("10. Cosine Law", r#"c^2 = a^2 + b^2 - 2ab\cos(C)"#, 12.0),
    (
        "11. Sum of Series",
        r#"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}"#,
        12.0,
    ),
    (
        "12. Geometric Series",
        r#"\sum_{i=0}^{\infty} r^i = \frac{1}{1-r}, |r| < 1"#,
        12.0,
    ),
    (
        "13. Binomial Expansion",
        r#"(x + y)^n = \sum_{k=0}^{n} \binom{n}{k} x^k y^{n-k}"#,
        12.0,
    ),
    (
        "14. Logarithm Rule",
        r#"\log(ab) = \log(a) + \log(b)"#,
        12.0,
    ),
    ("15. Power Rule", r#"\frac{d}{dx} x^n = nx^{n-1}"#, 12.0),
    ("16. Product Rule", r#"(fg)' = f'g + fg'"#, 12.0),
    (
        "17. Chain Rule",
        r#"\frac{d}{dx} f(g(x)) = f'(g(x)) \cdot g'(x)"#,
        12.0,
    ),
    (
        "18. Fundamental Theorem",
        r#"\int_a^b f'(x) dx = f(b) - f(a)"#,
        12.0,
    ),
    (
        "19. Wave Equation",
        r#"\frac{\partial^2 u}{\partial t^2} = c^2 \nabla^2 u"#,
        12.0,
    ),
    (
        "20. Normal Distribution",
        r#"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}"#,
        11.0,
    ),
    (
        "21. Complex Number",
        r#"z = a + bi, |z| = \sqrt{a^2 + b^2}"#,
        12.0,
    ),
    (
        "22. Matrix Determinant",
        r#"\begin{vmatrix} a & b \\ c & d \end{vmatrix} = ad - bc"#,
        12.0,
    ),
];

#[cfg(feature = "latex")]
#[test]
fn render_each_latex_formula_to_pdf() {
    // Run this test with `cargo test --features latex --test latex_render_each -- --nocapture`
    use std::fs;
    use std::path::PathBuf;

    use genpdfi_extended::{elements, fonts, style, Alignment, Document};

    // Prepare output directory
    let out_dir = PathBuf::from("tests/output/latex_render_each");
    fs::create_dir_all(&out_dir).expect("create tests/output/latex_render_each dir");

    // Load font
    let font_data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd = fonts::FontData::new(font_data, None).expect("font data");
    let family = fonts::FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };

    // Simplified test: for each LaTeX formula, create a document, push a heading and the Latex element,
    // then render to a PDF file.
    for (i, (title, formula, size)) in LATEX_FORMULAS.iter().enumerate() {
        // Save time for instrumentation
        let start = std::time::Instant::now();
        eprint!("Rendering LaTeX formula {}... ", i + 1);
        let mut doc = Document::new(family.clone());
        doc.set_title(format!("LaTeX Formula {}", i + 1));
        doc.push(elements::Paragraph::new("").styled_string(
            title.to_string(),
            style::Style::new().with_font_size(14).bold(),
        ));
        doc.push(elements::Paragraph::new(""));

        // Construct the LaTeX element
        let latex = elements::Latex::new(*formula, *size).with_alignment(Alignment::Center);
        doc.push(latex);

        let output_path = out_dir.join(format!("latex_formula_{:02}.pdf", i + 1));
        doc.render_to_file(&output_path).expect("render document");

        // Validation 1: Check file exists
        assert!(
            output_path.exists(),
            "Output file {} should exist",
            output_path.display()
        );

        // Validation 2: Check file is not empty
        let metadata = fs::metadata(&output_path).expect("should be able to read file metadata");
        assert!(
            metadata.len() > 0,
            "Output file {} should not be empty (size: {})",
            output_path.display(),
            metadata.len()
        );

        // Validation 3: Check that the PDF contains exactly one vectorial Form XObject
        validate_pdf_contains_single_vectorial_form(&output_path);

        let duration = start.elapsed();
        eprintln!("done in {:.2?}", duration);
    }
}

#[cfg(feature = "latex")]
fn validate_pdf_contains_single_vectorial_form(pdf_path: &std::path::Path) {
    use lopdf::{Document, Object};

    let document = Document::load(pdf_path).expect("PDF should be readable");

    let mut vectorial_form_count = 0;

    // Scan all streams to find Form XObjects with vectorial content
    for (_, obj) in document.objects.iter() {
        if let Object::Stream(stream) = obj {
            if let Ok(subtype) = stream.dict.get(b"Subtype") {
                if let Some(subtype_str) = get_xobject_subtype(subtype) {
                    if subtype_str == "Form" {
                        // Check if it contains vectorial PDF operators
                        if is_vectorial_content(&stream.content) {
                            vectorial_form_count += 1;
                        }
                    }
                }
            }
        }
    }

    assert_eq!(
        vectorial_form_count,
        1,
        "PDF {} should contain exactly 1 vectorial Form XObject, found {}",
        pdf_path.display(),
        vectorial_form_count
    );
}

#[cfg(feature = "latex")]
fn get_xobject_subtype(obj: &lopdf::Object) -> Option<String> {
    match obj {
        lopdf::Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
        lopdf::Object::Name(name) => Some(String::from_utf8_lossy(name).to_string()),
        _ => None,
    }
}

#[cfg(feature = "latex")]
fn is_vectorial_content(content: &[u8]) -> bool {
    // Try to decompress first
    let decompressed = decompress_zlib(content);
    let content_str = String::from_utf8_lossy(&decompressed);

    // Check for PDF vector operators (paths)
    content_str.contains(" m ") || // moveto
    content_str.contains(" l ") || // lineto
    content_str.contains(" c ") || // curveto
    content_str.contains(" h ") || // close path
    content_str.contains(" f ") || // fill
    content_str.contains(" S ") || // stroke
    content_str.contains("q\n") || // save graphics state
    content_str.contains("Q\n") || // restore graphics state
    // content_str.contains("BT ") || // begin text
    // content_str.contains(" ET") || // end text
    content_str.contains(" re ") // rectangle
}

#[cfg(feature = "latex")]
fn decompress_zlib(data: &[u8]) -> Vec<u8> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    // Try zlib decompression (zlib header: 0x78)
    if data.len() > 1 && data[0] == 0x78 {
        let mut decoder = ZlibDecoder::new(data);
        let mut result = Vec::new();
        if decoder.read_to_end(&mut result).is_ok() && !result.is_empty() {
            return result;
        }
    }

    // Return original data if decompression fails
    data.to_vec()
}
