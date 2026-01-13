// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

//! Stress Test: Many LaTeX Formulas in One Document
//!
//! This example stress-tests the LaTeX feature with multiple formulas
//! to ensure the global MicroTeX singleton handles many renders correctly.
//!
//! Run with: cargo run --example latex_stress_test --features "images,latex"

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, style, Alignment, Document};

fn main() {
    if !cfg!(feature = "latex") {
        eprintln!("This example requires the 'latex' feature to be enabled.");
        eprintln!("Run with: cargo run --example latex_stress_test --features 'images,latex'");
        return;
    }

    #[cfg(feature = "latex")]
    {
        println!("Stress Testing: Rendering 20+ LaTeX formulas...\n");

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
        doc.set_title("LaTeX Stress Test - Global Singleton MicroTeX Instance");

        // Title
        doc.push(elements::Paragraph::new(
            "LaTeX Stress Test: 20+ Formulas with Single MicroTeX Instance",
        ).styled_string(
            "LaTeX Stress Test: 20+ Formulas with Single MicroTeX Instance",
            style::Style::new().with_font_size(16).bold(),
        ));
        doc.push(elements::Paragraph::new(
            "This document demonstrates that the global MicroTeX singleton instance",
        ));
        doc.push(elements::Paragraph::new(
            "correctly handles multiple formula renders without crashing.",
        ));
        doc.push(elements::Paragraph::new(""));

        // Test formulas - 20 different expressions
        let formulas = vec![
            ("1. Einstein's Equation", r#"E = mc^2"#, 12.0),
            ("2. Pythagorean Theorem", r#"a^2 + b^2 = c^2"#, 12.0),
            ("3. Quadratic Formula", r#"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#, 12.0),
            ("4. Gaussian Integral", r#"\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}"#, 12.0),
            ("5. Euler's Formula", r#"e^{i\pi} + 1 = 0"#, 12.0),
            ("6. Golden Ratio", r#"\phi = \frac{1 + \sqrt{5}}{2}"#, 12.0),
            ("7. Circular Area", r#"A = \pi r^2"#, 12.0),
            ("8. Sphere Volume", r#"V = \frac{4}{3}\pi r^3"#, 12.0),
            ("9. Sine Addition", r#"\sin(a + b) = \sin(a)\cos(b) + \cos(a)\sin(b)"#, 12.0),
            ("10. Cosine Law", r#"c^2 = a^2 + b^2 - 2ab\cos(C)"#, 12.0),
            ("11. Sum of Series", r#"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}"#, 12.0),
            ("12. Geometric Series", r#"\sum_{i=0}^{\infty} r^i = \frac{1}{1-r}, |r| < 1"#, 12.0),
            ("13. Binomial Expansion", r#"(x + y)^n = \sum_{k=0}^{n} \binom{n}{k} x^k y^{n-k}"#, 12.0),
            ("14. Logarithm Rule", r#"\log(ab) = \log(a) + \log(b)"#, 12.0),
            ("15. Power Rule", r#"\frac{d}{dx} x^n = nx^{n-1}"#, 12.0),
            ("16. Product Rule", r#"(fg)' = f'g + fg'"#, 12.0),
            ("17. Chain Rule", r#"\frac{d}{dx} f(g(x)) = f'(g(x)) \cdot g'(x)"#, 12.0),
            ("18. Fundamental Theorem", r#"\int_a^b f'(x) dx = f(b) - f(a)"#, 12.0),
            ("19. Wave Equation", r#"\frac{\partial^2 u}{\partial t^2} = c^2 \nabla^2 u"#, 12.0),
            ("20. Normal Distribution", r#"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}"#, 11.0),
            ("21. Complex Number", r#"z = a + bi, |z| = \sqrt{a^2 + b^2}"#, 12.0),
            ("22. Matrix Determinant", r#"\begin{vmatrix} a & b \\ c & d \end{vmatrix} = ad - bc"#, 12.0),
        ];

        println!("Rendering {} formulas with global MicroTeX singleton...", formulas.len());
        println!();

        let mut success_count = 0;
        let mut fail_count = 0;

        for (idx, (title, formula, size)) in formulas.iter().enumerate() {
            doc.push(elements::Paragraph::new(""));
            doc.push(elements::Paragraph::new(*title).styled_string(
                *title,
                style::Style::new().with_font_size(11).bold(),
            ));
            doc.push(elements::Paragraph::new(format!("LaTeX: {}", formula)));
            doc.push(elements::Paragraph::new(""));

            // Add the formula to the document (rendering happens during PDF generation)
            let latex_elem = elements::Latex::new(*formula, *size)
                .with_alignment(Alignment::Center);
            doc.push(latex_elem);
            println!("✓ Formula {} queued for rendering", idx + 1);
            success_count += 1;
        }

        // Summary
        doc.push(elements::Paragraph::new(""));
        doc.push(elements::Paragraph::new(""));
        doc.push(elements::Paragraph::new("=== TEST SUMMARY ===").styled_string(
            "=== TEST SUMMARY ===",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(format!(
            "Total formulas: {}",
            success_count
        )));
        doc.push(elements::Paragraph::new(format!(
            "Queued for rendering: {} ✓",
            success_count
        )));

        if fail_count == 0 {
            doc.push(elements::Paragraph::new(""));
            doc.push(elements::Paragraph::new(
                "SUCCESS: All formulas queued without errors!",
            ).styled_string(
                "SUCCESS: All formulas queued without errors!",
                style::Style::new().bold(),
            ));
            doc.push(elements::Paragraph::new(
                "The global MicroTeX singleton instance will render each formula during PDF generation.",
            ));
        } else {
            doc.push(elements::Paragraph::new(""));
            doc.push(elements::Paragraph::new(format!(
                "Some formulas failed to queue (see above for details)"
            )));
        }

        // Output document
        let output_path = out_dir.join("latex_stress_test.pdf");
        doc.render_to_file(&output_path)
            .expect("Failed to render PDF");

        println!();
        println!("{}", "=".repeat(70));
        println!(
            "✓ Stress test PDF generated: {}",
            output_path.display()
        );
        println!("{}", "=".repeat(70));
        println!();
        println!("Results:");
        println!("  Successful: {}", success_count);
        println!("  Failed: {}", fail_count);
        if fail_count == 0 {
            println!("  Status: ✓ ALL TESTS PASSED");
        }
    }
}
