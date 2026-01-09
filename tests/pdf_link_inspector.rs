use std::path::Path;

use lopdf::{Document as LoDocument, Object};
use tempfile::NamedTempFile;

use genpdfi_extended::style::Style;
use genpdfi_extended::Document as GDocument;
use genpdfi_extended::{elements, fonts};

/// Represents an extracted link annotation with its URI and rectangle bounds
#[derive(Debug, Clone)]
struct LinkAnnotation {
    uri: String,
    rect: [f64; 4], // [left, bottom, right, top]
}

fn extract_link_annotations(path: &Path) -> Vec<LinkAnnotation> {
    let doc = LoDocument::load(path).expect("load pdf");
    let pages = doc.get_pages();
    let mut annotations = Vec::new();

    // Extract from page objects - annotations might be in Resources or directly on Page
    for (_pnum, &page_id) in pages.iter() {
        if let Ok(page_obj) = doc.get_object(page_id) {
            if let lopdf::Object::Dictionary(ref page_dict) = page_obj {
                // Check if Annots is directly on page
                if let Ok(annots_obj) = page_dict.get(b"Annots") {
                    if let lopdf::Object::Array(ref annots) = annots_obj {
                        extract_annotations_from_array(&doc, annots, &mut annotations);
                    }
                }

                // Check in Resources dictionary
                if let Ok(res_obj) = page_dict.get(b"Resources") {
                    let res_dict = match res_obj {
                        lopdf::Object::Dictionary(d) => d,
                        lopdf::Object::Reference(rid) => {
                            if let Ok(lopdf::Object::Dictionary(d)) = doc.get_object(*rid) {
                                d
                            } else {
                                continue;
                            }
                        }
                        _ => continue,
                    };

                    // Check if Annots is in Resources
                    if let Ok(annots_obj) = res_dict.get(b"Annots") {
                        if let lopdf::Object::Array(ref annots) = annots_obj {
                            extract_annotations_from_array(&doc, annots, &mut annotations);
                        }
                    }
                }
            }
        }
    }

    // Exhaustive search for any Link annotations
    for (_obj_id, obj) in doc.objects.iter() {
        if let lopdf::Object::Dictionary(ref dict) = obj {
            if let Ok(lopdf::Object::Name(name)) = dict.get(b"Subtype") {
                if name == b"Link" {
                    if let Ok(uri_str) = extract_uri_from_annotation(dict, &doc) {
                        if let Ok(rect) = extract_rect_from_annotation(dict) {
                            // Avoid duplicates
                            if !annotations
                                .iter()
                                .any(|a| a.uri == uri_str && (a.rect[0] - rect[0]).abs() < 0.1)
                            {
                                annotations.push(LinkAnnotation { uri: uri_str, rect });
                            }
                        }
                    }
                }
            }
        }
    }

    annotations
}

fn extract_annotations_from_array(
    doc: &LoDocument,
    annots: &[lopdf::Object],
    result: &mut Vec<LinkAnnotation>,
) {
    for annot in annots.iter() {
        // Annotations can be either direct dictionaries or references
        let ann_dict = match annot {
            lopdf::Object::Dictionary(d) => d,
            lopdf::Object::Reference(rid) => {
                if let Ok(lopdf::Object::Dictionary(d)) = doc.get_object(*rid) {
                    d
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        if let Ok(lopdf::Object::Name(name)) = ann_dict.get(b"Subtype") {
            if name == b"Link" {
                if let Ok(uri_str) = extract_uri_from_annotation(&ann_dict, &doc) {
                    if let Ok(rect) = extract_rect_from_annotation(&ann_dict) {
                        result.push(LinkAnnotation { uri: uri_str, rect });
                    }
                }
            }
        }
    }
}

fn extract_uri_from_annotation(
    ann_dict: &lopdf::Dictionary,
    doc: &LoDocument,
) -> Result<String, String> {
    if let Ok(a_obj) = ann_dict.get(b"A") {
        let a_dict_opt = match a_obj {
            Object::Dictionary(d) => Some(d),
            Object::Reference(rref) => match doc.get_object(*rref) {
                Ok(Object::Dictionary(d2)) => Some(d2),
                _ => None,
            },
            _ => None,
        };
        if let Some(a_dict) = a_dict_opt {
            if let Ok(uri_obj) = a_dict.get(b"URI") {
                match uri_obj {
                    Object::String(ref bytes, _) => {
                        return Ok(String::from_utf8_lossy(bytes).into_owned());
                    }
                    Object::Name(ref name) => {
                        return Ok(String::from_utf8_lossy(name).into_owned());
                    }
                    _ => {}
                }
            }
        }
    }
    Err("No URI found".to_string())
}

fn extract_rect_from_annotation(ann_dict: &lopdf::Dictionary) -> Result<[f64; 4], String> {
    if let Ok(Object::Array(ref rect_array)) = ann_dict.get(b"Rect") {
        if rect_array.len() >= 4 {
            let mut coords = [0.0; 4];
            for i in 0..4 {
                match &rect_array[i] {
                    Object::Integer(val) => coords[i] = *val as f64,
                    Object::Real(val) => coords[i] = *val as f64,
                    _ => return Err("Invalid coordinate type".to_string()),
                }
            }
            return Ok(coords);
        }
    }
    Err("No Rect found".to_string())
}

fn extract_link_uris(path: &Path) -> Vec<String> {
    let doc = LoDocument::load(path).expect("load pdf");
    let pages = doc.get_pages();
    let mut uris = Vec::new();

    // First, try page-local annotations
    for (_pnum, &page_id) in pages.iter() {
        let page_obj = doc.get_object(page_id).expect("page object");
        if let Object::Dictionary(ref dict) = page_obj {
            if let Ok(Object::Array(ref annots)) = dict.get(b"Annots") {
                for annot in annots.iter() {
                    if let Object::Reference(rid) = annot {
                        if let Ok(obj) = doc.get_object(*rid) {
                            if let Object::Dictionary(ref ann_dict) = obj {
                                if let Ok(Object::Name(name)) = ann_dict.get(b"Subtype") {
                                    if name == b"Link" {
                                        if let Ok(a_obj) = ann_dict.get(b"A") {
                                            let a_dict_opt = match a_obj {
                                                Object::Dictionary(d) => Some(d),
                                                Object::Reference(rref) => {
                                                    match doc.get_object(*rref).unwrap() {
                                                        Object::Dictionary(d2) => Some(d2),
                                                        _ => None,
                                                    }
                                                }
                                                _ => None,
                                            };
                                            if let Some(a_dict) = a_dict_opt {
                                                if let Ok(uri_obj) = a_dict.get(b"URI") {
                                                    match uri_obj {
                                                        Object::String(ref bytes, _) => {
                                                            uris.push(
                                                                String::from_utf8_lossy(bytes)
                                                                    .into_owned(),
                                                            );
                                                        }
                                                        Object::Name(ref name) => {
                                                            uris.push(
                                                                String::from_utf8_lossy(name)
                                                                    .into_owned(),
                                                            );
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Exhaustive search: some annotations may not be referenced from pages directly (or may be indirect)
    for (_obj_id, obj) in doc.objects.iter() {
        if let Object::Dictionary(ref dict) = obj {
            if let Ok(Object::Name(name)) = dict.get(b"Subtype") {
                if name == b"Link" {
                    if let Ok(a_obj) = dict.get(b"A") {
                        let a_dict_opt = match a_obj {
                            Object::Dictionary(d) => Some(d),
                            Object::Reference(rref) => match doc.get_object(*rref).unwrap() {
                                Object::Dictionary(d2) => Some(d2),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some(a_dict) = a_dict_opt {
                            if let Ok(uri_obj) = a_dict.get(b"URI") {
                                match uri_obj {
                                    Object::String(ref bytes, _) => {
                                        uris.push(String::from_utf8_lossy(bytes).into_owned());
                                    }
                                    Object::Name(ref name) => {
                                        uris.push(String::from_utf8_lossy(name).into_owned());
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: raw scan for http(s) substrings (useful if annotations are encoded in a way
    // that the above dictionary traversal didn't find). This is primarily for diagnostics.
    if uris.is_empty() {
        if let Ok(raw) = std::fs::read(path) {
            let needle = b"http";
            let mut pos = 0usize;
            while let Some(i) = raw[pos..].windows(4).position(|w| w == needle) {
                let start = pos + i;
                // find an end delimiter
                let mut end = start;
                while end < raw.len() {
                    let b = raw[end];
                    if b == b'"'
                        || b == b'\''
                        || b == b')'
                        || b == b'>'
                        || b == b' '
                        || b == b'\n'
                        || b == b'\r'
                    {
                        break;
                    }
                    end += 1;
                }
                if end > start {
                    if let Ok(s) = String::from_utf8(raw[start..end].to_vec()) {
                        uris.push(s);
                    }
                }
                pos = end + 1;
            }
        }
    }

    uris
}

#[test]
fn test_text_link_annotation_present() {
    // Build a simple document with a text link
    let data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd = fonts::FontData::new(data, None).expect("font data");
    let family = fonts::FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };

    let mut doc = GDocument::new(family);
    doc.set_title("link test");

    let mut p = elements::Paragraph::new("");
    p.push("Visit: ");
    p.push_link("Rust", "https://www.rust-lang.org", Style::new());
    doc.push(p);

    let mut tmp = NamedTempFile::new().expect("tmpfile");
    doc.render(tmp.as_file_mut()).expect("render");

    let uris = extract_link_uris(tmp.path());
    assert!(
        uris.iter().any(|u| u.contains("rust-lang")),
        "No rust link found in PDF annotations: {:?}",
        uris
    );
}

#[cfg(feature = "images")]
#[test]
fn test_image_link_annotation_present() {
    // Build a document with an image that has a link
    let data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd = fonts::FontData::new(data.clone(), None).expect("font data");
    let family = fonts::FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };

    let mut doc = GDocument::new(family);
    doc.set_title("image link test");

    // Use an example raster image from the repo
    let img = elements::Image::from_path("examples/images/test_image.jpg")
        .expect("load image")
        .with_link("https://example.com");
    doc.push(img);

    let mut tmp = NamedTempFile::new().expect("tmpfile");
    doc.render(tmp.as_file_mut()).expect("render");

    // Extract all link annotations
    let annotations = extract_link_annotations(tmp.path());

    // Verify URI is present
    let uri_found = annotations.iter().any(|a| a.uri.contains("example.com"));
    assert!(
        uri_found,
        "No example.com link found in PDF annotations: {:?}",
        annotations
    );

    // Verify annotation rectangle is reasonable (not empty, within page bounds)
    let rect_found = annotations.iter().any(|a| {
        a.uri.contains("example.com") && 
        a.rect[2] > a.rect[0] && // right > left
        a.rect[3] > a.rect[1] // top > bottom
    });
    assert!(
        rect_found,
        "No valid clickable area found for example.com link. Annotations: {:?}",
        annotations
    );
}

#[cfg(feature = "images")]
#[test]
fn test_multiple_image_links() {
    // Build a document with multiple images with different links
    let data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();
    let fd = fonts::FontData::new(data.clone(), None).expect("font data");
    let family = fonts::FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };

    let mut doc = GDocument::new(family);
    doc.set_title("multiple image links test");

    // Add first image with link
    let img1 = elements::Image::from_path("examples/images/test_image.jpg")
        .expect("load image")
        .with_link("https://example.com");
    doc.push(img1);

    // Add second image with different link
    let img2 = elements::Image::from_path("examples/images/test_image.jpg")
        .expect("load image")
        .with_link("https://github.com");
    doc.push(img2);

    let mut tmp = NamedTempFile::new().expect("tmpfile");
    doc.render(tmp.as_file_mut()).expect("render");

    // Extract all link annotations
    let annotations = extract_link_annotations(tmp.path());

    // Verify both URIs are present
    assert_eq!(
        annotations.len(),
        2,
        "Expected 2 annotations, found: {:?}",
        annotations
    );

    let has_example = annotations.iter().any(|a| a.uri.contains("example.com"));
    let has_github = annotations.iter().any(|a| a.uri.contains("github.com"));

    assert!(has_example, "Missing example.com link");
    assert!(has_github, "Missing github.com link");

    // Verify both have valid rectangles
    for annot in annotations.iter() {
        assert!(
            annot.rect[2] > annot.rect[0],
            "Invalid rectangle for {}: {:?}",
            annot.uri,
            annot.rect
        );
        assert!(
            annot.rect[3] > annot.rect[1],
            "Invalid rectangle for {}: {:?}",
            annot.uri,
            annot.rect
        );
    }
}
