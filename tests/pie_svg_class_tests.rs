#![cfg(feature = "images")]

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, Document};

fn create_doc_and_render(title: &str, svg: &str, out: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let font_data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
    let fd = fonts::FontData::new(font_data, None)?;
    let family = fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };

    let mut doc = Document::new(family);
    doc.set_title(title);
    doc.push(elements::Paragraph::new(title));

    // Collect parser warnings
    let mut warnings = Vec::new();
    match printpdf::Svg::parse(svg, &mut warnings) {
        Ok(_) => {
            if !warnings.is_empty() {
                doc.push(elements::Paragraph::new("Parser warnings:"));
                for w in warnings.iter() {
                    doc.push(elements::Paragraph::new(format!("- {:?}", w)));
                }
            } else {
                doc.push(elements::Paragraph::new("No parser warnings."));
            }
        }
        Err(e) => {
            doc.push(elements::Paragraph::new(format!("Parser error: {}", e)));
        }
    }

    match elements::Image::from_svg_string(svg) {
        Ok(img) => doc.push(img),
        Err(e) => doc.push(elements::Paragraph::new(format!("Image parse error: {}", e))),
    }

    let mut f = fs::File::create(out)?;
    doc.render(&mut f)?;
    println!("Wrote {}", out.display());
    Ok(())
}

#[test]
fn pie_svg_class_variants_generate_pdf() {
    // Ensure output dir
    let out_dir = PathBuf::from("tests/output");
    fs::create_dir_all(&out_dir).expect("create tests/output");

    // Load the problematic SVG extracted by the user
    let svg = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pie.svg"));
    let out0 = out_dir.join("pie_original.pdf");
    create_doc_and_render("Pie - original", svg, &out0).expect("render original");

    // Variant 1: remove class attributes from pie arcs
    let svg_no_class = svg.replace(" class=\"pieCircle\"", "");
    let out1 = out_dir.join("pie_no_class.pdf");
    create_doc_and_render("Pie - no class", &svg_no_class, &out1).expect("render no_class");

    // Variant 2: inline style attributes (stroke/opacity) into arc paths
    // We add stroke/opacity attributes to the arc path elements.
    let svg_inlined = svg.replace("<path d=", "<path stroke=\"black\" stroke-width=\"2\" opacity=\"0.7\" d=");
    let out2 = out_dir.join("pie_inlined.pdf");
    create_doc_and_render("Pie - inlined styles", &svg_inlined, &out2).expect("render inlined");
}