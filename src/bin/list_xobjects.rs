// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

use clap::Parser;
use lopdf::{Document, Object};
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "list_xobjects")]
#[command(about = "List all XObjects (images, forms, SVGs) in a PDF", long_about = None)]
struct Args {
    /// Path to the PDF file to analyze
    #[arg(value_name = "FILE")]
    pdf_file: PathBuf,

    /// Show detailed information about each XObject
    #[arg(short, long)]
    detailed: bool,
}

fn decompress_stream(data: &[u8]) -> Vec<u8> {
    use flate2::read::ZlibDecoder;

    // Try zlib decompression
    if data.len() > 1 && data[0] == 0x78 {
        let mut decoder = ZlibDecoder::new(data);
        let mut result = Vec::new();
        if decoder.read_to_end(&mut result).is_ok() && !result.is_empty() {
            return result;
        }
    }

    data.to_vec()
}

fn get_string_value(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
        Object::Name(name) => Some(String::from_utf8_lossy(name).to_string()),
        _ => None,
    }
}

fn detect_content_type(content: &str) -> &'static str {
    // Check for PDF vector operators (paths)
    if content.contains(" m ") || // moveto
        content.contains(" l ") || // lineto
        content.contains(" c ") || // curveto
        content.contains(" h ") || // close path
        content.contains(" f ") || // fill
        content.contains(" S ") || // stroke
        content.contains("q\n") || // save graphics state
        content.contains("Q\n") || // restore graphics state
        content.contains("BT ") || // begin text
        content.contains(" ET") || // end text
        content.contains(" re ")
    // rectangle
    {
        "Vectorial (PDF paths/text)"
    } else if content.contains("BI ") || content.contains("EI") {
        // BI = begin image, EI = end image
        "Rasterized (embedded image)"
    } else if content.contains("<svg") || content.contains("<?xml") {
        "SVG (XML)"
    } else {
        "Unknown content type"
    }
}

fn analyze_form_content(stream_content: &[u8]) -> String {
    // Try to decompress first
    let decompressed = decompress_stream(stream_content);
    let content = String::from_utf8_lossy(&decompressed);

    // Detect content type
    let content_type = detect_content_type(&content);

    // If still unknown, try raw content
    if content_type == "Unknown content type" {
        let raw_content = String::from_utf8_lossy(stream_content);
        let raw_type = detect_content_type(&raw_content);
        if raw_type != "Unknown content type" {
            return format!("{} (from raw content)", raw_type);
        }
    }

    content_type.to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.pdf_file.exists() {
        eprintln!(
            "Error: The file '{}' does not exist",
            args.pdf_file.display()
        );
        std::process::exit(1);
    }

    println!("\n════════════════════════════════════════════════════════");
    println!("XObject Analysis: {}", args.pdf_file.display());
    println!("════════════════════════════════════════════════════════\n");

    let document = Document::load(&args.pdf_file)?;

    let mut xobject_count = 0;
    let mut image_count = 0;
    let mut form_count = 0;
    let mut svg_count = 0;

    println!("Scanning all objects for XObjects...\n");

    // Scan ALL streams in the document to find XObjects
    for (obj_id, obj) in document.objects.iter() {
        if let Object::Stream(stream) = obj {
            // Check if it's an XObject (Form or Image)
            if let Ok(subtype) = stream.dict.get(b"Subtype") {
                if let Some(subtype_str) = get_string_value(subtype) {
                    match subtype_str.as_str() {
                        "Form" => {
                            xobject_count += 1;
                            form_count += 1;

                            println!("Form XObject: {:?}", obj_id);

                            // Check if it contains SVG
                            let content = String::from_utf8_lossy(&stream.content);
                            if content.contains("<svg") || content.contains("<?xml") {
                                svg_count += 1;
                                println!("  ✓ Contains SVG content");
                                if args.detailed {
                                    println!("    Stream size: {} bytes", stream.content.len());
                                    let decompressed = decompress_stream(&stream.content);
                                    println!("    Decompressed size: {} bytes", decompressed.len());
                                    let preview = content.chars().take(200).collect::<String>();
                                    println!("    Preview: {}...", preview);
                                }
                            } else {
                                let content_type = analyze_form_content(&stream.content);
                                println!("  Type: Form XObject ({})", content_type);
                                if let Ok(Object::Integer(w)) = stream.dict.get(b"Width") {
                                    println!("    Width: {}", w);
                                }
                                if let Ok(Object::Integer(h)) = stream.dict.get(b"Height") {
                                    println!("    Height: {}", h);
                                }
                                if let Ok(Object::Array(bbox)) = stream.dict.get(b"BBox") {
                                    let bbox_str = bbox
                                        .iter()
                                        .filter_map(|obj| match obj {
                                            Object::Integer(i) => Some(i.to_string()),
                                            Object::Real(r) => Some(r.to_string()),
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    println!("    BBox: [{}]", bbox_str);
                                }
                                if args.detailed {
                                    println!("    Stream size: {} bytes", stream.content.len());
                                    let decompressed = decompress_stream(&stream.content);
                                    println!("    Decompressed size: {} bytes", decompressed.len());
                                    // Show first 500 bytes of content
                                    let preview = String::from_utf8_lossy(&decompressed);
                                    let preview_str = preview.chars().take(500).collect::<String>();
                                    println!("    Content preview: {}...", preview_str);
                                }
                            }
                            println!();
                        }
                        "Image" => {
                            xobject_count += 1;
                            image_count += 1;

                            println!("Image XObject: {:?}", obj_id);

                            if let Ok(Object::Integer(w)) = stream.dict.get(b"Width") {
                                println!("  Width: {}", w);
                            }
                            if let Ok(Object::Integer(h)) = stream.dict.get(b"Height") {
                                println!("  Height: {}", h);
                            }
                            if let Ok(cs) = stream.dict.get(b"ColorSpace") {
                                if let Some(cs_str) = get_string_value(cs) {
                                    println!("  ColorSpace: {}", cs_str);
                                }
                            }
                            println!();
                        }
                        "PS" => {
                            println!("PostScript XObject: {:?}\n", obj_id);
                        }
                        _ => {
                            // Unknown subtype, but check if it's a Form variant with SVG
                            if subtype_str.contains("Form") {
                                let content = String::from_utf8_lossy(&stream.content);
                                if content.contains("<svg") || content.contains("<?xml") {
                                    xobject_count += 1;
                                    form_count += 1;
                                    svg_count += 1;

                                    println!("Form XObject ({}): {:?}", subtype_str, obj_id);
                                    println!("  ✓ Contains SVG content");
                                    if args.detailed {
                                        println!("    Stream size: {} bytes", stream.content.len());
                                        let decompressed = decompress_stream(&stream.content);
                                        println!(
                                            "    Decompressed size: {} bytes",
                                            decompressed.len()
                                        );
                                        let preview = content.chars().take(200).collect::<String>();
                                        println!("    Preview: {}...", preview);
                                    }
                                    println!();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Summary
    println!("════════════════════════════════════════════════════════");
    println!("SUMMARY");
    println!("════════════════════════════════════════════════════════");
    println!("Total XObjects: {}", xobject_count);
    println!("  - Images: {}", image_count);
    println!("  - Forms: {}", form_count);
    println!("  - SVGs (in Forms): {}", svg_count);

    if svg_count > 0 {
        println!(
            "\n✓ PDF contains {} SVG-based content (LaTeX/Mermaid)",
            svg_count
        );
    } else if form_count > 0 {
        println!(
            "\n✓ PDF contains {} vectorial Form XObjects (LaTeX/Mermaid rendered as PDF paths)",
            form_count
        );
    } else if image_count > 0 {
        println!("\n✓ PDF contains {} rasterized Image XObjects", image_count);
    } else {
        println!("\n✗ No XObjects found in this PDF");
    }

    Ok(())
}
