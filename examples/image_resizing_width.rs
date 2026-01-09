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
        println!("Running example: image_resizing_width");

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
    doc.set_title("Image Resizing: width-based");

    // Image path
    let image_path = "examples/images/test_image.jpg";

    // Original image
    doc.push(elements::Paragraph::new("1. Original image"));
    let original = elements::Image::from_path(image_path)
        .expect("Failed to load image")
        .with_alignment(Alignment::Center);
    doc.push(original);

    doc.push(elements::Paragraph::new(""));

    // Resized image: 50% of page available width
    doc.push(elements::Paragraph::new("2. Resized to 50% of page width"));
    let resized = elements::Image::from_path(image_path)
        .expect("Failed to load image")
        .resizing_page_with(0.5)
        .with_alignment(Alignment::Center);
    doc.push(resized);

    // Generate the output PDF
    let mut pdf_file = fs::File::create(&out_dir.join("image_resizing_width.pdf"))
        .expect("create output file");
    doc.render(&mut pdf_file).expect("render document");
    println!("✓ Created examples/output/image_resizing_width.pdf");
    }
}
