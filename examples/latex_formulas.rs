// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

/// Example: 10 LaTeX Formulas with Multi-Size Calibration
///
/// This example demonstrates formula rendering at multiple font sizes (10pt, 12pt, 14pt, 16pt, 18pt).
/// For each size, it uses compute_scale_factor() to calibrate the key character height.
/// Different subsets of the 10 formulas are rendered at each size to validate the calibration approach.
use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, style, Document};

fn main() {
    if !cfg!(feature = "images") {
        eprintln!("This example requires the 'images' feature to be enabled.");
        return;
    }

    #[cfg(feature = "images")]
    {
        println!("Generating LaTeX formulas example with MicroTeX...\n");

        // Prepare output directory
        let out_dir = PathBuf::from("examples/output");
        fs::create_dir_all(&out_dir).expect("create examples/output dir");

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

        // Initialize MicroTeX renderer ONCE
        let renderer = match microtex_rs::MicroTex::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to initialize MicroTeX: {}", e);
                return;
            }
        };

        // Create document
        let mut doc = Document::new(family);
        doc.set_title("LaTeX Formulas - Multi-Size Calibration");

        // Title
        doc.push(elements::Paragraph::new(
            "LaTeX Formula Rendering - Multi-Size Calibration",
        ));
        doc.push(elements::Paragraph::new(
            "Testing key character sizing across multiple font sizes",
        ));
        doc.push(elements::Paragraph::new(""));

        // Define 10 formulas with proper LaTeX delimiters
        let formulas = vec![
            (
                "Peak Frequency Formula",
                r#"\[f_{peak} = f_k + \frac{\delta f}{2} \cdot \frac{m_{k-1} - m_{k+1}}{m_{k-1} - 2m_k + m_{k+1}}\]"#,
            ),
            ("Pythagorean Theorem", r#"x^2 + y^2 = z^2"#),
            ("Gaussian Integral", r#"\int_0^\infty e^{-x} dx"#),
            ("Mass-Energy Equivalence", r#"E = mc^2"#),
            ("Pythagorean Triple", r#"a^2 + b^2 = c^2"#),
            (
                "Quadratic Formula",
                r#"\[x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}\]"#,
            ),
            ("Sum Formula", r#"\[\sum_{i=1}^{n} i = \frac{n(n+1)}{2}\]"#),
            ("Sphere Volume", r#"\[V = \frac{4}{3}\pi r^3\]"#),
            ("Circle Area", r#"A = \pi r^2"#),
            ("Electric Power", r#"P = VI"#),
        ];

        // Test multiple sizes: 10pt, 12pt, 14pt, 16pt, 18pt
        let sizes = vec![10.0, 12.0, 14.0, 16.0, 18.0];

        for size_pt in sizes {
            println!("\n=== CALIBRATION FOR {}pt ===", size_pt as i32);

            // Compute scale factor for this size
            let (_ref_height, target_height_px, scale_factor) =
                match compute_scale_factor(&renderer, size_pt) {
                    Some(value) => value,
                    None => {
                        eprintln!("Failed to compute scale factor for {}pt", size_pt);
                        continue;
                    }
                };

            println!(
                "Target height: {:.1}px (at 720 DPI) | Scale factor: {:.4}\n",
                target_height_px, scale_factor
            );

            // Create style for this font size
            let text_style = style::Style::new().with_font_size(size_pt as u8);

            // Add section header
            doc.push(elements::Paragraph::new(""));
            doc.push(elements::Paragraph::new("").styled_string(
                format!("--- {}pt Font Size ---", size_pt as i32),
                text_style.clone(),
            ));
            doc.push(elements::Paragraph::new("").styled_string(
                format!("Scale factor: {:.4}", scale_factor),
                text_style.clone(),
            ));
            doc.push(elements::Paragraph::new(""));

            // Render a subset of formulas (3 formulas per size to keep file manageable)
            let subset_indices = match size_pt as i32 {
                10 => vec![0, 1, 2],
                12 => vec![3, 4, 5],
                14 => vec![6, 7, 8],
                16 => vec![9, 0, 3],
                18 => vec![2, 5, 8],
                _ => vec![],
            };

            for &idx in &subset_indices {
                if idx < formulas.len() {
                    let (title, latex) = formulas[idx];
                    doc.push(elements::Paragraph::new("").styled_string(title, text_style.clone()));

                    let latex_display = format!("LaTeX: {}", latex);
                    doc.push(
                        elements::Paragraph::new("")
                            .styled_string(latex_display, text_style.clone()),
                    );
                    doc.push(elements::Paragraph::new(""));

                    match render_formula_svg_scaled_with_renderer(latex, scale_factor, &renderer) {
                        Ok(svg) => match elements::Image::from_svg_string(&svg) {
                            Ok(img) => {
                                doc.push(img);
                                println!("✓ {}pt: {}", size_pt as i32, title);
                            }
                            Err(e) => {
                                doc.push(elements::Paragraph::new("").styled_string(
                                    format!("[SVG failed: {}]", e),
                                    text_style.clone(),
                                ));
                                println!("✗ {}pt: SVG failed: {}: {}", size_pt as i32, title, e);
                            }
                        },
                        Err(e) => {
                            doc.push(elements::Paragraph::new("").styled_string(
                                format!("[Render failed: {}]", e),
                                text_style.clone(),
                            ));
                            println!("✗ {}pt: Render failed: {}: {}", size_pt as i32, title, e);
                        }
                    }

                    doc.push(elements::Paragraph::new(""));
                }
            }
        }

        doc.push(elements::Paragraph::new(""));
        doc.push(elements::Paragraph::new(
            "Note: Each section uses compute_scale_factor() to calibrate the key character height.",
        ));
        doc.push(elements::Paragraph::new(
            "The 'm' character in each formula should match the height of 'm' in the surrounding text.",
        ));

        // Output document
        let output_path = out_dir.join("latex_formulas.pdf");
        doc.render_to_file(&output_path)
            .expect("Failed to render PDF");

        println!("\n{}", "=".repeat(70));
        println!("✓ PDF generated: {}", output_path.display());
        println!("{}", "=".repeat(70));
    }
}

/// Compute scale factor to match key character height (e.g., 'm') to a target height
fn compute_scale_factor(
    renderer: &microtex_rs::MicroTex,
    target_height_pt: f32,
) -> Option<(f32, f32, f32)> {
    const EMPIRICAL_ADJUSTMENT_FACTOR: f32 = 4.5;
    let reference_svg = match render_formula_svg_with_renderer(r#"\[m\]"#, renderer) {
        Ok(svg) => svg,
        Err(e) => {
            eprintln!("Failed to render reference: {}", e);
            return None;
        }
    };
    let ref_height = extract_height(&reference_svg).unwrap_or(100.0);
    println!(
        "  Reference formula 'm' SVG height: {:.1}px (at 720 DPI)\n",
        ref_height
    );
    let target_height_px = target_height_pt * 10.0;
    let mut scale_factor = target_height_px / ref_height;
    scale_factor = scale_factor / EMPIRICAL_ADJUSTMENT_FACTOR;
    Some((ref_height, target_height_px, scale_factor))
}

/// Extract height from SVG attributes
fn extract_height(svg: &str) -> Option<f32> {
    if let Some(height_start) = svg.find("height=\"") {
        if let Some(height_end) = svg[height_start + 8..].find("\"") {
            let height_str = &svg[height_start + 8..height_start + 8 + height_end];
            return height_str.parse::<f32>().ok();
        }
    }
    None
}

/// Render LaTeX formula to SVG with MicroTeX (base rendering, no scaling)
fn render_formula_svg_with_renderer(
    latex: &str,
    renderer: &microtex_rs::MicroTex,
) -> Result<String, Box<dyn std::error::Error>> {
    // Use the provided renderer instance
    let config = microtex_rs::RenderConfig {
        dpi: 720,                // Higher DPI for better quality
        line_width: 20.0,        // Standard line width
        line_height: 20.0 / 3.0, // Standard line height
        text_color: 0xff000000,  // Black
        has_background: false,
        render_glyph_use_path: true,
        ..Default::default()
    };

    let svg = renderer.render(latex, &config)?;
    Ok(svg)
}

/// Render and scale formula to match key character size
fn render_formula_svg_scaled_with_renderer(
    latex: &str,
    scale_factor: f32,
    renderer: &microtex_rs::MicroTex,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut svg = render_formula_svg_with_renderer(latex, renderer)?;

    // Extract original dimensions using regex-like parsing
    let mut orig_width: f32 = 100.0;
    let mut orig_height: f32 = 100.0;

    // Parse width attribute more robustly
    if let Some(width_start) = svg.find("width=\"") {
        let width_attr_start = width_start + 7;
        if let Some(width_end) = svg[width_attr_start..].find("\"") {
            let width_str = &svg[width_attr_start..width_attr_start + width_end];
            if let Ok(w) = width_str.parse::<f32>() {
                orig_width = w;
            }
        }
    }

    // Parse height attribute more robustly
    if let Some(height_start) = svg.find("height=\"") {
        let height_attr_start = height_start + 8;
        if let Some(height_end) = svg[height_attr_start..].find("\"") {
            let height_str = &svg[height_attr_start..height_attr_start + height_end];
            if let Ok(h) = height_str.parse::<f32>() {
                orig_height = h;
            }
        }
    }

    // Calculate new dimensions
    let new_width = orig_width * scale_factor;
    let new_height = orig_height * scale_factor;

    // Replace only width and height attributes (more carefully)
    // Use a more precise pattern to avoid false matches
    let width_pattern = format!("width=\"{}\"", orig_width);
    let height_pattern = format!("height=\"{}\"", orig_height);

    if svg.contains(&width_pattern) {
        svg = svg.replacen(
            &width_pattern,
            &format!("width=\"{}\"", new_width as i32),
            1,
        );
    } else {
        // Fallback: if exact match fails, try to replace in <svg tag
        if let Some(svg_tag_end) = svg.find(">") {
            let svg_tag = &svg[..svg_tag_end];
            if svg_tag.contains("width=") {
                svg = svg.replacen(
                    &format!("width=\"{}\"", orig_width as i32),
                    &format!("width=\"{}\"", new_width as i32),
                    1,
                );
            }
        }
    }

    if svg.contains(&height_pattern) {
        svg = svg.replacen(
            &height_pattern,
            &format!("height=\"{}\"", new_height as i32),
            1,
        );
    } else {
        // Fallback
        if let Some(svg_tag_end) = svg.find(">") {
            let svg_tag = &svg[..svg_tag_end];
            if svg_tag.contains("height=") {
                svg = svg.replacen(
                    &format!("height=\"{}\"", orig_height as i32),
                    &format!("height=\"{}\"", new_height as i32),
                    1,
                );
            }
        }
    }

    // Ensure viewBox is set for proper scaling
    if !svg.contains("viewBox") {
        let viewbox = format!(
            "viewBox=\"0 0 {} {}\"",
            orig_width as i32, orig_height as i32
        );
        svg = svg.replacen("<svg ", &format!("<svg {} ", viewbox), 1);
    }

    Ok(svg)
}
