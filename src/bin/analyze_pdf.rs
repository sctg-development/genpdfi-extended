use clap::Parser;
use lopdf::{Document, Object};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "analyze_pdf")]
#[command(about = "Analyze a PDF to debug structure and resources", long_about = None)]
struct Args {
    /// Path to the PDF file to analyze
    #[arg(value_name = "FILE")]
    pdf_file: PathBuf,
}

fn decompress_stream(data: &[u8]) -> Vec<u8> {
    use flate2::read::ZlibDecoder;

    // Try zlib decompression (zlib header: 0x78 followed by various bytes)
    if data.len() > 1 && data[0] == 0x78 {
        let mut decoder = ZlibDecoder::new(data);
        let mut result = Vec::new();
        if decoder.read_to_end(&mut result).is_ok() && !result.is_empty() {
            return result;
        }
    }

    // Return original data if decompression fails
    data.to_vec()
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
    println!("PDF ANALYSIS: {}", args.pdf_file.display());
    println!("════════════════════════════════════════════════════════\n");

    let document = Document::load(args.pdf_file.clone())?;

    // PDF version
    analyze_pdf_version(&document)?;

    // General information
    analyze_document_info(&document)?;

    // PDF objects
    analyze_objects(&document)?;

    // Embedded fonts
    analyze_fonts(&document)?;

    // Pages and content
    analyze_pages(&document)?;

    // MathML and specialized content
    analyze_mathml_streams(&document)?;

    println!("\n════════════════════════════════════════════════════════");
    println!("Analysis completed");
    println!("════════════════════════════════════════════════════════\n");

    Ok(())
}

// Helper function to retrieve a string value from an object
fn get_string_value(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
        Object::Name(name) => Some(String::from_utf8_lossy(name).to_string()),
        _ => None,
    }
}

fn analyze_pdf_version(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 PDF VERSION");
    println!("─────────────────────────────────────────────────────");

    let pages = document.get_pages();
    if !pages.is_empty() {
        println!("  ✓ Document contains {} page(s)", pages.len());
    }

    // Get the actual PDF version from the document
    let version = document.version.as_str();
    println!("  PDF Version: {}", version);

    // Parse version for better display
    if let Some((major, minor)) = parse_version(version) {
        match (major, minor) {
            (2, _) => {
                println!("  ✓ PDF 2.0 - ISO 32000-2:2020 compliant");
                println!("    Supports: MathML, structured content, AI content");
            }
            (1, 7) => {
                println!("  ✓ PDF 1.7 - ISO 32000-1:2008 compliant");
                println!("    Advanced features enabled");
            }
            (1, 6) => {
                println!("  ✓ PDF 1.6 - ISO 32000-1:2008 (subset)");
                println!("    Standard features");
            }
            _ => println!("  PDF {}.{}", major, minor),
        }
    }

    // Check for advanced features that indicate PDF 2.0 compatibility
    let mut advanced_features = Vec::new();
    for (_id, object) in document.objects.iter() {
        if let Object::Stream(stream) = object {
            let dict = &stream.dict;
            if let Ok(subtype_obj) = dict.get(b"Subtype") {
                if let Some(subtype_str) = get_string_value(subtype_obj) {
                    if subtype_str.contains("mathml")
                        || subtype_str.contains("MathML")
                        || subtype_str.contains("application#2fmathml")
                    {
                        if !advanced_features.contains(&"MathML streams") {
                            advanced_features.push("MathML streams");
                        }
                    }
                }
            }
        }
    }

    if !advanced_features.is_empty() {
        println!(
            "  📊 Advanced features detected: {}",
            advanced_features.join(", ")
        );
    }

    println!();
    Ok(())
}

// Helper function to parse PDF version string
fn parse_version(version: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        if let (Ok(major), Ok(minor)) = (parts[0].parse(), parts[1].parse()) {
            return Some((major, minor));
        }
    }
    None
}

fn analyze_document_info(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 DOCUMENT INFORMATION");
    println!("─────────────────────────────────────────────────────");

    // Try to find the Info dictionary
    let mut found_info = false;
    for (id, object) in document.objects.iter() {
        if let Object::Dictionary(dict) = object {
            // Check if it's an Info object by looking at the keys
            let has_info_keys = dict
                .iter()
                .any(|(k, _)| k == b"Title" || k == b"Author" || k == b"Subject");

            if has_info_keys {
                found_info = true;
                println!("  Info object found (ID: {:?}):", id);

                if let Ok(title) = dict.get(b"Title") {
                    if let Some(title_str) = get_string_value(title) {
                        println!("    Title: {}", title_str);
                    }
                }
                if let Ok(author) = dict.get(b"Author") {
                    if let Some(author_str) = get_string_value(author) {
                        println!("    Author: {}", author_str);
                    }
                }
                if let Ok(subject) = dict.get(b"Subject") {
                    if let Some(subject_str) = get_string_value(subject) {
                        println!("    Subject: {}", subject_str);
                    }
                }
                if let Ok(producer) = dict.get(b"Producer") {
                    if let Some(producer_str) = get_string_value(producer) {
                        println!("    Producer: {}", producer_str);
                    }
                }
            }
        }
    }

    if !found_info {
        println!("  No document information found");
    }

    println!();
    Ok(())
}

fn analyze_objects(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 PDF OBJECTS");
    println!("─────────────────────────────────────────────────────");

    let mut object_types: BTreeMap<String, usize> = BTreeMap::new();
    let mut object_details = Vec::new();

    for (id, object) in document.objects.iter() {
        let type_name = match object {
            Object::Null => "Null",
            Object::Integer(_) => "Integer",
            Object::Real(_) => "Real",
            Object::Boolean(_) => "Boolean",
            Object::Name(_) => "Name",
            Object::String(_, _) => "String",
            Object::Array(_) => "Array",
            Object::Dictionary(_) => "Dictionary",
            Object::Stream(_) => "Stream",
            Object::Reference(_) => "Reference",
        };

        *object_types.entry(type_name.to_string()).or_insert(0) += 1;

        // Details of important objects
        match object {
            Object::Stream(stream) => {
                let dict = &stream.dict;
                if let Ok(subtype_obj) = dict.get(b"Subtype") {
                    if let Some(subtype_str) = get_string_value(subtype_obj) {
                        object_details
                            .push(format!("  [{:?}] Stream: Subtype={}", id, subtype_str));

                        // Look for MathML content
                        if subtype_str.contains("Math") || subtype_str.contains("XML") {
                            println!(
                                "  ⚠️  [{:?}] Stream potentially MathML: Subtype={}",
                                id, subtype_str
                            );
                        }
                    }
                }
            }
            Object::Dictionary(dict) => {
                if let Ok(type_obj) = dict.get(b"Type") {
                    if let Some(type_str) = get_string_value(type_obj) {
                        if type_str == "Font" || type_str == "XObject" || type_str == "Pattern" {
                            object_details.push(format!("  [{:?}] Type={}", id, type_str));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    println!("  Total objects: {}", document.objects.len());
    println!("\n  Object types:");
    for (type_name, count) in object_types.iter() {
        println!("    {}: {} object(s)", type_name, count);
    }

    if !object_details.is_empty() {
        println!("\n  Details of important objects:");
        for detail in object_details.iter().take(20) {
            println!("{}", detail);
        }
        if object_details.len() > 20 {
            println!("    ... and {} other objects", object_details.len() - 20);
        }
    }

    println!();
    Ok(())
}

fn analyze_fonts(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔤 EMBEDDED FONTS");
    println!("─────────────────────────────────────────────────────");

    let mut font_count = 0;
    let mut embedded_fonts = Vec::new();

    for (id, object) in document.objects.iter() {
        if let Object::Dictionary(dict) = object {
            if let Ok(type_obj) = dict.get(b"Type") {
                if let Some(type_str) = get_string_value(type_obj) {
                    if type_str == "Font" {
                        font_count += 1;
                        println!("  Font found (ID: {:?})", id);

                        if let Ok(subtype) = dict.get(b"Subtype") {
                            if let Some(subtype_str) = get_string_value(subtype) {
                                println!("    Subtype: {}", subtype_str);
                            }
                        }

                        if let Ok(base_font) = dict.get(b"BaseFont") {
                            if let Some(base_font_str) = get_string_value(base_font) {
                                println!("    BaseFont: {}", base_font_str);
                            }
                        }

                        // Search for embedded fonts
                        let has_font_file = dict.iter().any(|(k, _)| {
                            k == b"FontFile" || k == b"FontFile2" || k == b"FontFile3"
                        });

                        if has_font_file {
                            println!("    ✓ EMBEDDED font");
                            if let Ok(base_font) = dict.get(b"BaseFont") {
                                if let Some(font_name) = get_string_value(base_font) {
                                    embedded_fonts.push(font_name);
                                }
                            }
                        } else if let Ok(descriptor_obj) = dict.get(b"FontDescriptor") {
                            if let Object::Dictionary(descriptor) = descriptor_obj {
                                let has_desc_font = descriptor.iter().any(|(k, _)| {
                                    k == b"FontFile" || k == b"FontFile2" || k == b"FontFile3"
                                });

                                if has_desc_font {
                                    println!("    ✓ EMBEDDED font (in FontDescriptor)");
                                    if let Ok(base_font) = dict.get(b"BaseFont") {
                                        if let Some(font_name) = get_string_value(base_font) {
                                            embedded_fonts.push(font_name);
                                        }
                                    }
                                }
                            }
                        }

                        println!();
                    }
                }
            }
        }
    }

    println!("  Total fonts: {}", font_count);
    if !embedded_fonts.is_empty() {
        println!("  Embedded fonts: {}", embedded_fonts.join(", "));
    } else {
        println!("  No embedded fonts found");
    }

    println!();
    Ok(())
}

fn analyze_pages(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    println!("📰 PAGES AND CONTENT");
    println!("─────────────────────────────────────────────────────");

    let pages = document.get_pages();
    let pages_count = pages.len();
    println!("  Number of pages: {}", pages_count);

    let mut page_num = 0;
    for (page_id, _) in pages.iter() {
        page_num += 1;

        // page_id is a u32 (key from BTreeMap), but get_object needs (u32, u16)
        // We need to construct a proper ObjectId
        let obj_id = (*page_id, 0u16);
        if let Ok(page_obj) = document.get_object(obj_id) {
            if let Object::Dictionary(page_dict) = page_obj {
                println!("\n  Page {}:", page_num);

                // Handle the contents
                if let Ok(contents_obj) = page_dict.get(b"Contents") {
                    let content_ids = match contents_obj {
                        Object::Array(arr) => arr.clone(),
                        other => vec![other.clone()],
                    };

                    for content_ref in content_ids {
                        if let Object::Reference(content_id) = content_ref {
                            if let Ok(content_obj) = document.get_object(content_id) {
                                if let Object::Stream(stream) = content_obj {
                                    let content_len = stream.content.len();
                                    println!(
                                        "    Content stream (ID: {:?}): {} bytes",
                                        content_id, content_len
                                    );

                                    // Analyze content stream for rendering operations
                                    if let Ok(content_str) = std::str::from_utf8(&stream.content) {
                                        // Check for text showing operations
                                        let has_text_show = content_str.contains("Tj") || content_str.contains("TJ");
                                        if has_text_show {
                                            println!("      ✓ Contains text showing operations (Tj/TJ)");
                                        }
                                        
                                        // Check for graphics state operations
                                        let has_gs = content_str.contains("/GS");
                                        if has_gs {
                                            println!("      ✓ Uses graphics state operations");
                                        }

                                        // Look for XObject references (Do operator)
                                        let has_do = content_str.contains(" Do");
                                        if has_do {
                                            println!("      ✓ References XObjects (formulas, images)");
                                        }

                                        // Look for MathML elements in the content
                                        if content_str.contains("Math")
                                            || content_str.contains("<math")
                                        {
                                            println!("      ⚠️  Potentially contains MathML!");
                                        }
                                        let has_bdc = content_str.contains("BDC");
                                        let has_emc = content_str.contains("EMC");
                                        if has_bdc || has_emc {
                                            println!("      ✓ Uses BDC/EMC tags (tagged PDF)");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Page resources - detailed analysis
                if let Ok(resources_obj) = page_dict.get(b"Resources") {
                    if let Object::Dictionary(resources_dict) = resources_obj {
                        if !resources_dict.is_empty() {
                            println!("    Resources:");
                            
                            // Analyze XObjects
                            if let Ok(xobjects_obj) = resources_dict.get(b"XObject") {
                                if let Object::Dictionary(xobjects_dict) = xobjects_obj {
                                    println!("      - XObjects ({} total):", xobjects_dict.len());
                                    for (xobj_key, xobj_ref) in xobjects_dict.iter().take(10) {
                                        if let Object::Reference(xobj_id) = xobj_ref {
                                            if let Ok(xobj) = document.get_object(*xobj_id) {
                                                if let Object::Stream(xobj_stream) = xobj {
                                                    if let Ok(subtype) = xobj_stream.dict.get(b"Subtype") {
                                                        if let Some(subtype_str) = get_string_value(subtype) {
                                                            println!("        - {:?}: {} ({} bytes)",
                                                                String::from_utf8_lossy(xobj_key),
                                                                subtype_str,
                                                                xobj_stream.content.len()
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Check for other resources
                            for (key, _) in resources_dict.iter() {
                                match key.as_slice() {
                                    b"Font" => println!("      - Fonts present"),
                                    b"Pattern" => println!("      - Patterns present"),
                                    b"ColorSpace" => println!("      - ColorSpaces present"),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!();
    Ok(())
}

// Determine if a stream is likely MathML based on Subtype and content.
// Improved heuristic: if Subtype explicitly mentions Math or MathML, accept.
// If Subtype mentions XML, require presence of MathML-specific markers (namespace or tags).
// Otherwise, scan content for MathML tags or namespace declarations.
fn is_potential_mathml(subtype: Option<&str>, content: &str) -> bool {
    // Normalize to lowercase for case-insensitive searches
    let content_lc = content.to_lowercase();

    if let Some(sub) = subtype {
        let sub_lc = sub.to_lowercase();
        // Explicit math indicators in subtype
        if sub_lc.contains("math") || sub_lc.contains("mathml") {
            return true;
        }

        // If subtype says XML, inspect content for MathML markers (more strict)
        if sub_lc.contains("xml") {
            if content_lc.contains("<math")
                || content_lc.contains("xmlns=\"http://www.w3.org/1998/math/mathml\"")
                || content_lc.contains("application/mathml")
                || content_lc.contains("<mi")
                || content_lc.contains("<mn")
                || content_lc.contains("<mo")
                || content_lc.contains("<msqrt")
                || content_lc.contains("<mrow")
                || content_lc.contains("<mfrac")
                || content_lc.contains("<msup")
                || content_lc.contains("<msub")
                || content_lc.contains("<msubsup")
                || content_lc.contains("<mtable")
            {
                return true;
            }
            return false;
        }
    }

    // No subtype hint: look for MathML markers in content
    if content_lc.contains("<math")
        || content_lc.contains("xmlns=\"http://www.w3.org/1998/math/mathml\"")
        || content_lc.contains("<mi")
        || content_lc.contains("<mn")
        || content_lc.contains("<mo")
        || content_lc.contains("<msqrt")
        || content_lc.contains("<mrow")
        || content_lc.contains("<mfrac")
        || content_lc.contains("<msup")
        || content_lc.contains("<msub")
        || content_lc.contains("<msubsup")
        || content_lc.contains("<mtable")
    {
        return true;
    }

    false
}

fn analyze_mathml_streams(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 MATHML AND SPECIAL CONTENT");
    println!("─────────────────────────────────────────────────────");

    let mut found = false;
    let mut mathml_count = 0;
    let mut mathml_streams = Vec::new();

    for (id, object) in document.objects.iter() {
        if let Object::Stream(stream) = object {
            let dict = &stream.dict;
            let subtype_opt = if let Ok(subtype_obj) = dict.get(b"Subtype") {
                get_string_value(subtype_obj)
            } else {
                None
            };

            // Get the raw stream content and try to decompress it
            let raw_content = &stream.content;
            let decompressed = decompress_stream(raw_content);
            let content_to_use = if !decompressed.is_empty() && decompressed != *raw_content {
                decompressed
            } else {
                raw_content.to_vec()
            };

            // Try UTF-8 first, fallback to Latin-1 if needed
            let content_str = if let Ok(s) = std::str::from_utf8(&content_to_use) {
                s.to_string()
            } else {
                // Fallback: interpret as Latin-1
                content_to_use.iter().map(|&b| b as char).collect()
            };

            // Check if this is a MathML stream (could be regular stream or embedded file)
            let is_mathml_stream = if let Some(subtype) = subtype_opt.as_deref() {
                subtype.contains("mathml")
                    || subtype.contains("MathML")
                    || subtype.contains("application#2fmathml")
            } else {
                is_potential_mathml(subtype_opt.as_deref(), &content_str)
            };

            if is_mathml_stream {
                mathml_count += 1;
                found = true;

                println!("  ⚠️  [{:?}] MathML Stream/EmbeddedFile", id);

                if let Some(subtype_str) = subtype_opt.as_deref() {
                    // Decode PDF name encoding (#2f = /)
                    let decoded_subtype = subtype_str.replace("#2f", "/");
                    println!("    Subtype: {}", decoded_subtype);
                }

                println!("    Content length: {} bytes", content_to_use.len());

                // Check if it's an EmbeddedFile type
                if let Ok(type_obj) = dict.get(b"Type") {
                    if let Some(type_name) = get_string_value(type_obj) {
                        if type_name.contains("EmbeddedFile") {
                            println!("    Type: EmbeddedFile");
                        }
                    }
                }

                // Display full content
                if !content_str.is_empty() {
                    println!("    Content:\n{}", content_str);
                    println!();
                }

                // Store for later extraction
                mathml_streams.push((id, subtype_opt.clone(), content_str.clone()));

                // Analyze MathML elements present
                let mut found_tags = Vec::new();
                if content_str.contains("<math") {
                    found_tags.push("math");
                }
                if content_str.contains("<mrow") {
                    found_tags.push("mrow");
                }
                if content_str.contains("<mi") {
                    found_tags.push("mi");
                }
                if content_str.contains("<mn") {
                    found_tags.push("mn");
                }
                if content_str.contains("<mo") {
                    found_tags.push("mo");
                }
                if content_str.contains("<mfrac") {
                    found_tags.push("mfrac");
                }
                if content_str.contains("<msqrt") {
                    found_tags.push("msqrt");
                }
                if content_str.contains("<msubsup") {
                    found_tags.push("msubsup");
                }
                if content_str.contains("<msup") {
                    found_tags.push("msup");
                }
                if content_str.contains("<msub") {
                    found_tags.push("msub");
                }
                if content_str.contains("<mtable") {
                    found_tags.push("mtable");
                }

                if !found_tags.is_empty() {
                    println!("    MathML tags found: {}", found_tags.join(", "));
                }

                // Check for MathML namespace
                if content_str.contains("http://www.w3.org/1998/math/mathml")
                    || content_str.contains("http://www.w3.org/1998/Math/mathml")
                {
                    println!("    ✓ Contains MathML namespace declaration");
                }

                println!();
            }
        }
    }

    if !found {
        println!("  No MathML-specific streams detected");
    } else {
        println!("  Total MathML streams found: {}", mathml_count);
        println!("\n  📊 SUMMARY OF EXTRACTED MATHML:");
        println!("  ─────────────────────────────────────────────────────");
        for (id, subtype_opt, content) in mathml_streams.iter() {
            println!("\n  Stream ID: {:?}", id);
            if let Some(subtype) = subtype_opt {
                println!("  Subtype: {}", subtype);
            }
            println!("  MathML:\n{}\n", content);
        }
    }

    println!();
    Ok(())
}

#[cfg(test)]

const COMPLEX_PDF: &[u8] = include_bytes!("../../tests/mathml-AF-complex test.pdf");

mod tests {
    use super::*;
    use lopdf::Object;

    #[test]
    fn test_get_string_value_string() {
        let obj = Object::String(b"hello".to_vec(), Default::default());
        assert_eq!(get_string_value(&obj), Some("hello".to_string()));
    }

    #[test]
    fn test_get_string_value_name() {
        let obj = Object::Name(b"Name".to_vec());
        assert_eq!(get_string_value(&obj), Some("Name".to_string()));
    }

    #[test]
    fn test_get_string_value_other() {
        let obj = Object::Integer(42.into());
        assert_eq!(get_string_value(&obj), None);
    }

    #[test]
    fn test_is_potential_mathml_explicit_mathml_subtype() {
        // Explicit subtype indicating MathML
        assert!(is_potential_mathml(Some("MathML"), ""));
        assert!(is_potential_mathml(Some("application/mathml+xml"), ""));
        assert!(is_potential_mathml(Some("mathml"), "")); // case-insensitive
    }

    #[test]
    fn test_is_potential_mathml_xml_with_math_markers() {
        // Subtype XML requires MathML markers in content
        assert!(is_potential_mathml(Some("XML"), "<math></math>"));
        assert!(is_potential_mathml(Some("XML"), "<mi>x</mi>"));
        assert!(is_potential_mathml(Some("XML"), "<mrow><mi>x</mi></mrow>"));
        assert!(is_potential_mathml(
            Some("XML"),
            "<mfrac><mn>1</mn><mn>2</mn></mfrac>"
        ));
    }

    #[test]
    fn test_is_potential_mathml_xml_without_math_markers() {
        // Subtype XML without MathML markers should not match
        assert!(!is_potential_mathml(Some("XML"), "<root></root>"));
        assert!(!is_potential_mathml(Some("XML"), "<svg></svg>"));
    }

    #[test]
    fn test_is_potential_mathml_no_subtype_with_content() {
        // No subtype: content-based detection
        assert!(is_potential_mathml(None, "<math></math>"));
        assert!(is_potential_mathml(
            None,
            "xmlns=\"http://www.w3.org/1998/math/mathml\""
        ));
        assert!(is_potential_mathml(None, "<mi>x</mi>"));
        assert!(is_potential_mathml(None, "<msqrt><mi>x</mi></msqrt>"));
    }

    #[test]
    fn test_is_potential_mathml_no_match() {
        // Non-math cases
        assert!(!is_potential_mathml(Some("Image"), "some content"));
        assert!(!is_potential_mathml(None, "random XML without math"));
        assert!(!is_potential_mathml(None, "<div></div>"));
    }

    #[test]
    fn test_is_potential_mathml_case_insensitive() {
        // Test case-insensitive matching
        assert!(is_potential_mathml(Some("APPLICATION/MATHML+XML"), ""));
        assert!(is_potential_mathml(None, "<MATH></MATH>"));
        assert!(is_potential_mathml(None, "<Mi>x</Mi>"));
    }

    #[test]
    fn test_is_potential_mathml_complex_structures() {
        // Test with more complex MathML structures
        let complex_mathml = r#"
            <math xmlns="http://www.w3.org/1998/Math/MathML">
                <mfrac>
                    <mrow><mi>a</mi><msup><mi>x</mi><mn>2</mn></msup></mrow>
                    <mrow><mi>b</mi><mo>=</mo><mn>0</mn></mrow>
                </mfrac>
            </math>
        "#;
        assert!(is_potential_mathml(None, complex_mathml));
        assert!(is_potential_mathml(Some("XML"), complex_mathml));
    }

    #[test]
    fn test_get_string_value_with_unicode() {
        let obj = Object::String("π ≈ 3.14".as_bytes().to_vec(), Default::default());
        assert_eq!(get_string_value(&obj), Some("π ≈ 3.14".to_string()));
    }

    #[test]
    fn test_is_potential_mathml_various_math_tags() {
        // Test detection of various MathML element tags
        assert!(is_potential_mathml(None, "<msubsup>"));
        assert!(is_potential_mathml(None, "<mrow>"));
        assert!(is_potential_mathml(None, "<mtable>"));
        assert!(is_potential_mathml(
            Some("XML"),
            "<msup><mi>x</mi><mn>2</mn></msup>"
        ));
        assert!(is_potential_mathml(
            Some("XML"),
            "<msub><mi>x</mi><mn>0</mn></msub>"
        ));
    }

    #[test]
    fn test_decompress_stream_noop() {
        let data = b"notcompressed".to_vec();
        let out = decompress_stream(&data);
        assert_eq!(out, data);
    }

    #[test]
    fn test_decompress_stream_zlib() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"hello world").unwrap();
        let compressed = encoder.finish().unwrap();

        // compressed zlib stream should be decompressed
        let out = decompress_stream(&compressed);
        assert!(out.len() > 0);
        assert!(std::str::from_utf8(&out).unwrap().contains("hello world"));
    }
}
