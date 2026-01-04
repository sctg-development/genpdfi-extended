use std::fs;
use std::path::PathBuf;
use std::process::Command;

use genpdfi_extended::fonts::{FontCache, FontData, FontFamily};
use genpdfi_extended::render::Renderer;
use genpdfi_extended::{Mm, Position, Size};

fn main() {
    println!("Running example: basic_and_structural");

    // Prepare output dir
    let out_dir = PathBuf::from("examples/output");
    fs::create_dir_all(&out_dir).expect("create examples/output dir");

    // Create renderer and add some text using bundled font
    let mut r = Renderer::new(Size::new(210.0, 297.0), "ex_basic").expect("renderer");
    let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
    // Use a built-in font for example output to ensure visible text across viewers.
    let fd = FontData::new(data.clone(), Some(printpdf::BuiltinFont::Helvetica)).expect("font data");
    let family = FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };
    let mut cache = FontCache::new(family);
    cache.load_pdf_fonts(&mut r).expect("load fonts");

    let area = r.first_page().first_layer().area();
    let s = "Hello example: basic_and_structural";
    // For debugging: print glyph ids for the default regular font
    let family = cache.default_font_family();
    let reg_font = family.regular;
    let glyphs: Vec<u16> = reg_font.glyph_ids(&cache, s.chars());
    println!("Glyph ids for '{}': {:?}", s, glyphs);
    area.print_str(&cache, Position::new(Mm::from(10.0), Mm::from(280.0)), genpdfi_extended::style::Style::new(), s).expect("print");


    // Write out PDF
    let out_path = out_dir.join("example_basic.pdf");
    let mut buf = Vec::new();
    r.write(&mut buf).expect("write");
    std::fs::write(&out_path, &buf).expect("write file");
    println!("Wrote {}", out_path.display());

    // Do a simple structural parse (printpdf) to confirm and print ops for debugging
    let mut warnings = Vec::new();
    let parsed = printpdf::PdfDocument::parse(&buf, &printpdf::PdfParseOptions::default(), &mut warnings).expect("parse");
    println!("Parsed PDF pages: {}", parsed.pages.len());
    // print font resource keys (parser exposes map)
    println!("Parsed resources fonts: {:?}", parsed.resources.fonts.map.keys().collect::<Vec<_>>());
    if !parsed.pages.is_empty() {
        println!("Page 0 ops:");
        for op in parsed.pages[0].ops.iter() {
            println!("  {:?}", op);
        }
    }

    // Optionally run the visual validator (if you have PyMuPDF + Pillow installed)
    let py_status = Command::new("python3")
        .arg("tests/scripts/validate_pdf.py")
        .arg("--pdf")
        .arg(out_path.to_str().unwrap())
        .status();

    match py_status {
        Ok(s) => {
            if let Some(code) = s.code() {
                if code == 77 {
                    println!("Python validator skipped: PyMuPDF/Pillow not available");
                } else if s.success() {
                    println!("Python visual validator: OK");
                } else {
                    println!("Python visual validator failed (exit code {})", code);
                }
            }
        }
        Err(e) => {
            println!("Could not run python validator: {}", e);
        }
    }
}
