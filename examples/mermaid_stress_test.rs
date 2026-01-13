// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

//! Stress Test: Many Mermaid Diagrams in One Document
//!
//! This example renders multiple Mermaid diagrams to verify the `mermaid` feature
//! and the transformation to SVG embedded in the PDF work as expected.
//!
//! Run with: cargo run --example mermaid_stress_test --features "images,mermaid"

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, style, Alignment, Document};

fn main() {
    if !cfg!(feature = "mermaid") {
        eprintln!("This example requires the 'mermaid' feature to be enabled.");
        eprintln!("Run with: cargo run --example mermaid_stress_test --features 'images,mermaid'");
        return;
    }

    #[cfg(feature = "mermaid")]
    {
        // Quick runtime check: ensure headless Chrome is available before building the PDF.
        // This avoids a long error during PDF generation on systems without Chrome. We use the
        // shared singleton so the example also warms up the global browser instance.
        match elements::Mermaid::ensure_browser() {
            Ok(_) => println!("Headless Chrome appears usable; proceeding with Mermaid stress test..."),
            Err(e) => {
                eprintln!("Headless Chrome is not available or failed to start: {}", e);
                eprintln!("Install Chrome / Chromium and ensure it's runnable in headless mode to run this example.");
                return;
            }
        }

        println!("Stress Testing: Rendering multiple Mermaid diagrams...\n");

        // Prepare output directory
        let out_dir = PathBuf::from("examples/output");
        fs::create_dir_all(&out_dir).expect("create examples/output dir");

        // Load font
        let font_data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();

        let fd = fonts::FontData::new(font_data, None).expect("font data");
        let family = fonts::FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };

        // Create document
        let mut doc = Document::new(family);
        doc.set_title("Mermaid Stress Test - Multiple Diagrams");

        // Title
        doc.push(
            elements::Paragraph::new("")
                .styled_string(
                    "Mermaid Stress Test: Multiple Diagrams",
                    style::Style::new().with_font_size(16).bold(),
                ),
        );
        doc.push(elements::Paragraph::new("This example renders several simple Mermaid diagrams."));
        doc.push(elements::Paragraph::new(""));

        // A set of simple diagrams to render
        let diagrams = vec![
            ("1. Simple Flow", r#"---
title: Node
---
flowchart LR
    id
"#),
            ("2. Left-to-right", r#"---
config:
  flowchart:
    htmlLabels: false
---
flowchart LR
    markdown["`This **is** _Markdown_`"]
    newLines["`Line1
    Line 2
    Line 3`"]
    markdown --> newLines
"#),
            ("3. Sequence", r#"sequenceDiagram
    Alice->>John: Hello John, how are you?
    John-->>Alice: Great!
    Alice-)John: See you later!
"#),
            ("4. Class", r#"---
title: Animal example
---
classDiagram
    note "From Duck till Zebra"
    Animal <|-- Duck
    note for Duck "can fly\ncan swim\ncan dive\ncan help in debugging"
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
    Animal: +mate()
    class Duck{
        +String beakColor
        +swim()
        +quack()
    }
    class Fish{
        -int sizeInFeet
        -canEat()
    }
    class Zebra{
        +bool is_wild
        +run()
    }
"#),
            ("5. State", r#"---
title: Simple sample
---
stateDiagram-v2
    [*] --> Still
    Still --> [*]

    Still --> Moving
    Moving --> Still
    Moving --> Crash
    Crash --> [*]
"#),
            ("6. Pie", r#"pie title Pets adopted by volunteers
    "Dogs" : 386
    "Cats" : 85
    "Rats" : 15
"#),
            ("7. Gantt", r#"gantt
    dateFormat  YYYY-MM-DD
    title       Adding GANTT diagram functionality to mermaid
    excludes    weekends
    %% (`excludes` accepts specific dates in YYYY-MM-DD format, days of the week ("sunday") or "weekends", but not the word "weekdays".)

    section A section
    Completed task            :done,    des1, 2014-01-06,2014-01-08
    Active task               :active,  des2, 2014-01-09, 3d
    Future task               :         des3, after des2, 5d
    Future task2              :         des4, after des3, 5d

    section Critical tasks
    Completed task in the critical line :crit, done, 2014-01-06,24h
    Implement parser and jison          :crit, done, after des1, 2d
    Create tests for parser             :crit, active, 3d
    Future task in critical line        :crit, 5d
    Create tests for renderer           :2d
    Add to mermaid                      :until isadded
    Functionality added                 :milestone, isadded, 2014-01-25, 0d

    section Documentation
    Describe gantt syntax               :active, a1, after des1, 3d
    Add gantt diagram to demo page      :after a1  , 20h
    Add another diagram to demo page    :doc1, after a1  , 48h

    section Last section
    Describe gantt syntax               :after doc1, 3d
    Add gantt diagram to demo page      :20h
    Add another diagram to demo page    :48h
"#),
            ("8. Mindmap", r#"mindmap
  root((System Features))
    Signal Processing
      Real-time FFT
      Butterworth filters
      Differential subtraction
      Peak detection
    Hardware Control
      USB-HID interface
      I2C thermal sensors
      SPI DAC/DDS control
      Modbus TCP server
    Web Interface
      Real-time streaming
      OAuth2/JWT security
      Multi-language UI
      Interactive graphs
    Extensibility
      Python integration
      Plugin drivers
      Hot-reload config
      REST API"#),
            ("9. Git graph", r#"---
title: Example Git diagram
---
gitGraph
   commit
   commit
   branch develop
   checkout develop
   commit
   commit
   checkout main
   merge develop
   commit
   commit
"#),
            ("10. Timeline", r#"timeline
    title History of Social Media Platform
    2002 : LinkedIn
    2004 : Facebook
         : Google
    2005 : YouTube
    2006 : Twitter
"#),
        ];

        let mut success_count = 0usize;

        for (idx, (title, diagram)) in diagrams.iter().enumerate() {
            doc.push(elements::Paragraph::new(""));
            doc.push(
                elements::Paragraph::new("").styled_string(
                    *title,
                    style::Style::new().with_font_size(11).bold(),
                ),
            );
            doc.push(elements::Paragraph::new(format!("Mermaid source: {}", diagram)));

            // Queue the Mermaid element for rendering. We intentionally do not pre-validate
            // here because opening a new tab and navigating/reloading the helper page for each
            // validation is expensive; rendering will occur later when the document is rendered.
            // This avoids double compilation and significantly reduces runtime for many diagrams.
            let mer = elements::Mermaid::new(*diagram).with_alignment(Alignment::Center).with_scale(2.0);
            doc.push(mer);
            println!("✓ Diagram {} queued for rendering", idx + 1);
            success_count += 1;
        }

        // Summary
        doc.push(elements::Paragraph::new(""));
        doc.push(elements::Paragraph::new("").styled_string(
            "=== TEST SUMMARY ===",
            style::Style::new().with_font_size(12).bold(),
        ));
        doc.push(elements::Paragraph::new(format!("Total diagrams: {}", success_count)));

        // Output document
        let output_path = out_dir.join("mermaid_stress_test.pdf");
        doc.render_to_file(&output_path)
            .expect("Failed to render PDF with Mermaid diagrams");

        println!();
        println!("{}", "=".repeat(70));
        println!("✓ Stress test PDF generated: {}", output_path.display());
        println!("{}", "=".repeat(70));
        println!();
        println!("Results:  Successful: {}", success_count);
    }
}