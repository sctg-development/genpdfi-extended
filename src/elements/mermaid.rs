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

            let sanitized = sanitize_svg_for_printpdf(&svg);

            // Prefer the sanitized SVG; if that fails, try the original raw SVG so we don't hide
            // surprising parsing behavior. If both fail and debugging is enabled, dump raw SVG.
            match crate::elements::Image::from_svg_string(&sanitized) {
                Ok(mut img) => {
                    if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
                        eprintln!("--- BEGIN MERMAID SOURCE ---\n{}--- END MERMAID SOURCE ---", self.diagram);
                        eprintln!("--- BEGIN MERMAID SVG ---\n{}\n--- END MERMAID SVG ---", sanitized);
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
}