//! Example: Advanced LaTeX Features - Inline and Positioned Formulas
//!
//! This example demonstrates advanced usage of the Latex element including:
//! - Inline formulas (integrated within text)
//! - Positioned formulas (at specific coordinates)
//! - Mixed content with text and formulas
//!
//! Run with: cargo run --example latex_advanced --features "images,latex"

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, style, Alignment, Document, Position};

fn main() {
    if !cfg!(feature = "latex") {
        eprintln!("This example requires the 'latex' feature to be enabled.");
        eprintln!("Run with: cargo run --example latex_advanced --features 'images,latex'");
        return;
    }

    #[cfg(feature = "latex")]
    {
        println!("Generating advanced LaTeX features example...\n");

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
        doc.set_title("Advanced LaTeX Features");

        // Title
        doc.push(elements::Paragraph::new("")
            .styled_string(
                "Advanced LaTeX Features",
                style::Style::new().with_font_size(18).bold(),
            ));
        doc.push(elements::Paragraph::new(""));

        // Section 1: Block Formulas (default behavior)
        doc.push(elements::Paragraph::new("").styled_string(
            "1. Block Formulas",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(
            "Block formulas stand alone on their own line, centered or aligned:",
        ));
        doc.push(elements::Paragraph::new(""));

        doc.push(elements::Latex::new(r#"\[y = mx + b\]"#, 12.0)
            .with_alignment(Alignment::Center));
        doc.push(elements::Paragraph::new(""));

        // Section 2: Inline Formulas (theoretical)
        doc.push(elements::Paragraph::new("").styled_string(
            "2. Inline Formula Mode",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(
            "Inline mode allows formulas to render in formula-rendering context.",
        ));
        doc.push(elements::Paragraph::new(""));

        // Create inline formula
        let inline_formula = elements::Latex::new(r#"x^2 + y^2 = r^2"#, 10.0).inline();
        doc.push(inline_formula);
        doc.push(elements::Paragraph::new(""));

        // Section 3: Positioned Formulas
        doc.push(elements::Paragraph::new("").styled_string(
            "3. Positioned Formulas",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(
            "Formulas can be placed at specific positions on the page.",
        ));
        doc.push(elements::Paragraph::new(""));

        // Positioned formula (at coordinates 20, 100)
        doc.push(elements::Latex::new(r#"\[E = mc^2\]"#, 14.0)
            .with_position(Position::new(20, 100)));
        doc.push(elements::Paragraph::new(""));

        // Section 4: Different Size Specifications
        doc.push(elements::Paragraph::new("").styled_string(
            "4. Size Specifications",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(
            "The size parameter uses 'pseudo points' (relative to text sizes):",
        ));
        doc.push(elements::Paragraph::new(""));

        let sizes = vec![
            (8.0, "8pt - Very Small"),
            (10.0, "10pt - Small"),
            (12.0, "12pt - Normal"),
            (14.0, "14pt - Large"),
            (16.0, "16pt - X-Large"),
        ];

        for (size, label) in sizes {
            doc.push(elements::Paragraph::new(&format!("{}:", label)));
            doc.push(elements::Latex::new(r#"\[\sqrt[3]{a^2 + b^2}\]"#, size)
                .with_alignment(Alignment::Center));
            doc.push(elements::Paragraph::new(""));
        }

        // Section 5: Alignment Options
        doc.push(elements::Paragraph::new("").styled_string(
            "5. Alignment Options",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(""));

        doc.push(elements::Paragraph::new("Left aligned:"));
        doc.push(elements::Latex::new(r#"\[\lim_{x \to \infty} \frac{1}{x} = 0\]"#, 12.0)
            .with_alignment(Alignment::Left));
        doc.push(elements::Paragraph::new(""));

        doc.push(elements::Paragraph::new("Center aligned:"));
        doc.push(elements::Latex::new(r#"\[\lim_{x \to \infty} \frac{1}{x} = 0\]"#, 12.0)
            .with_alignment(Alignment::Center));
        doc.push(elements::Paragraph::new(""));

        doc.push(elements::Paragraph::new("Right aligned:"));
        doc.push(elements::Latex::new(r#"\[\lim_{x \to \infty} \frac{1}{x} = 0\]"#, 12.0)
            .with_alignment(Alignment::Right));
        doc.push(elements::Paragraph::new(""));

        // Section 6: Mathematical Notation
        doc.push(elements::Paragraph::new("").styled_string(
            "6. Mathematical Notation Examples",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(""));

        let examples = vec![
            ("Greek Letters", r#"\[\alpha + \beta = \gamma\]"#),
            ("Superscripts & Subscripts", r#"\[x^2 + y_1 + z_{n-1}\]"#),
            ("Fractions", r#"\[\frac{\partial f}{\partial x} = \frac{df}{dx}\]"#),
            ("Radicals", r#"\[\sqrt{2} + \sqrt[3]{x} = y\]"#),
            ("Summation", r#"\[\sum_{i=1}^{\infty} \frac{1}{i^2} = \frac{\pi^2}{6}\]"#),
            ("Integration", r#"\[\int_0^{2\pi} \sin(x) \, dx = 0\]"#),
        ];

        for (name, formula) in examples {
            doc.push(elements::Paragraph::new(&format!("{}:", name)));
            doc.push(elements::Latex::new(formula, 12.0)
                .with_alignment(Alignment::Center));
            doc.push(elements::Paragraph::new(""));
        }

        // Output document
        let output_path = out_dir.join("latex_advanced.pdf");
        doc.render_to_file(&output_path)
            .expect("Failed to render PDF");

        println!("{}", "=".repeat(70));
        println!("✓ Advanced LaTeX example PDF generated: {}", output_path.display());
        println!("{}", "=".repeat(70));
        println!("\nFeatures demonstrated:");
        println!("  • Block formulas with different alignments");
        println!("  • Positioned formulas at specific coordinates");
        println!("  • Multiple size specifications (8pt to 16pt)");
        println!("  • Alignment options (Left, Center, Right)");
        println!("  • Complex mathematical notation");
        println!("  • Greek letters, fractions, radicals, summation, integration");
    }
}
