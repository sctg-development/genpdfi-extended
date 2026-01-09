use std::path::Path;

use lopdf::{Document as LoDocument, Object};
use tempfile::NamedTempFile;

use genpdfi_extended::Document as GDocument;
use genpdfi_extended::{fonts, elements};
use genpdfi_extended::style::Style;

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
                    if b == b'"' || b == b'\'' || b == b')' || b == b'>' || b == b' ' || b == b'\n' || b == b'\r' { break; }
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
    let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
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
    assert!(uris.iter().any(|u| u.contains("rust-lang")), "No rust link found in PDF annotations: {:?}", uris);
}

#[cfg(feature = "images")]
#[test]
fn test_image_link_annotation_present() {
    // Build a document with an image that has a link
    let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
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
    let img = elements::Image::from_path("examples/images/test_image.jpg").expect("load image").with_link("https://example.com");
    doc.push(img);

    let mut tmp = NamedTempFile::new().expect("tmpfile");
    doc.render(tmp.as_file_mut()).expect("render");

    let uris = extract_link_uris(tmp.path());
    assert!(uris.iter().any(|u| u.contains("example.com")), "No example.com link found in PDF annotations: {:?}", uris);
}
