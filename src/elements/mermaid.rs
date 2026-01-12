//! Mermaid diagram element
//!
//! This element renders a Mermaid diagram by invoking a headless Chrome instance that executes
//! the Mermaid JS runtime embedded in the crate's `examples/helper` files and returns an SVG.
//! The resulting SVG is embedded into the PDF using the existing `Image` element.
//!
//! This element is only available when the `mermaid` feature is enabled.

#[cfg(feature = "mermaid")]
use crate::{Alignment, Position, Size};

#[cfg(feature = "mermaid")]
mod inner {
    use std::fmt::Display;

    use escape_string::escape;
    use headless_chrome::Browser;
    use unescape::unescape;

    use crate::error::{Context as _, Error, ErrorKind};
    use crate::render;
    use crate::style::Style;
    use crate::{Alignment, Context, Element, RenderResult, Size, Position};

    use super::Mermaid;

    #[derive(Debug)]
    struct CompileError;

    impl Display for CompileError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Mermaid compile error")
        }
    }

    impl std::error::Error for CompileError {}

    use once_cell::sync::OnceCell;

// Shared headless Chrome instance used across Mermaid renders.
// Lazily initialized with OnceCell to avoid spawning a new browser for every diagram,
// which would be expensive and slow. Reusing a single Browser improves performance
// and reduces resource usage.
static BROWSER: OnceCell<Browser> = OnceCell::new();
    /// Initialize or check the shared headless Chrome instance used for Mermaid rendering.
    ///
    /// This will lazily start a Browser on first call and return quickly afterwards.
    /// Errors are wrapped to provide a helpful diagnostic if Chrome cannot be launched.
    pub fn ensure_browser() -> Result<(), Error> {
        // Attempt to initialize the global Browser once. `get_or_try_init` returns a
        // reference if already initialized or runs the closure to create it.
        BROWSER.get_or_try_init(|| {
            Browser::default().map_err(|e| Error::new(format!("Failed to start headless chrome: {}", e), ErrorKind::Internal))
        })?;
        Ok(())
    }

    fn get_browser() -> Result<&'static Browser, Error> {
        // Return a reference to the shared Browser, initializing it if necessary.
        // Using a &'static Browser makes it convenient to use across async calls and closures.
        BROWSER.get_or_try_init(|| {
            Browser::default().map_err(|e| Error::new(format!("Failed to start headless chrome: {}", e), ErrorKind::Internal))
        })
    }

    impl Mermaid {
        /// Renders the diagram string to an SVG string using an embedded helper page and
        /// the `mermaid.min.js` script from `examples/helper`.
        pub fn render_svg(diagram: &str) -> Result<String, Error> {
            // Use the shared Browser instance rather than starting a new one for every render.
            let browser = get_browser()?;

            // The helper files used by the existing example are embedded in the crate and reused
            // here so the rendering logic stays consistent. We embed both the `index.html`
            // which defines a `render` helper and the `mermaid.min.js` runtime so no external
            // network access is required during rendering.
            let mermaid_js = include_str!("../../examples/helper/mermaid.min.js");
            let html_payload = include_str!("../../examples/helper/index.html");

            // Open a new tab and load the embedded helper page via a data URI. Using a data
            // URI keeps the page self-contained and avoids I/O to disk or network.
            let tab = browser
                .new_tab()
                .map_err(|e| Error::new(format!("Failed to open tab: {}", e), ErrorKind::Internal))?;
            tab.navigate_to(&format!("data:text/html;charset=utf-8,{}", html_payload))
                .map_err(|e| Error::new(format!("Failed to navigate: {}", e), ErrorKind::Internal))?;
            // Wait for the navigation to finish so that the `render` helper is present on the page.
            tab.wait_until_navigated()
                .map_err(|e| Error::new(format!("Navigation error: {}", e), ErrorKind::Internal))?;

            // Inject the mermaid runtime into the page so it can compile the diagram string.
            // We pass `false` for the optional await flag because loading the runtime is
            // synchronous for our embedded script.
            tab.evaluate(mermaid_js, false)
                .map_err(|e| Error::new(format!("Failed to evaluate mermaid script: {}", e), ErrorKind::Internal))?;

            // Call the helper `render` function defined in index.html. We must escape the diagram
            // so that single quotes and other characters don't break the JS call. The helper
            // returns a JSON-encoded string on success or an error object on failure.
            let js_call = format!("render('{}')", escape(diagram));
            let data = tab
                .evaluate(&js_call, true)
                .map_err(|e| Error::new(format!("JS execution error: {}", e), ErrorKind::Internal))?;

            let raw = data.value.unwrap_or_default().to_string();
            // The returned value is a quoted string; unescape and strip surrounding quotes.
            let svg = unescape(raw.trim_matches('\"')).unwrap_or_default();

            // Detect whether the helper reported a JS-side error (we return a JSON error object
            // from `examples/helper/index.html` in that case), or whether the result was `null`/empty.
            if svg.trim().starts_with('{') && svg.contains("\"error\"") {
                return Err(Error::new(format!("Mermaid JS error: {}", svg), ErrorKind::InvalidData));
            }

            if svg == "null" || svg.trim().is_empty() {
                // Provide a diagnostic with the raw JS response and a small snippet of the diagram so
                // users can quickly see what went wrong.
                let snippet = if diagram.len() > 200 { format!("{}...", &diagram[..200]) } else { diagram.to_string() };
                return Err(Error::new(
                    format!(
                        "Mermaid failed to compile diagram (raw: {:?}; diagram snippet: {})",
                        raw,
                        snippet
                    ),
                    ErrorKind::InvalidData,
                ));
            }

            Ok(svg)
        }
    }

    /// Apply a direct scale to the provided SVG markup by inserting a `<g transform="scale(...)">`
    /// around the SVG contents and multiplying simple numeric `width`/`height` attributes when present.
    /// It also adjusts a `viewBox` attribute's width/height so that SVGs that rely on viewBox
    /// sizing are not cropped after applying the transform.
    ///
    /// This is intentionally lightweight (string-based) to avoid pulling in an XML parser as a
    /// dependency. It handles common cases emitted by Mermaid and similar tools, but leaves
    /// complex or non-numeric attributes unchanged.
    pub(crate) fn apply_scale_to_svg(svg: &str, scale: f32) -> String {
        if (scale - 1.0).abs() < f32::EPSILON {
            return svg.to_string();
        }

        // Helper to replace numeric attrs like width="123" or width="123px"
        fn replace_dim_attr(s: &str, attr: &str, scale: f32) -> String {
            let mut out = s.to_string();
            let key = format!("{}=\"", attr);
            if let Some(pos) = out.find(&key) {
                let val_start = pos + key.len();
                if let Some(val_end_rel) = out[val_start..].find('"') {
                    let val_end = val_start + val_end_rel;
                    let raw = &out[val_start..val_end];
                    // Detect 'px' unit and strip it for numeric parsing
                    let has_px = raw.ends_with("px");
                    let stripped = raw.trim_end_matches("px");
                    if let Ok(n) = stripped.parse::<f32>() {
                        let newv = n * scale;
                        // Prefer integer formatting when the value is whole
                        let new_str = if (newv - newv.trunc()).abs() < 1e-6 {
                            format!("{}", newv as i64)
                        } else {
                            // Use default formatting for non-integers
                            format!("{}", newv)
                        };
                        let new_full = if has_px { format!("{}px", new_str) } else { new_str };
                        let old = format!("{}=\"{}\"", attr, raw);
                        let new = format!("{}=\"{}\"", attr, new_full);
                        out = out.replacen(&old, &new, 1);
                    }
                }
            }
            out
        }

        // Helper to scale viewBox="minx miny width height" by multiplying width/height.
        fn replace_viewbox(s: &str, scale: f32) -> String {
            let mut out = s.to_string();
            let key = "viewBox=\"";
            if let Some(pos) = out.find(key) {
                let val_start = pos + key.len();
                if let Some(val_end_rel) = out[val_start..].find('"') {
                    let val_end = val_start + val_end_rel;
                    let raw = &out[val_start..val_end];
                    // Split into 4 numbers; be permissive about whitespace
                    let parts: Vec<&str> = raw.split_whitespace().collect();
                    if parts.len() == 4 {
                        if let (Ok(minx), Ok(miny), Ok(w), Ok(h)) = (
                            parts[0].parse::<f32>(),
                            parts[1].parse::<f32>(),
                            parts[2].parse::<f32>(),
                            parts[3].parse::<f32>(),
                        ) {
                            let neww = w * scale;
                            let newh = h * scale;
                            let new_raw = format!("{} {} {} {}", minx, miny, // keep minx/miny unchanged
                                                  // Prefer integer formatting when possible
                                                  if (neww - neww.trunc()).abs() < 1e-6 { format!("{}", neww as i64) } else { format!("{}", neww) },
                                                  if (newh - newh.trunc()).abs() < 1e-6 { format!("{}", newh as i64) } else { format!("{}", newh) });
                            let old = format!("viewBox=\"{}\"", raw);
                            let new = format!("viewBox=\"{}\"", new_raw);
                            out = out.replacen(&old, &new, 1);
                        }
                    }
                }
            }
            out
        }

        // Find the opening <svg ...> tag's end so we can insert a <g> wrapper immediately after it
        if let Some(start) = svg.find("<svg") {
            if let Some(rel_gt) = svg[start..].find('>') {
                let open_end = start + rel_gt + 1;
                let mut opening = svg[start..open_end].to_string();

                // Update width/height and viewBox in the opening tag
                opening = replace_dim_attr(&opening, "width", scale);
                opening = replace_dim_attr(&opening, "height", scale);
                opening = replace_viewbox(&opening, scale);

                // Find closing tag so we can place the closing </g> before it
                if let Some(close_pos) = svg.rfind("</svg>") {
                    let mut out = String::with_capacity(svg.len() + 64);
                    out.push_str(&svg[..start]);
                    out.push_str(&opening);
                    out.push_str(&format!("<g transform=\"scale({})\">", scale));
                    out.push_str(&svg[open_end..close_pos]);
                    out.push_str("</g>");
                    out.push_str("</svg>");
                    return out;
                }

                // If no closing tag found, fall back to a less precise insertion
                let mut out = svg.to_string();
                out.insert_str(open_end, &format!("<g transform=\"scale({})\">", scale));
                out.push_str("</g>");
                return out;
            }
        }

        // Fallback: return original if we couldn't apply a transformation safely
        svg.to_string()
    }

    /// Temporary workaround for a printpdf parsing bug: remove `class="pieCircle"` when
    /// it appears inside `<path ...>` tags only.
    ///
    /// Some SVGs produced by Mermaid attach `class="pieCircle"` (or single-quoted variants)
    /// to `<path>` elements representing chart slices. The `printpdf`/`lopdf` parser
    /// can convert certain SVG constructs into PDF XObjects with an incorrect structure
    /// (a Stream where a Dictionary is expected), which leads to parsing errors such as
    /// "Invalid dictionary reference" and missing fills in the final PDF.
    ///
    /// This helper performs a minimal, local string transformation to remove the
    /// `class="pieCircle"` attribute only for `<path>` tags. It is intentionally small
    /// and conservative to avoid affecting unrelated elements. This function is a
    /// temporary workaround and should be removed when `printpdf` fixes the root cause.
    pub(crate) fn strip_slice_class_from_path_tags(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut i = 0usize;
        while let Some(start) = s[i..].find("<path") {
            let abs_start = i + start;
            // copy up to start
            out.push_str(&s[i..abs_start]);
            // find end of tag
            if let Some(tag_end_rel) = s[abs_start..].find('>') {
                let tag_end = abs_start + tag_end_rel + 1; // include '>'
                let mut tag = s[abs_start..tag_end].to_string();
                // remove occurrences of class="pieCircle" or class='pieCircle' inside the tag
                // only remove the attribute token, leave other attributes intact
                tag = tag.replace(" class=\"pieCircle\"", "");
                tag = tag.replace(" class='pieCircle'", "");
                // Also handle cases where class="pieCircle" may not have a leading space
                tag = tag.replace("class=\"pieCircle\" ", "");
                tag = tag.replace("class='pieCircle' ", "");
                out.push_str(&tag);
                i = tag_end;
                continue;
            } else {
                // malformed tag; copy remainder and break
                out.push_str(&s[abs_start..]);
                i = s.len();
                break;
            }
        }
        if i < s.len() {
            out.push_str(&s[i..]);
        }
        out
    }

    impl Element for Mermaid {
        fn render(&mut self, context: &Context, area: render::Area<'_>, style: Style) -> Result<RenderResult, Error> {
            // Render diagram either as SVG or PNG (raster fallback)
            // Render diagram to SVG string
            let svg = match Self::render_svg(&self.diagram) {
                Ok(s) => s,
                Err(e) => return Err(e),
            };

            // Create an Image from the SVG bytes and delegate rendering to it so we reuse all
            // existing positioning, scaling and link behavior. Apply a light sanitizer to fix
            // common HTML-in-XML problems (notably bare <br> tags) before parsing.
            fn sanitize_svg_for_printpdf(s: &str) -> String {
                // Many HTML-producing tools emit variants of <br> that are not strictly valid
                // XML (e.g., `<br>` or `<BR >`). Some SVG parsers used by downstream libs
                // expect well-formed XML, so normalize these variants to `<br/>` to improve
                // compatibility and avoid parse errors.
                s.replace("<br />", "<br/>")
                    .replace("<br >", "<br/>")
                    .replace("<BR />", "<br/>")
                    .replace("<BR >", "<br/>")
                    .replace("<br>", "<br/>")
                    .replace("<BR>", "<br/>")
            }

            // Apply an optional SVG-scale transform before sanitizing and parsing.
            let scaled_svg = if self.scale != 1.0 {
                apply_scale_to_svg(&svg, self.scale)
            } else {
                svg.clone()
            };

            // Temporary workaround: some SVGs emitted by Mermaid attach `class` attributes
            // to arc `<path>` elements (e.g. `class="slice"`) which can cause the
            // PDF SVG parser to generate malformed XObjects resulting in render issues
            // (see project issue TODO: replace with real issue link). Strip only
            // `class="slice"` attributes when they appear inside `<path>` tags.
            // This should be removed once the upstream parser handles these cases.
            let preprocessed = strip_slice_class_from_path_tags(&scaled_svg);
            let sanitized = sanitize_svg_for_printpdf(&preprocessed);

            // Prefer the sanitized SVG; if that fails, try the original raw SVG so we don't hide
            // surprising parsing behavior. If both fail and debugging is enabled, dump raw SVG.
            match crate::elements::Image::from_svg_string(&sanitized) {
                Ok(mut img) => {
                    if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
                        eprintln!("--- BEGIN MERMAID SOURCE ---\n{}--- END MERMAID SOURCE ---", self.diagram);
                        eprintln!("--- BEGIN MERMAID SANITIZED SVG ---\n{}\n--- END MERMAID SANITIZED SVG ---", sanitized);
                    }
                    img = img.with_alignment(self.alignment);
                    if let Some(pos) = self.position {
                        img = img.with_position(pos);
                    }
                    if let Some(link) = &self.link {
                        img = img.with_link(link.clone());
                    }
                    img.render(context, area, style)
                }
                Err(_san_err) => {
                    match crate::elements::Image::from_svg_string(&svg) {
                        Ok(mut img) => {
                            img = img.with_alignment(self.alignment);
                            if let Some(pos) = self.position {
                                img = img.with_position(pos);
                            }
                            if let Some(link) = &self.link {
                                img = img.with_link(link.clone());
                            }
                            img.render(context, area, style)
                        }
                        Err(err) => {
                            // Parsing failed even for raw SVG: if debugging is enabled, dump the raw
                            // SVG to stderr for analysis; otherwise fall back to a placeholder.
                            if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
                                eprintln!("--- BEGIN MERMAID SVG (debug dump) ---");
                                eprintln!("{}", svg);
                                eprintln!("--- END MERMAID SVG ---");
                            }

                            // If SVG parsing fails even for the raw SVG, provide a visible,
                            // non-panicking fallback: log the error and render a short
                            // diagnostic paragraph in the PDF so users see what happened.
                            let msg = format!("Mermaid SVG parsing failed: {}", err);
                            eprintln!("{}", msg);
                            let mut p = crate::elements::Paragraph::new(msg);
                            // Style to make the error visible but unobtrusive
                            p = p.styled_string("Mermaid rendering failed", crate::style::Style::new().with_font_size(9));
                            p.render(context, area, style)
                        }
                    }
                }
            }
        }
    }
}

/// An element that renders a Mermaid diagram into the PDF as an SVG image.
///
/// This element is only available when the `mermaid` feature is enabled.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "mermaid")]
/// # {
/// use genpdfi_extended::elements::Mermaid;
/// use genpdfi_extended::Alignment;
/// let mut m = Mermaid::new("graph TB\na-->b");
/// m = m.with_alignment(Alignment::Center);
/// // Add `m` to a document to render it in a PDF
/// # }
/// ```
#[cfg(feature = "mermaid")]
#[derive(Clone, Debug)]
pub struct Mermaid {
    diagram: String,

    /// Scaling factor applied directly to the generated SVG. A value of 1.0 means no scaling.
    scale: f32,

    /// Positioning and presentation helpers mirrored from `Image`.
    alignment: Alignment,
    position: Option<Position>,
    link: Option<String>,
}

#[cfg(feature = "mermaid")]
impl Mermaid {
    /// Create a new Mermaid element from the source string.
    pub fn new<S: Into<String>>(diagram: S) -> Self {
        Mermaid {
            diagram: diagram.into(),
            scale: 1.0,
            alignment: Alignment::default(),
            position: None,
            link: None,
        }
    }

    /// Ensure the global headless Chrome instance is up and running.
    ///
    /// Useful for examples that want to pre-check the environment before building the full PDF.
    pub fn ensure_browser() -> Result<(), crate::error::Error> {
        inner::ensure_browser()
    }

    /// Set alignment used when no absolute position is given.
    pub fn with_alignment(mut self, a: Alignment) -> Self {
        self.alignment = a;
        self
    }

    /// Set an absolute position for the element.
    pub fn with_position(mut self, p: Position) -> Self {
        self.position = Some(p);
        self
    }

    /// Attach a hyperlink to the rendered SVG.
    pub fn with_link<S: Into<String>>(mut self, link: S) -> Self {
        self.link = Some(link.into());
        self
    }

    /// Set the scale to apply directly to the generated SVG.
    ///
    /// The scaling is applied to the SVG markup (a wrapper `<g transform="scale(...)">`
    /// is inserted) and numeric `width`/`height` attributes are multiplied accordingly.
    pub fn with_scale(mut self, s: f32) -> Self {
        self.scale = s;
        self
    }
}

#[cfg(all(test, feature = "mermaid"))]
mod tests {
    use super::*;
    use crate::render::Renderer;
    use crate::style::Style;
    use crate::Element;

    #[test]
    fn create_mermaid_element_works() {
        let _ = Mermaid::new("graph TB\na-->b");
    }

    #[test]
    fn render_mermaid_smoke() {
        // Create a renderer and a drawing area similar to other image tests.
        let mut r = Renderer::new(Size::new(200.0, 200.0), "t").expect("renderer");
        let area = r.first_page().first_layer().area();

        // Prepare a very small context - Image rendering does not depend on font metrics but
        // Context is required by the Element API.
        let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
        let fd = crate::fonts::FontData::new(data, None).expect("font data");
        let family = crate::fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };
        let cache = crate::fonts::FontCache::new(family);
        let context = crate::Context::new(cache);

        let mut m = Mermaid::new("graph TB\na-->b");

        match m.render(&context, area.clone(), Style::new()) {
            Ok(res) => {
                // If rendering succeeded we expect a non-zero size
                assert!(res.size.width.0 > 0.0);
                assert!(res.size.height.0 > 0.0);
            }
            Err(e) => {
                // If headless chrome is not available, we treat that as an environment limitation
                // and skip the test by returning early. The error message from headless_chrome
                // is propagated by our wrapper and contains "Failed to start headless chrome".
                let s = format!("{}", e);
                if s.contains("Failed to start headless chrome") {
                    return;
                }
                // Otherwise fail the test so unexpected problems surface.
                panic!("Mermaid render failed: {}", e);
            }
        }
    }

    #[test]
    fn invalid_syntax_returns_error() {
        let mut m = Mermaid::new("grph TB\na-->b");
        // Try to render; if browser is not available we skip like above
        let mut r = Renderer::new(Size::new(200.0, 200.0), "t").expect("renderer");
        let area = r.first_page().first_layer().area();
        let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
        let fd = crate::fonts::FontData::new(data, None).expect("font data");
        let family = crate::fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };
        let cache = crate::fonts::FontCache::new(family);
        let context = crate::Context::new(cache);

        match m.render(&context, area.clone(), Style::new()) {
            Ok(_) => panic!("Expected compilation error for invalid mermaid syntax"),
            Err(e) => {
                let s = format!("{}", e);
                if s.contains("Failed to start headless chrome") {
                    return;
                }
                // For invalid mermaid syntax compile should fail. The helper may return
                // a JS-side error object or a compilation failure message, accept both.
                assert!(s.contains("Mermaid failed to compile") || s.contains("compile") || s.contains("Mermaid JS error"));
            }
        }
    }

    #[test]
    fn sanitize_br_conversion() {
        // The sanitizer should convert bare <br> variants to XML-friendly <br/>
        let input = "<p>Line1<br>Line2<br >Line3<BR>End<br /></p>";
        let got = {
            fn sanitize_svg_for_printpdf(s: &str) -> String {
                s.replace("<br />", "<br/>")
                    .replace("<br >", "<br/>")
                    .replace("<BR />", "<br/>")
                    .replace("<BR >", "<br/>")
                    .replace("<br>", "<br/>")
                    .replace("<BR>", "<br/>")
            }
            sanitize_svg_for_printpdf(input)
        };
        assert_eq!(got, "<p>Line1<br/>Line2<br/>Line3<br/>End<br/></p>");
    }

    #[test]
    fn apply_scale_to_svg_works() {
        let input = "<svg width=\"100\" height=\"50\" viewBox=\"0 0 100 50\"><rect /></svg>";
        let got = inner::apply_scale_to_svg(input, 2.0);
        assert!(got.contains("transform=\"scale(2)\""));
        assert!(got.contains("width=\"200\""));
        assert!(got.contains("height=\"100\""));
        assert!(got.contains("viewBox=\"0 0 200 100\""));
    }

    #[test]
    fn apply_scale_to_svg_viewbox_only_and_px() {
        // Case with only viewBox (no explicit width/height): the viewBox width/height should grow
        let input = "<svg viewBox=\"0 0 100 50\"><rect /></svg>";
        let got = inner::apply_scale_to_svg(input, 2.0);
        assert!(got.contains("viewBox=\"0 0 200 100\""));

        // Case with px units
        let input2 = "<svg width=\"100px\" height=\"50px\"><rect /></svg>";
        let got2 = inner::apply_scale_to_svg(input2, 1.5);
        assert!(got2.contains("width=\"150px\""));
        assert!(got2.contains("height=\"75px\""));
    }

    #[test]
    fn strip_slice_class_from_paths_works() {
        let input = "<svg><path class=\"pieCircle\" d=\"M...Z\"/><path class=\"other\" d=\"M...Z\"/><text class=\"pieCircle\">X</text></svg>";
        // The class on the path should be removed but the text's class should remain
        let out = inner::strip_slice_class_from_path_tags(input);
        assert!(out.contains("<path d=\"M...Z\"/>") );
        assert!(out.contains("class=\"pieCircle\">X</text>") );
        assert!(out.contains("class=\"other\""));
    }
}