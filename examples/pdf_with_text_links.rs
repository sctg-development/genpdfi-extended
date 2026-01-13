// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{fonts, elements, Document, Alignment};
use genpdfi_extended::style::{Style, Color};

fn main() {
    println!("Running example: PDF with text links");

    // Prepare output dir
    let out_dir = PathBuf::from("examples/output");
    fs::create_dir_all(&out_dir).expect("create examples/output dir");

    // Load font family
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

    // Create a document
    let mut doc = Document::new(family);
    doc.set_title("PDF with text links");

    // A paragraph with inline links
    let mut p = elements::Paragraph::new("Click the link: ");
    p.push_link(
        "Rust homepage",
        "https://www.rust-lang.org",
        Style::new().with_color(Color::Rgb(0, 64, 160)).with_font_size(12),
    );
    p.push(" — follow to learn more about Rust.");
    doc.push(p);

    // Another paragraph demonstrating mailto and fragment links
    let mut p2 = elements::Paragraph::new("");
    p2.push("Contact: ");
    p2.push_link(
        "Email us",
        "mailto:you@example.com",
        Style::new().with_color(Color::Rgb(0, 128, 0)),
    );
    p2.push(" — or read section: ");
    p2.push_link(
        "About",
        "https://example.com/page#about",
        Style::new().with_color(Color::Rgb(128, 0, 128)),
    );
    doc.push(p2);

    // Centered footer link
    let mut footer = elements::Paragraph::new("");
    footer.push_link(
        "Open project on GitHub",
        "https://github.com",
        Style::new().with_color(Color::Rgb(0, 0, 0)),
    );
    footer.set_alignment(Alignment::Center);
    doc.push(footer);

    // Generate the output PDF
    let mut pdf_file = fs::File::create(&out_dir.join("pdf_with_text_links.pdf"))
        .expect("create output file");
    doc.render(&mut pdf_file).expect("render document");
    println!("✓ Created examples/output/pdf_with_text_links.pdf");
}
