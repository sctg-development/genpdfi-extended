//! Stress Test: Mermaid Diagrams with Automatic Scaling
//!
//! This example renders the same set of Mermaid diagrams as
//! `mermaid_stress_test.rs` but uses the `Mermaid::with_auto_scale()` API to
//! automatically cap the rendered diagram at 90% of the page width/height.
//!
//! Run with: cargo run --example mermaid_auto_scale --features "images,mermaid"

use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{elements, fonts, style, Alignment, Document};

fn main() {
    if !cfg!(feature = "mermaid") {
        eprintln!("This example requires the 'mermaid' feature to be enabled.");
        eprintln!("Run with: cargo run --example mermaid_auto_scale --features 'images,mermaid'");
        return;
    }

    #[cfg(feature = "mermaid")]
    {
        // Warm up / check headless Chrome availability
        match elements::Mermaid::ensure_browser() {
            Ok(_) => println!("Headless Chrome appears usable; proceeding with Mermaid auto-scale test..."),
            Err(e) => {
                eprintln!("Headless Chrome is not available or failed to start: {}", e);
                eprintln!("Install Chrome / Chromium and ensure it's runnable in headless mode to run this example.");
                return;
            }
        }

        println!("Rendering diagrams with automatic scaling (max 90% of page dims)...\n");

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

        let mut doc = Document::new(family);
        doc.set_title("Mermaid Auto Scale Test - Multiple Diagrams");

        doc.push(
            elements::Paragraph::new("")
                .styled_string(
                    "Mermaid Auto Scale Test: Multiple Diagrams",
                    style::Style::new().with_font_size(16).bold(),
                ),
        );
        doc.push(elements::Paragraph::new("This example renders several Mermaid diagrams using `with_auto_scale()`."));
        doc.push(elements::Paragraph::new(""));

        // The same diagrams used in mermaid_stress_test.rs
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
"#),("11. AtMega32U4",r#"graph TB
    subgraph MCU["ATMega32u4 - Pins"]
        I2C_SDA["D2 - SDA"]
        I2C_SCL["D3 - SCL"]
        SPI_CS_TEC["D10 - CS_TEC"]
        SPI_CS_LASER["D9 - CS_LASER"]
        SPI_MOSI["D16 - MOSI"]
        SPI_SCK["D15 - SCK"]
        GPIO_TEC["D4 - ON_OFF_TEC"]
        GPIO_LASER["D5 - ON_OFF_LASER"]
        GPIO_FAULT["D6 - FAULT_READ"]
    end
  
    subgraph ADC["ADS1115 - Monitoring"]
        ADC_A0["A0 - I_TEC"]
        ADC_A1["A1 - I_LASER"]
        ADC_A2["A2 - TEMP"]
        ADC_A3["A3 - V_TEC"]
    end
  
    subgraph DAC["DACs de contrôle"]
        DAC_TEC["LTC2641<br/>TEC Control"]
        DAC_LASER["LTC2641<br/>Laser Control"]
    end
  
    subgraph DL150["Module DL150"]
        TEC["TEC Driver"]
        LASER["Laser Driver"]
        SENS["Capteurs"]
    end
  
    I2C_SDA -->|"I2C Data"| ADC
    I2C_SCL -->|"I2C Clock"| ADC
  
    SPI_CS_TEC -->|"Chip Select"| DAC_TEC
    SPI_CS_LASER -->|"Chip Select"| DAC_LASER
    SPI_MOSI -->|"Data"| DAC_TEC
    SPI_MOSI -->|"Data"| DAC_LASER
    SPI_SCK -->|"Clock"| DAC_TEC
    SPI_SCK -->|"Clock"| DAC_LASER
  
    GPIO_TEC -->|"Enable"| TEC
    GPIO_LASER -->|"Enable"| LASER
    GPIO_FAULT <-->|"Status"| DL150
  
    DAC_TEC -->|"Analog Out"| TEC
    DAC_LASER -->|"Analog Out"| LASER
  
    SENS -->|"I_TEC"| ADC_A0
    SENS -->|"I_LASER"| ADC_A1
    SENS -->|"Temp"| ADC_A2
    SENS -->|"V_TEC"| ADC_A3
"#)
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
            // Use the document's default font family (a `fonts::FontFamily<fonts::Font>`) so
            // the `Style::with_font_family` signature matches `FontFamily<Font>`.

            doc.push(elements::Paragraph::new("").styled_string(
                format!("Mermaid source: {}", diagram),
                style::Style::new().with_font_size(8),
            ));

            // Use original scaling
            let mer = elements::Mermaid::new(*diagram).with_alignment(Alignment::Center);
            doc.push(mer);
            // Use automatic scaling rather than a fixed scale
            let mer = elements::Mermaid::new(*diagram).with_alignment(Alignment::Center).with_auto_scale(2.0, 0.9);
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
        let output_path = out_dir.join("mermaid_auto_scale.pdf");
        doc.render_to_file(&output_path)
            .expect("Failed to render PDF with Mermaid diagrams");

        println!();
        println!("{}", "=".repeat(70));
        println!("✓ Auto-scale stress test PDF generated: {}", output_path.display());
        println!("{}", "=".repeat(70));
        println!();
        println!("Results:  Successful: {}", success_count);
    }
}
