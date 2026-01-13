// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

// This example uses the `images` feature. It is safe to compile without that
// feature enabled because `main` will early-exit when the feature is not active.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use genpdfi_extended::render::Renderer;
use genpdfi_extended::{Mm, Position, Rotation, Scale, Size};

fn main() {
    println!("Running example: image_positions (feature 'images' required)");

    // Prepare output dir
    let out_dir = PathBuf::from("examples/output");
    fs::create_dir_all(&out_dir).expect("create examples/output dir");

    let img_path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/images/test_image.jpg"
    ));
    // If the images feature is not enabled, or no image is present, skip the example.
    if cfg!(not(feature = "images")) {
        eprintln!("images feature not enabled; skipping example");
        return;
    }
    if !img_path.exists() {
        eprintln!("No test image found at {} - copy one to examples/images/test_image.jpg to run this example", img_path.display());
        return;
    }

    // Load NotoSans and SpaceMono into a FontCache so we can render textual labels
    use genpdfi_extended::fonts::{FontCache, FontData, FontFamily};
    let data_noto = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd_noto = FontData::new(data_noto.clone(), None).expect("font data");
    let family_noto = FontFamily {
        regular: fd_noto.clone(),
        bold: fd_noto.clone(),
        italic: fd_noto.clone(),
        bold_italic: fd_noto.clone(),
    };

    let data_space_reg = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/SpaceMono-Regular.ttf"
    ))
    .to_vec();
    let data_space_bold = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/SpaceMono-Bold.ttf"
    ))
    .to_vec();
    let fd_space_reg = FontData::new(data_space_reg.clone(), None).expect("space font data");
    let fd_space_bold = FontData::new(data_space_bold.clone(), None).expect("space bold data");
    let family_space = FontFamily {
        regular: fd_space_reg.clone(),
        bold: fd_space_bold.clone(),
        italic: fd_space_reg.clone(),
        bold_italic: fd_space_bold.clone(),
    };

    let mut cache = FontCache::new(family_noto);
    let space_family = cache.add_font_family(family_space);

    // Prepare a builtin cache (Helvetica) for a literal fallback label
    use printpdf::BuiltinFont;
    let data_builtin = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd_builtin =
        FontData::new(data_builtin, Some(BuiltinFont::Helvetica)).expect("builtin font data");
    let family_builtin = FontFamily {
        regular: fd_builtin.clone(),
        bold: fd_builtin.clone(),
        italic: fd_builtin.clone(),
        bold_italic: fd_builtin.clone(),
    };
    let mut builtin_cache = FontCache::new(family_builtin);

    let positions = vec![(10.0, 250.0), (100.0, 150.0), (150.0, 50.0)]; // (x_mm, y_mm top-left)

    for (i, (x_mm, y_mm)) in positions.into_iter().enumerate() {
        let mut r = Renderer::new(Size::new(210.0, 297.0), &format!("img_example_{}", i))
            .expect("renderer");
        // Ensure PDF fonts are embedded into this renderer instance
        cache.load_pdf_fonts(&mut r).expect("load fonts");
        builtin_cache
            .load_pdf_fonts(&mut r)
            .expect("load builtin fonts");
        let area = r.first_page().first_layer().area();
        // Use conditional compilation to avoid requiring the `image` crate when the
        // `images` feature is not enabled. When enabled, load the image and add it to the area.
        #[cfg(feature = "images")]
        {
            let img = image::open(&img_path).expect("open image");
            area.add_image(
                &img,
                Position::new(Mm::from(x_mm), Mm::from(y_mm)),
                Scale::new(1.0, 1.0),
                Rotation::from_degrees(0.0),
                Some(150.0),
            );
        }

        // Add textual label showing the image position: NotoSans-Regular 20 and SpaceMono-Bold 10
        let mut label = format!("Image position: {:.2}, {:.2} top-left", x_mm, y_mm);
        let s1 = genpdfi_extended::style::Style::new()
            .with_font_family(cache.default_font_family())
            .with_font_size(20);
        // Add font info to label
        label.push_str(&format!(" Builtin (default_font_family): size 20pt"));
        area.print_str(
            &cache,
            Position::new(Mm::from(10.0), Mm::from(280.0)),
            s1,
            &label,
        )
        .expect("print label");
        let s2 = genpdfi_extended::style::Style::new()
            .with_font_family(space_family)
            .bold()
            .with_font_size(10);
        // Add font info to label
        label.push_str(&format!(" | SpaceMono-Bold: size 10pt"));
        area.print_str(
            &cache,
            Position::new(Mm::from(10.0), Mm::from(270.0)),
            s2,
            &label,
        )
        .expect("print label mono");

        // Emit the builtin Helvetica literal label (12pt)
        let s_builtin = genpdfi_extended::style::Style::new()
            .with_font_family(builtin_cache.default_font_family())
            .with_font_size(12);
        label = format!("Literal label in Builtin 12pt at {:.2}, {:.2}", 10.0, 260.0);
        area.print_str(
            &builtin_cache,
            Position::new(Mm::from(10.0), Mm::from(260.0)),
            s_builtin,
            &label,
        )
        .expect("print builtin label");

        // Emit a SpaceMono label at bottom-left
        let s_bottom = genpdfi_extended::style::Style::new()
            .with_font_family(space_family)
            .with_font_size(10);
        label = format!(
            "Bottom-left label in SpaceMono-Regular 10pt at {:.2}, {:.2}",
            10.0, 10.0
        );
        area.print_str(
            &cache,
            Position::new(Mm::from(10.0), Mm::from(10.0)),
            s_bottom,
            &label,
        )
        .expect("print bottom label");

        let out_path = out_dir.join(format!("example_imgpos_{}.pdf", i));
        let mut f = std::fs::File::create(&out_path).expect("create file");
        r.write(&mut f).expect("write pdf");
        println!("Wrote {}", out_path.display());

        // Parse and print matrix info for the first UseXobject encountered (helpful for inspection)
        let mut buf = Vec::new();
        let mut f = std::fs::File::open(&out_path).expect("open pdf for parse");
        use std::io::Read as _;
        f.read_to_end(&mut buf).expect("read pdf");
        let mut warnings = Vec::new();
        let parsed = printpdf::PdfDocument::parse(
            &buf,
            &printpdf::PdfParseOptions::default(),
            &mut warnings,
        )
        .expect("parse");
        for page in parsed.pages.iter() {
            let mut last_matrix: Option<[f64; 6]> = None;
            for op in page.ops.iter() {
                let s = format!("{:?}", op);
                if let Some(start) = s.find("Raw([") {
                    if let Some(rel_end) = s[start..].find("])") {
                        let nums = &s[start + 5..start + rel_end];
                        let parts: Vec<&str> = nums
                            .split(',')
                            .map(|p| p.trim())
                            .filter(|p| !p.is_empty())
                            .collect();
                        if parts.len() == 6 {
                            let mut vals = [0f64; 6];
                            for i in 0..6 {
                                if let Ok(v) = parts[i].parse::<f64>() {
                                    vals[i] = v;
                                }
                            }
                            last_matrix = Some(vals);
                        }
                    }
                }

                if let printpdf::Op::UseXobject {
                    id: _,
                    transform: _,
                } = op
                {
                    if let Some(mat) = last_matrix {
                        println!(
                            "Observed matrix: [{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}]",
                            mat[0], mat[1], mat[2], mat[3], mat[4], mat[5]
                        );
                        break;
                    }
                }
            }
        }

        // Optionally run the validator (if present)
        let py_status = Command::new("python")
            .arg("tests/scripts/validate_pdf.py")
            .arg("--pdf")
            .arg(out_path.to_str().unwrap())
            .arg("--positions")
            .arg(&format!("{:.2},{:.2},20,20", x_mm + 10.0, y_mm + 5.0))
            .status();

        match py_status {
            Ok(s) => {
                if let Some(code) = s.code() {
                    if code == 77 {
                        println!("Python validator skipped: PyMuPDF/Pillow not available");
                    } else if s.success() {
                        println!("Python visual validator: OK for {}", out_path.display());
                    } else {
                        println!(
                            "Python visual validator failed for {} (exit code {})",
                            out_path.display(),
                            code
                        );
                    }
                }
            }
            Err(e) => println!("Could not run python validator: {}", e),
        }
    }
}
