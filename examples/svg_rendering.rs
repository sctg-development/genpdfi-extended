// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{fonts, elements, Document};

fn main() {
    if !cfg!(feature = "images") {
        eprintln!("This example requires the 'images' feature to be enabled.");
        return;
    }

    #[cfg(feature = "images")]
    {
        println!("Running example: SVG rendering");

        // Prepare output directory
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
    doc.set_title("SVG Rendering Example");

    // Add title
    doc.push(elements::Paragraph::new("SVG Rendering Examples"));

    // Example 1: Simple colored circles
    doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("1. Simple SVG with colored circles"));
    let svg1 = r##"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg">
        <circle cx="50" cy="50" r="40" fill="#FF6B6B"/>
        <circle cx="150" cy="50" r="40" fill="#4ECDC4"/>
    </svg>"##;
    let img1 = elements::Image::from_svg_string(svg1)
        .expect("Failed to load SVG");
    doc.push(img1);

    // Example 2: Geometric shapes
    doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("2. Geometric shapes"));
    let svg2 = r##"<svg width="200" height="150" xmlns="http://www.w3.org/2000/svg">
        <rect x="10" y="10" width="80" height="60" fill="#95E1D3"/>
        <polygon points="150,20 190,80 110,80" fill="#F38181"/>
        <line x1="10" y1="100" x2="190" y2="100" stroke="#666" stroke-width="2"/>
        <path d="M 50 120 Q 100 110 150 120" stroke="#4ECDC4" stroke-width="2" fill="none"/>
    </svg>"##;
    let img2 = elements::Image::from_svg_string(svg2)
        .expect("Failed to load SVG");
    doc.push(img2);

    // Example 3: Scaled SVG
    doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("3. SVG with scaling (50% of page width)"));
    let svg3 = r##"<svg width="300" height="100" xmlns="http://www.w3.org/2000/svg">
        <rect width="300" height="100" fill="#FFE66D"/>
        <circle cx="60" cy="50" r="30" fill="#A8D8EA"/>
        <circle cx="150" cy="50" r="30" fill="#FF9AA2"/>
        <circle cx="240" cy="50" r="30" fill="#AA96DA"/>
    </svg>"##;
    let img3 = elements::Image::from_svg_string(svg3)
        .expect("Failed to load SVG")
        .resizing_page_with(0.5);
    doc.push(img3);

    // Example 4: Rotated SVG
    doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("4. SVG with rotation (45 degrees)"));
    let svg4 = r##"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
        <rect x="10" y="10" width="80" height="80" fill="#FCBAD3"/>
        <circle cx="50" cy="50" r="20" fill="#FF1493"/>
    </svg>"##;
    let img4 = elements::Image::from_svg_string(svg4)
        .expect("Failed to load SVG")
        .with_clockwise_rotation(genpdfi_extended::Rotation::from_degrees(45.0));
    doc.push(img4);

    // Example 5: Complex SVG with multiple elements
    doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("5. Complex SVG illustration"));
    let svg5 = r##"<svg width="200" height="180" xmlns="http://www.w3.org/2000/svg">
        <!-- Sun -->
        <circle cx="170" cy="30" r="25" fill="#FFD93D"/>
        <!-- Mountains -->
        <polygon points="0,120 80,40 160,100 200,50 200,180 0,180" fill="#6BCB77"/>
        <!-- Clouds -->
        <ellipse cx="40" cy="50" rx="25" ry="15" fill="#F0F0F0"/>
        <ellipse cx="60" cy="55" rx="20" ry="12" fill="#F0F0F0"/>
    </svg>"##;
    let img5 = elements::Image::from_svg_string(svg5)
        .expect("Failed to load SVG");
    doc.push(img5);

    // Exemple 6: SVG Formula
        doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("6. SVG Formula"));
    let svg6 = include_str!("./images/out_math.svg");
    let img6 = elements::Image::from_svg_string(svg6)
        .expect("Failed to load SVG")
        .resizing_page_with(0.5);
    doc.push(img6);

    doc.push(elements::Paragraph::new(""));
    doc.push(elements::Paragraph::new("All SVG examples rendered as vector graphics without rasterization."));

    // Generate the output PDF
    let mut pdf_file = fs::File::create(&out_dir.join("svg_rendering.pdf"))
        .expect("create output file");
    doc.render(&mut pdf_file).expect("render document");
    println!("✓ Created examples/output/svg_rendering.pdf");
    }
}
