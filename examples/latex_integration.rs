// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

//! Example: LaTeX Formulas Integration with genpdfi_extended
//!
//! This example demonstrates how to use the new `Latex` element to render
//! LaTeX formulas directly in PDF documents.
//!
//! Run with: cargo run --example latex_integration --features "images,latex"

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, style, Alignment, Document};

fn main() {
    if !cfg!(feature = "latex") {
        eprintln!("This example requires the 'latex' feature to be enabled.");
        eprintln!("Run with: cargo run --example latex_integration --features 'images,latex'");
        return;
    }

    #[cfg(feature = "latex")]
    {
        println!("Generating LaTeX integration example...\n");

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

        // Create document
        let mut doc = Document::new(family);
        doc.set_title("LaTeX Formulas - Direct Integration");

        // Title
        doc.push(elements::Paragraph::new(
            "LaTeX Formulas - Direct Integration with PDF",
        ));
        doc.push(elements::Paragraph::new(
            "Using the new Latex element for seamless formula rendering",
        ));
        doc.push(elements::Paragraph::new(""));

        // Section 1: Basic formulas
        doc.push(
            elements::Paragraph::new("Section 1: Basic Formulas").styled_string(
                "Section 1: Basic Formulas",
                style::Style::new().with_font_size(14),
            ),
        );
        doc.push(elements::Paragraph::new(""));

        // Pythagorean theorem centered
        doc.push(
            elements::Paragraph::new("The Pythagorean theorem:")
                .styled_string("The Pythagorean theorem:", style::Style::new()),
        );
        doc.push(
            elements::Latex::new(r#"a^2 + b^2 = c^2"#, 14.0).with_alignment(Alignment::Center),
        );
        doc.push(elements::Paragraph::new(""));

        // Section 2: Complex formulas at different sizes
        doc.push(
            elements::Paragraph::new("Section 2: Formulas at Different Sizes").styled_string(
                "Section 2: Formulas at Different Sizes",
                style::Style::new().with_font_size(14),
            ),
        );
        doc.push(elements::Paragraph::new(""));

        // 10pt formula
        doc.push(elements::Paragraph::new("Small (10pt):"));
        doc.push(elements::Latex::new(r#"\[E = mc^2\]"#, 10.0));
        doc.push(elements::Paragraph::new(""));

        // 12pt formula
        doc.push(elements::Paragraph::new("Normal (12pt):"));
        doc.push(elements::Latex::new(
            r#"\[f_{peak} = f_k + \frac{\delta f}{2} \cdot \frac{m_{k-1} - m_{k+1}}{m_{k-1} - 2m_k + m_{k+1}}\]"#,
            12.0,
        ));
        doc.push(elements::Paragraph::new(""));

        // 14pt formula
        doc.push(elements::Paragraph::new("Large (14pt):"));
        doc.push(
            elements::Latex::new(r#"\[\int_0^\infty e^{-x} dx = 1\]"#, 14.0)
                .with_alignment(Alignment::Center),
        );
        doc.push(elements::Paragraph::new(""));

        // Section 3: More complex formulas
        doc.push(
            elements::Paragraph::new("Section 3: Complex Mathematical Expressions").styled_string(
                "Section 3: Complex Mathematical Expressions",
                style::Style::new().with_font_size(14),
            ),
        );
        doc.push(elements::Paragraph::new(""));

        let formulas = vec![
            (
                "Quadratic Formula",
                r#"\[x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}\]"#,
            ),
            ("Sum Notation", r#"\[\sum_{i=1}^{n} i = \frac{n(n+1)}{2}\]"#),
            ("Sphere Volume", r#"\[V = \frac{4}{3}\pi r^3\]"#),
            ("Circle Area", r#"A = \pi r^2"#),
        ];

        for (title, formula) in formulas {
            doc.push(elements::Paragraph::new(title));
            doc.push(elements::Latex::new(formula, 12.0).with_alignment(Alignment::Center));
            doc.push(elements::Paragraph::new(""));
        }

        // Notes
        doc.push(elements::Paragraph::new(""));
        doc.push(
            elements::Paragraph::new("Notes:")
                .styled_string("Notes:", style::Style::new().with_font_size(11).bold()),
        );
        doc.push(elements::Paragraph::new(
            "• Formulas are rendered using MicroTeX at 720 DPI for high quality",
        ));
        doc.push(elements::Paragraph::new(
            "• The size parameter is in 'pseudo points' (relative to text font sizes)",
        ));
        doc.push(elements::Paragraph::new(
            "• Each formula can be centered, left-aligned, or at an explicit position",
        ));
        doc.push(elements::Paragraph::new(
            "• SVG formulas scale perfectly without quality loss",
        ));

        // Output document
        let output_path = out_dir.join("latex_integration.pdf");
        doc.render_to_file(&output_path)
            .expect("Failed to render PDF");

        println!("\n{}", "=".repeat(70));
        println!("✓ PDF generated: {}", output_path.display());
        println!("{}", "=".repeat(70));
    }
}
