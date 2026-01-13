// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{fonts, elements, Document, Alignment};

fn main() {
    if !cfg!(feature = "images") {
        eprintln!("Skipping example: 'images' feature not enabled");
        return;
    }

    #[cfg(feature = "images")]
    {
        println!("Running example: PDF with image links");

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
        doc.set_title("PDF with image links");

        // Example image with link
        doc.push(elements::Paragraph::new("1. Clickable image below (opens example.com):"));
        let image_path = "examples/images/test_image.jpg";
        let img = elements::Image::from_path(image_path)
            .expect("Failed to load image")
            .resizing_page_with(0.5)
            .with_alignment(Alignment::Center)
            .with_link("https://example.com");
        doc.push(img);

        // Another example combining text and image link
        doc.push(elements::Paragraph::new("\n2. Image with a different link:"));
        let img2 = elements::Image::from_path(image_path)
            .expect("Failed to load image")
            .resizing_page_with(0.25)
            .with_alignment(Alignment::Center)
            .with_link("https://github.com");
        doc.push(img2);

        // Generate the output PDF
        let mut pdf_file = fs::File::create(&out_dir.join("pdf_with_image_links.pdf"))
            .expect("create output file");
        doc.render(&mut pdf_file).expect("render document");
        println!("✓ Created examples/output/pdf_with_image_links.pdf");
    }
}
