#![cfg(feature = "images")]

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, Alignment, Document};

fn main() {
    println!("Running example: image_resizing_height");

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
    doc.set_title("Image Resizing: height-based");

    // Image path
    let image_path = "examples/images/triangle-ruler-1016726_640.png";

    // Original image
    doc.push(elements::Paragraph::new("1. Original image"));
    let original = elements::Image::from_path(image_path)
        .expect("Failed to load image")
        .with_alignment(Alignment::Center);
    doc.push(original);

    doc.push(elements::Paragraph::new(""));

    // Resized image: 30% of page available height
    doc.push(elements::Paragraph::new("2. Resized to 30% of page height"));
    let resized = elements::Image::from_path(image_path)
        .expect("Failed to load image")
        .resizing_page_height(0.3)
        .with_alignment(Alignment::Center);
    doc.push(resized);

    // Generate the output PDF
    let mut pdf_file =
        fs::File::create(&out_dir.join("image_resizing_height.pdf")).expect("create output file");
    doc.render(&mut pdf_file).expect("render document");
    println!("✓ Created examples/output/image_resizing_height.pdf");
}
