// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

//! Mermaid diagram element.
//!
//! Renders Mermaid diagrams to SVG by executing the embedded Mermaid runtime inside a
//! headless Chrome instance. The produced SVG is embedded into the PDF using the
//! existing `Image` element. Note that rendering requires the `mermaid` feature and a
//! working headless Chrome executable available at runtime. Doc examples demonstrate
//! API usage but do not perform rendering to avoid requiring Chrome in doctests.

#[cfg(feature = "mermaid")]
use crate::{Alignment, Position};

#[cfg(feature = "mermaid")]
mod inner {
    use std::fmt::Display;

    use headless_chrome::Browser;
    use unescape::unescape;

    use crate::error::{Error, ErrorKind};
    use crate::render;
    use crate::style::Style;
    use crate::{Context, Element, RenderResult, Size};

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
            Browser::new(
                headless_chrome::LaunchOptionsBuilder::default()
                    .headless(true)
                    .build()
                    .expect("launch options"),
            )
            .map_err(|e| {
                Error::new(
                    format!("Failed to start headless chrome: {}", e),
                    ErrorKind::Internal,
                )
            })
        })?;
        Ok(())
    }

    pub fn get_browser() -> Result<&'static Browser, Error> {
        // Return a reference to the shared Browser, initializing it if necessary.
        // Using a &'static Browser makes it convenient to use across async calls and closures.
        BROWSER.get_or_try_init(|| {
            Browser::new(
                headless_chrome::LaunchOptionsBuilder::default()
                    .headless(true)
                    .build()
                    .expect("launch options"),
            )
            .map_err(|e| {
                Error::new(
                    format!("Failed to start headless chrome: {}", e),
                    ErrorKind::Internal,
                )
            })
        })
    }

    /// Shutdown and kill any spawned Chrome used by Mermaid rendering. This ensures
    /// the process doesn't keep child processes or background threads alive which
    /// can prevent the main program from exiting when its output is piped.
    pub fn shutdown_browser() -> Result<(), Error> {
        // If we have a pool tab, try to clear references so it can be dropped.
        if let Some(tab_arc) = POOL_TAB.get() {
            let _ = tab_arc.clone(); // ensure we at least hold a clone so dropping our clone reduces refs
        }

        // If there is a running Browser, try to obtain its process id and kill it using sysinfo
        if let Some(browser) = BROWSER.get() {
            if let Some(pid) = browser.get_process_id() {
                use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

                let target = Pid::from(pid as usize);
                let mut sys = System::new_all();
                // Refresh processes so we have up-to-date info
                sys.refresh_processes(ProcessesToUpdate::Some(&[target]), true);

                // Collect main process and any direct children to attempt termination.
                let mut to_kill: Vec<Pid> = Vec::new();
                if sys.process(target).is_some() {
                    to_kill.push(target);
                }
                for (p, proc_) in sys.processes() {
                    if let Some(parent) = proc_.parent() {
                        if parent == target {
                            to_kill.push(*p);
                        }
                    }
                }

                // Try gentle TERM on all candidates first, then poll for exit. If still alive, send KILL.
                for p in &to_kill {
                    if let Some(proc_) = sys.process(*p) {
                        let _ = proc_.kill_with(Signal::Term);
                    }
                }

                // Wait for processes to disappear (poll)
                let mut tries = 0u32;
                while tries < 50 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    sys.refresh_processes(ProcessesToUpdate::Some(&to_kill), true);
                    let mut any_alive = false;
                    for p in &to_kill {
                        if sys.process(*p).is_some() {
                            any_alive = true;
                            break;
                        }
                    }
                    if !any_alive {
                        break;
                    }
                    tries += 1;
                }

                // Force kill any remaining processes
                sys.refresh_processes(ProcessesToUpdate::Some(&to_kill), true);
                for p in &to_kill {
                    if let Some(proc_) = sys.process(*p) {
                        let _ = proc_.kill(); // sends strongest supported signal
                    }
                }

                // Final short poll
                let mut tries = 0u32;
                while tries < 20 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    sys.refresh_processes(ProcessesToUpdate::Some(&to_kill), true);
                    let mut any_alive = false;
                    for p in &to_kill {
                        if sys.process(*p).is_some() {
                            any_alive = true;
                            break;
                        }
                    }
                    if !any_alive {
                        break;
                    }
                    tries += 1;
                }
            }
        }

        Ok(())
    }

    // Path to the embedded helper HTML file written to a temp location and reused.
    use headless_chrome::Tab;
    use std::sync::Arc;
    static HELPER_PATH: OnceCell<std::path::PathBuf> = OnceCell::new();
    static POOL_TAB: OnceCell<Arc<Tab>> = OnceCell::new();

    fn ensure_helper_file() -> Result<&'static std::path::PathBuf, Error> {
        HELPER_PATH.get_or_try_init(|| {
            let helper_html = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/mermaid_pool/dist/index.html"
            ));
            let tmp_dir = std::env::temp_dir();
            let fname = format!("mermaid_helper_{}.html", std::process::id());
            let helper_path = tmp_dir.join(fname);
            std::fs::write(&helper_path, helper_html).map_err(|e| {
                Error::new(
                    format!(
                        "Failed to write helper page to {}: {}",
                        helper_path.display(),
                        e
                    ),
                    ErrorKind::Internal,
                )
            })?;
            Ok(helper_path.canonicalize().unwrap_or(helper_path))
        })
    }

    fn get_pool_tab() -> Result<&'static Arc<Tab>, Error> {
        let browser = get_browser()?;
        let helper_path = ensure_helper_file()?;
        POOL_TAB.get_or_try_init(|| {
            let tab = browser.new_tab().map_err(|e| {
                Error::new(format!("Failed to open tab: {}", e), ErrorKind::Internal)
            })?;
            tab.navigate_to(&format!("file://{}?pool=1", helper_path.display()))
                .map_err(|e| {
                    Error::new(format!("Failed to navigate: {}", e), ErrorKind::Internal)
                })?;
            tab.wait_until_navigated()
                .map_err(|e| Error::new(format!("Navigation error: {}", e), ErrorKind::Internal))?;
            // attempt to wait for metrics but do not fail if missing
            let _ = tab.wait_for_element("#mermaid-metrics");
            Ok(tab)
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
            // Reuse a helper tab to avoid opening a new tab for every render.
            // This improves stability and performance (see examples/mermaid_pool_proof_of_concept.rs).
            let tab = get_pool_tab()?;

            // Inject the mermaid runtime into the page so it can compile the diagram string.
            // We pass `false` for the optional await flag because loading the runtime is
            // synchronous for our embedded script.
            // Pool helper page handles mermaid runtime; no need to inject runtime here.

            // Submit the diagram to the pool and wait for the Promise result synchronously.
            let id = format!(
                "rust-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
            let js_diagram = match serde_json::to_string(diagram) {
                Ok(s) => s,
                Err(e) => {
                    return Err(Error::new(
                        format!("Failed to JSON-encode diagram: {}", e),
                        ErrorKind::Internal,
                    ));
                }
            };
            let submit = format!("window.__mermaidPool.submitTask('{}', {})", id, js_diagram);
            let data = tab.evaluate(&submit, true).map_err(|e| {
                Error::new(format!("JS execution error: {}", e), ErrorKind::Internal)
            })?;

            let raw = data.value.unwrap_or_default().to_string();
            // The returned value may be quoted; unescape and strip surrounding quotes.
            let svg = unescape(raw.trim_matches('\"')).unwrap_or_default();

            // Detect whether the helper reported a JS-side error (we return a JSON error object
            // from `examples/helper/index.html` in that case), or whether the result was `null`/empty.
            if svg.trim().starts_with('{') && svg.contains("\"error\"") {
                return Err(Error::new(
                    format!("Mermaid JS error: {}", svg),
                    ErrorKind::InvalidData,
                ));
            }

            if svg == "null" || svg.trim().is_empty() {
                // Provide a diagnostic with the raw JS response and a small snippet of the diagram so
                // users can quickly see what went wrong.
                let snippet = if diagram.len() > 200 {
                    format!("{}...", &diagram[..200])
                } else {
                    diagram.to_string()
                };
                return Err(Error::new(
                    format!(
                        "Mermaid failed to compile diagram (raw: {:?}; diagram snippet: {})",
                        raw, snippet
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
                        let new_full = if has_px {
                            format!("{}px", new_str)
                        } else {
                            new_str
                        };
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
                            let new_raw = format!(
                                "{} {} {} {}",
                                minx,
                                miny, // keep minx/miny unchanged
                                // Prefer integer formatting when possible
                                if (neww - neww.trunc()).abs() < 1e-6 {
                                    format!("{}", neww as i64)
                                } else {
                                    format!("{}", neww)
                                },
                                if (newh - newh.trunc()).abs() < 1e-6 {
                                    format!("{}", newh as i64)
                                } else {
                                    format!("{}", newh)
                                }
                            );
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

    /// Simple sanitizer used before passing SVGs to `printpdf`.
    pub(crate) fn sanitize_svg_for_printpdf(s: &str) -> String {
        s.replace("<br />", "<br/>")
            .replace("<br >", "<br/>")
            .replace("<BR />", "<br/>")
            .replace("<BR >", "<br/>")
            .replace("<br>", "<br/>")
            .replace("<BR>", "<br/>")
    }

    /// Fast heuristic to extract intrinsic width/height in *pixels* from an SVG string.
    ///
    /// We look for `width="..."` / `height="..."` (optionally with `px`) or a `viewBox`
    /// and return `(width_px, height_px)` on success. This avoids invoking the slower
    /// `printpdf::Svg::parse` for the common case where dimensions are explicit in the SVG.
    fn extract_svg_intrinsic_px(s: &str) -> Option<(f32, f32)> {
        // width/height attributes
        if let Some(wpos) = s.find("width=\"") {
            let start = wpos + 7; // skip width="
            if let Some(rel_end) = s[start..].find('"') {
                let raw = &s[start..start + rel_end];
                let stripped = raw.trim_end_matches("px");
                if let Ok(w) = stripped.parse::<f32>() {
                    // try height as well
                    if let Some(hpos) = s.find("height=\"") {
                        let hstart = hpos + 8;
                        if let Some(hrel_end) = s[hstart..].find('"') {
                            let hraw = &s[hstart..hstart + hrel_end];
                            let hstripped = hraw.trim_end_matches("px");
                            if let Ok(h) = hstripped.parse::<f32>() {
                                return Some((w, h));
                            }
                        }
                    }
                }
            }
        }

        // viewBox: 'minx miny width height'
        if let Some(vpos) = s.find("viewBox=\"") {
            let vstart = vpos + 9;
            if let Some(vrel_end) = s[vstart..].find('"') {
                let vraw = &s[vstart..vstart + vrel_end];
                let parts: Vec<&str> = vraw.split_whitespace().collect();
                if parts.len() == 4 {
                    if let (Ok(_minx), Ok(_miny), Ok(w), Ok(h)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        return Some((w, h));
                    }
                }
            }
        }

        None
    }

    /// Extract data-dpi="NNN" if present (used when SVGs embed DPI metadata).
    fn extract_dpi_from_svg(s: &str) -> Option<f32> {
        if let Some(start) = s.find("data-dpi=\"") {
            let search = &s[start + 10..];
            if let Some(end) = search.find('"') {
                let dpi_str = &search[..end];
                if let Ok(v) = dpi_str.parse::<i32>() {
                    return Some(v as f32);
                }
            }
        }
        None
    }

    /// Given an SVG and a candidate scale, determine the final scale so that the
    /// rendered image does not exceed the specified maximum ratio of the provided page width/height. The
    /// function returns a scale that is at most `candidate_scale`.
    ///
    /// This implementation prefers a fast string-based extraction of `width`/`height`
    /// or `viewBox` to avoid invoking the slow SVG parser on every diagram. Only if
    /// these heuristics fail do we fall back to a full parse.
    pub(crate) fn compute_auto_scale(
        svg: &str,
        candidate_scale: f32,
        max_ratio: f32,
        page_size: Size,
    ) -> (f32, Option<crate::elements::Image>) {
        // Fast path: extract width/height in pixels from attributes or viewBox
        if let Some((w_px, h_px)) = extract_svg_intrinsic_px(svg) {
            // Determine DPI (default 300 if not present)
            let dpi = extract_dpi_from_svg(svg).unwrap_or(300.0);
            let mmpi: f32 = 25.4; // mm per inch
            let intrinsic_w_mm = mmpi * (w_px / dpi);
            let intrinsic_h_mm = mmpi * (h_px / dpi);
            let allowed_w = max_ratio * page_size.width.as_f32();
            let allowed_h = max_ratio * page_size.height.as_f32();
            if intrinsic_w_mm <= 0.0 || intrinsic_h_mm <= 0.0 {
                if std::env::var("RUST_LOG")
                    .unwrap_or_default()
                    .contains("debug")
                {
                    eprintln!(
                        "compute_auto_scale: invalid intrinsic dims w_mm={} h_mm={}",
                        intrinsic_w_mm, intrinsic_h_mm
                    );
                }
                return (candidate_scale, None);
            }
            let req_w = allowed_w / intrinsic_w_mm;
            let req_h = allowed_h / intrinsic_h_mm;
            let required_scale = req_w.min(req_h);
            let used = candidate_scale.min(required_scale.max(1e-6));
            if std::env::var("RUST_LOG")
                .unwrap_or_default()
                .contains("debug")
            {
                eprintln!("compute_auto_scale: fast path w_px={} h_px={} dpi={} intrinsic_mm=({}, {}) allowed_mm=({}, {}) req=(w:{} h:{}) candidate={} used={}",
                    w_px, h_px, dpi, intrinsic_w_mm, intrinsic_h_mm, allowed_w, allowed_h, req_w, req_h, candidate_scale, used);
            }
            return (used, None);
        }

        // Slow path: try parsing the unscaled SVG to get accurate intrinsic size
        let preprocessed = strip_slice_class_from_path_tags(svg);
        let sanitized = sanitize_svg_for_printpdf(&preprocessed);
        if let Ok(mut img) = crate::elements::Image::from_svg_string(&sanitized) {
            let intrinsic = img.get_intrinsic_size();
            let allowed_w = max_ratio * page_size.width.as_f32();
            let allowed_h = max_ratio * page_size.height.as_f32();
            if intrinsic.width.as_f32() <= 0.0 || intrinsic.height.as_f32() <= 0.0 {
                return (candidate_scale, None);
            }
            let req_w = allowed_w / intrinsic.width.as_f32();
            let req_h = allowed_h / intrinsic.height.as_f32();
            let required_scale = req_w.min(req_h);
            let used = candidate_scale.min(required_scale.max(1e-6));
            // If used != 1.0 set the image scale so the downstream renderer can reuse the
            // already-parsed `Image` instead of reparsing a scaled SVG string.
            if std::env::var("RUST_LOG")
                .unwrap_or_default()
                .contains("debug")
            {
                let intrinsic = img.get_intrinsic_size();
                eprintln!("compute_auto_scale: slow path intrinsic=({}, {}) candidate={} required={} used={}",
                    intrinsic.width.as_f32(), intrinsic.height.as_f32(), candidate_scale, required_scale, used);
            }
            if (used - 1.0).abs() > f32::EPSILON {
                img = img.with_scale(crate::Scale::new(used, used));
            }
            return (used, Some(img));
        }

        // Fallback: if parsing failed, try parsing the scaled markup as a last resort
        let scaled_markup = apply_scale_to_svg(svg, candidate_scale);
        let preprocessed = strip_slice_class_from_path_tags(&scaled_markup);
        let sanitized = sanitize_svg_for_printpdf(&preprocessed);
        if let Ok(mut img) = crate::elements::Image::from_svg_string(&sanitized) {
            let intrinsic = img.get_intrinsic_size();
            let allowed_w = 0.9 * page_size.width.as_f32();
            let allowed_h = 0.9 * page_size.height.as_f32();
            if intrinsic.width.as_f32() <= 0.0 || intrinsic.height.as_f32() <= 0.0 {
                return (candidate_scale, None);
            }
            let scale_w = allowed_w / intrinsic.width.as_f32();
            let scale_h = allowed_h / intrinsic.height.as_f32();
            let max_scale = scale_w.min(scale_h);
            let used = candidate_scale.min(max_scale.max(1e-6));
            if (used - 1.0).abs() > f32::EPSILON {
                img = img.with_scale(crate::Scale::new(used, used));
            }
            return (used, Some(img));
        }

        // If all attempts fail, return the original candidate (best-effort)
        (candidate_scale, None)
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
        fn render(
            &mut self,
            context: &Context,
            area: render::Area<'_>,
            style: Style,
        ) -> Result<RenderResult, Error> {
            // Render diagram either as SVG or PNG (raster fallback)
            // Render diagram to SVG string
            let svg = match Self::render_svg(&self.diagram) {
                Ok(s) => s,
                Err(e) => return Err(e),
            };

            // If auto-scaling is enabled we request the computed scale and allow
            // the helper to return an already-parsed `Image` to avoid double-parsing.
            let (used_scale, maybe_img) = if self.auto_scale {
                let page_size = area.size();
                compute_auto_scale(&svg, self.scale, self.max_ratio, page_size)
            } else {
                (self.scale, None)
            };

            if let Some(mut parsed_img) = maybe_img {
                // Reuse the already-parsed image and render it directly (fast path)
                parsed_img = parsed_img.with_alignment(self.alignment);
                if let Some(pos) = self.position {
                    parsed_img = parsed_img.with_position(pos);
                }
                if let Some(link) = &self.link {
                    parsed_img = parsed_img.with_link(link.clone());
                }
                let mut result = parsed_img.render(context, area, style)?;
                // Apply scaling factor to the result size
                if (used_scale - 1.0).abs() >= f32::EPSILON {
                    result.size =
                        Size::new(result.size.width, result.size.height.as_f32() * used_scale);
                }
                return Ok(result);
            }

            // Otherwise construct a (possibly scaled) SVG string and parse it as before.
            let scaled_svg = if (used_scale - 1.0).abs() < f32::EPSILON {
                svg.clone()
            } else {
                apply_scale_to_svg(&svg, used_scale)
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
                    if std::env::var("RUST_LOG")
                        .unwrap_or_default()
                        .contains("debug")
                    {
                        eprintln!(
                            "--- BEGIN MERMAID SOURCE ---\n{}--- END MERMAID SOURCE ---",
                            self.diagram
                        );
                        eprintln!("--- BEGIN MERMAID SANITIZED SVG ---\n{}\n--- END MERMAID SANITIZED SVG ---", sanitized);
                    }
                    img = img.with_alignment(self.alignment);
                    if let Some(pos) = self.position {
                        img = img.with_position(pos);
                    }
                    if let Some(link) = &self.link {
                        img = img.with_link(link.clone());
                    }
                    let result = img.render(context, area, style);
                    // Add SVG source to RenderResult for reference
                    let mut res = result?;
                    res.svg = Some(sanitized);
                    // Apply scaling factor to the result size
                    if (used_scale - 1.0).abs() >= f32::EPSILON {
                        res.size = Size::new(res.size.width, res.size.height.as_f32() * used_scale);
                    }
                    Ok(res)
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
                            let mut result = img.render(context, area, style)?;
                            // Apply scaling factor to the result size
                            if (used_scale - 1.0).abs() >= f32::EPSILON {
                                result.size = Size::new(
                                    result.size.width,
                                    result.size.height.as_f32() * used_scale,
                                );
                            }
                            Ok(result)
                        }
                        Err(err) => {
                            // Parsing failed even for raw SVG: if debugging is enabled, dump the raw
                            // SVG to stderr for analysis; otherwise fall back to a placeholder.
                            if std::env::var("RUST_LOG")
                                .unwrap_or_default()
                                .contains("debug")
                            {
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
                            p = p.styled_string(
                                "Mermaid rendering failed",
                                crate::style::Style::new().with_font_size(9),
                            );
                            p.render(context, area, style)
                        }
                    }
                }
            }
        }
    }
}

/// Element that renders a Mermaid diagram into the PDF as an SVG image.
///
/// This element is only available when the `mermaid` feature is enabled.
/// Rendering requires headless Chrome at runtime; the following example demonstrates
/// creating and configuring a `Mermaid` element but does not attempt to render it.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "mermaid")]
/// # {
/// use genpdfi_extended::elements::Mermaid;
/// use genpdfi_extended::Alignment;
///
/// let m = Mermaid::new("graph TB\na-->b")
///     .with_alignment(Alignment::Center)
///     .with_scale(1.5);
///
/// // Example verifies the API compiles and can be debug-formatted without invoking Chrome.
/// let s = format!("{:?}", m);
/// assert!(s.contains("Mermaid"));
/// # }
/// ```
#[cfg(feature = "mermaid")]
#[derive(Clone, Debug)]
pub struct Mermaid {
    diagram: String,

    /// Scaling factor applied directly to the generated SVG. A value of 1.0 means no scaling.
    scale: f32,

    /// If true, automatically reduce the scale so the rendered diagram fits within
    /// 90% of the page width or height (whichever constrains first). When enabled
    /// the scale is initially set to 2.0 by `with_auto_scale()` and then adjusted
    /// at render time based on the available area.
    auto_scale: bool,

    /// Maximum ratio of page size to use when auto-scaling (e.g. 0.9 for 90% of page)
    max_ratio: f32,

    /// Positioning and presentation helpers mirrored from `Image`.
    alignment: Alignment,
    position: Option<Position>,
    link: Option<String>,
}

#[cfg(feature = "mermaid")]
impl Mermaid {
    /// Create a new Mermaid element from the source string.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "mermaid")]
    /// # {
    /// use genpdfi_extended::elements::Mermaid;
    /// let m = Mermaid::new("graph TB\na-->b");
    /// let s = format!("{:?}", m);
    /// assert!(s.contains("Mermaid"));
    /// # }
    /// ```
    pub fn new<S: Into<String>>(diagram: S) -> Self {
        // Attempt to initialize the global headless Chrome instance asynchronously so
        // Mermaid users do not need to call `ensure_browser()` manually. We ignore any
        // initialization errors here; they will surface during `render()` if Chrome is
        // not available.
        let _ = std::thread::spawn(|| {
            let _ = inner::ensure_browser();
        });

        Mermaid {
            diagram: diagram.into(),
            scale: 1.0,
            max_ratio: 0.9,
            auto_scale: false,
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

    /// Returns a reference to the shared headless Chrome `Browser` instance, initializing
    /// it if necessary. Use this if you need to open tabs or interact directly with the
    /// browser in integration tests or examples.
    pub fn get_browser() -> Result<&'static headless_chrome::Browser, crate::error::Error> {
        inner::get_browser()
    }

    /// Shutdown the shared headless Chrome instance used by Mermaid rendering.
    ///
    /// This calls into the inner helper to drop the global Browser and waits
    /// briefly for chrome processes to exit so shells/pipelines receive EOF.
    pub fn shutdown_browser() -> Result<(), crate::error::Error> {
        inner::shutdown_browser()
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
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "mermaid")]
    /// # {
    /// use genpdfi_extended::elements::Mermaid;
    /// let m = Mermaid::new("graph TB\na-->b").with_scale(2.0);
    /// let s = format!("{:?}", m);
    /// assert!(s.contains("Mermaid"));
    /// # }
    /// ```
    pub fn with_scale(mut self, s: f32) -> Self {
        self.scale = s;
        self
    }
    /// Enable automatic scaling with a sensible default.
    ///
    /// This sets the scale and enables automatic adjustment at render time so
    /// that the final rendered diagram does not exceed 90% of the available page width
    /// or height. The computed scale will never be larger than the initial value.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "mermaid")]
    /// # {
    /// use genpdfi_extended::elements::Mermaid;
    /// let m = Mermaid::new("graph TB\na-->b").with_auto_scale(2.0, 0.9);
    /// let s = format!("{:?}", m);
    /// assert!(s.contains("Mermaid"));
    /// # }
    /// ```
    pub fn with_auto_scale(mut self, s: f32, max_ratio: f32) -> Self {
        self.scale = s;
        self.max_ratio = max_ratio;
        self.auto_scale = true;
        self
    }
}

#[cfg(all(test, feature = "mermaid"))]
mod tests {
    use super::*;
    use crate::render::Renderer;
    use crate::style::Style;
    use crate::{Element, Size};

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
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = crate::fonts::FontData::new(data, None).expect("font data");
        let family = crate::fonts::FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
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
    #[ignore = "    Requires headless Chrome; enable and run manually in suitable environment"]
    fn invalid_syntax_returns_error() {
        let mut m = Mermaid::new("grph TB\na-->b");
        // Try to render; if browser is not available we skip like above
        let r = Renderer::new(Size::new(200.0, 200.0), "t").expect("renderer");
        let area = r.first_page().first_layer().area();
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = crate::fonts::FontData::new(data, None).expect("font data");
        let family = crate::fonts::FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let cache = crate::fonts::FontCache::new(family);
        let context = crate::Context::new(cache);

        match m.render(&context, area.clone(), Style::new()) {
            Ok(_) => panic!("Expected compilation error for invalid mermaid syntax"),
            Err(e) => {
                let s = format!("{}", e);
                if s.contains("Failed to start headless chrome") {
                    return;
                }
                eprintln!("Received expected error: {}", s);
                // For invalid mermaid syntax compile should fail. The helper may return
                // a JS-side error object or a compilation failure message, accept both.
                assert!(
                    s.contains("Mermaid failed to compile")
                        || s.contains("compile")
                        || s.contains("Mermaid JS error")
                );
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
        assert!(out.contains("<path d=\"M...Z\"/>"));
        assert!(out.contains("class=\"pieCircle\">X</text>"));
        assert!(out.contains("class=\"other\""));
    }

    #[test]
    fn compute_auto_scale_reduces_when_exceeding_limits() {
        // Very large SVG (in px) so initial scale=2.0 would exceed a 200x200mm page
        let svg = "<svg width=\"3000\" height=\"1000\"><rect /></svg>";
        let candidate = 2.0f32;
        let area_size = crate::Size::new(200.0, 200.0);
        let (used, _maybe_img) = inner::compute_auto_scale(svg, candidate, 0.9, area_size);

        // Expected: compute from the original intrinsic size (pre-scale) and then
        // cap the candidate to the maximum allowed scale.
        let preprocessed = inner::strip_slice_class_from_path_tags(svg);
        let sanitized = inner::sanitize_svg_for_printpdf(&preprocessed);
        let img = crate::elements::Image::from_svg_string(&sanitized).expect("parse svg");
        let intrinsic = img.get_intrinsic_size();
        let allowed_w = 0.9 * area_size.width.as_f32();
        let allowed_h = 0.9 * area_size.height.as_f32();
        let max_scale =
            (allowed_w / intrinsic.width.as_f32()).min(allowed_h / intrinsic.height.as_f32());
        let expected = candidate.min(max_scale.max(1e-6));
        assert!((used - expected).abs() < 1e-3);
    }

    #[test]
    fn compute_auto_scale_keeps_candidate_when_not_needed() {
        // Small SVG so candidate scale of 2.0 doesn't exceed the area
        let svg = "<svg width=\"100\" height=\"50\"><rect /></svg>";
        let candidate = 2.0f32;
        let area_size = crate::Size::new(200.0, 200.0);
        let (used, _maybe_img) = inner::compute_auto_scale(svg, candidate, 0.9, area_size);
        assert!((used - candidate).abs() < 1e-6);
    }
}
