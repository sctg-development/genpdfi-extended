//! LaTeX formula support for genpdfi-rs.
//!
//! This module provides the `Latex` element which allows rendering LaTeX formulas
//! into PDF documents using MicroTeX.
//!
//! Only available if the `latex` feature is enabled.

use crate::error::{Error, ErrorKind};
use crate::{render, style, Alignment, Context, Element, Position, RenderResult};
use std::sync::OnceLock;

/// Helper constants for LaTeX rendering at 720 DPI
const MICROTEX_DPI: i32 = 720;
const EMPIRICAL_ADJUSTMENT_FACTOR: f32 = 4.5;

/// Global MicroTeX renderer instance - initialized only once
static MICROTEX_RENDERER: OnceLock<microtex_rs::MicroTex> = OnceLock::new();

/// Get or initialize the MicroTeX renderer (thread-safe singleton).
/// MicroTeX must only be initialized once - multiple initializations crash the engine.
fn get_microtex_renderer() -> Result<&'static microtex_rs::MicroTex, Error> {
    if let Some(renderer) = MICROTEX_RENDERER.get() {
        return Ok(renderer);
    }

    match microtex_rs::MicroTex::new() {
        Ok(renderer) => {
            let _ = MICROTEX_RENDERER.set(renderer);
            Ok(MICROTEX_RENDERER.get().unwrap())
        }
        Err(_) => Err(Error::new(
            "Failed to initialize MicroTeX renderer (CRITICAL: Can only initialize once)",
            ErrorKind::Internal,
        )),
    }
}

/// A LaTeX formula element that renders mathematical expressions using MicroTeX.
///
/// # Examples
///
/// Basic usage:
/// ```
/// # #[cfg(feature = "latex")]
/// # {
/// use genpdfi_extended::elements;
///
/// let formula = elements::Latex::new(r#"E = mc^2"#, 12.0);
/// # }
/// ```
///
/// With alignment:
/// ```
/// # #[cfg(feature = "latex")]
/// # {
/// use genpdfi_extended::elements;
/// use genpdfi_extended::Alignment;
///
/// let formula = elements::Latex::new(r#"\[E = mc^2\]"#, 14.0)
///     .with_alignment(Alignment::Center);
/// # }
/// ```
#[derive(Clone)]
pub struct Latex {
    /// The LaTeX formula source code
    formula: String,
    /// Font size in "pseudo" points (equivalent to text font size)
    size_pt: f32,
    /// Optional explicit position (overrides alignment)
    position: Option<Position>,
    /// Horizontal alignment when not positioned explicitly
    alignment: Alignment,
    /// Whether to render inline (within text flow) or as a block
    inline: bool,
}

impl Latex {
    /// Creates a new LaTeX formula element.
    ///
    /// # Arguments
    ///
    /// * `formula` - The LaTeX source code to render
    /// * `size_pt` - Font size in "pseudo" points (e.g., 12.0 for 12pt)
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "latex")]
    /// # {
    /// use genpdfi_extended::elements;
    /// let formula = elements::Latex::new(r#"x^2 + y^2 = z^2"#, 14.0);
    /// # }
    /// ```
    pub fn new(formula: impl Into<String>, size_pt: f32) -> Self {
        Self {
            formula: formula.into(),
            size_pt,
            position: None,
            alignment: Alignment::Left,
            inline: false,
        }
    }

    /// Sets explicit positioning, overriding alignment.
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets horizontal alignment (used when position is not explicitly set).
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Marks this formula for inline rendering (integrated into text flow).
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// Marks this formula for block rendering (on its own line).
    pub fn block(mut self) -> Self {
        self.inline = false;
        self
    }

    /// Renders the LaTeX formula to SVG using MicroTeX and applies scaling.
    /// Uses a global singleton MicroTeX instance (initialized only once).
    fn render_to_scaled_svg(&self) -> Result<String, Error> {
        // Get or initialize the global MicroTeX renderer (only happens once)
        let renderer = get_microtex_renderer()?;

        let config = microtex_rs::RenderConfig {
            dpi: MICROTEX_DPI,
            line_width: 20.0,
            line_height: 20.0 / 3.0,
            text_color: 0xff000000,
            has_background: false,
            render_glyph_use_path: true,
            ..Default::default()
        };

        // Render reference formula "m" to calculate scale factor
        let reference_svg = renderer.render("m", &config).map_err(|_| {
            Error::new(
                "Failed to render reference formula 'm'",
                ErrorKind::Internal,
            )
        })?;

        let (_ref_width_px, ref_height_px) = extract_svg_dimensions(&reference_svg)?;

        // Target height in pixels at 720 DPI
        let target_height_px = self.size_pt * 10.0;

        // Calculate scale factor with empirical adjustment
        let mut scale_factor = target_height_px / ref_height_px;
        scale_factor = scale_factor / EMPIRICAL_ADJUSTMENT_FACTOR;

        // Render the actual formula
        let mut svg = renderer.render(&self.formula, &config).map_err(|_| {
            Error::new(
                format!("Failed to render LaTeX formula: {}", self.formula),
                ErrorKind::Internal,
            )
        })?;

        // Apply scaling
        svg = apply_svg_scale(&svg, scale_factor)?;

        Ok(svg)
    }
}

impl Element for Latex {
    fn render(
        &mut self,
        context: &Context,
        area: render::Area<'_>,
        style: style::Style,
    ) -> Result<RenderResult, Error> {
        // Render to scaled SVG
        let scaled_svg = self.render_to_scaled_svg()?;

        // Create an Image element from the SVG
        let mut image = super::Image::from_svg_string(&scaled_svg).map_err(|_| {
            Error::new(
                "Failed to convert LaTeX formula SVG to image",
                ErrorKind::Internal,
            )
        })?;

        // Apply positioning
        if let Some(pos) = self.position {
            image = image.with_position(pos);
        } else {
            image = image.with_alignment(self.alignment);
        }

        // Render the image
        image.render(context, area, style)
    }
}

/// Extracts width and height from SVG attributes in pixels.
fn extract_svg_dimensions(svg: &str) -> Result<(f32, f32), Error> {
    let mut width = None;
    let mut height = None;

    // Parse width attribute
    if let Some(width_start) = svg.find("width=\"") {
        let width_attr_start = width_start + 7;
        if let Some(width_end) = svg[width_attr_start..].find("\"") {
            let width_str = &svg[width_attr_start..width_attr_start + width_end];
            width = width_str.parse::<f32>().ok();
        }
    }

    // Parse height attribute
    if let Some(height_start) = svg.find("height=\"") {
        let height_attr_start = height_start + 8;
        if let Some(height_end) = svg[height_attr_start..].find("\"") {
            let height_str = &svg[height_attr_start..height_attr_start + height_end];
            height = height_str.parse::<f32>().ok();
        }
    }

    let width =
        width.ok_or_else(|| Error::new("Could not extract width from SVG", ErrorKind::Internal))?;
    let height = height
        .ok_or_else(|| Error::new("Could not extract height from SVG", ErrorKind::Internal))?;

    Ok((width, height))
}

/// Applies a scale factor to SVG dimensions.
fn apply_svg_scale(svg: &str, scale_factor: f32) -> Result<String, Error> {
    let (width_px, height_px) = extract_svg_dimensions(svg)?;

    let new_width = (width_px * scale_factor).ceil() as i32;
    let new_height = (height_px * scale_factor).ceil() as i32;

    let mut result = svg.to_string();

    // Replace width attribute
    let width_pattern = format!("width=\"{}\"", width_px as i32);
    let new_width_attr = format!("width=\"{}\"", new_width);
    result = result.replacen(&width_pattern, &new_width_attr, 1);

    // Replace height attribute
    let height_pattern = format!("height=\"{}\"", height_px as i32);
    let new_height_attr = format!("height=\"{}\"", new_height);
    result = result.replacen(&height_pattern, &new_height_attr, 1);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latex_new() {
        let latex = Latex::new(r#"E = mc^2"#, 12.0);
        assert_eq!(latex.size_pt, 12.0);
        assert_eq!(latex.formula, r#"E = mc^2"#);
        assert!(!latex.inline);
    }

    #[test]
    fn test_latex_with_alignment() {
        let latex = Latex::new(r#"x^2 + y^2"#, 14.0).with_alignment(Alignment::Center);
        assert_eq!(latex.alignment, Alignment::Center);
    }

    #[test]
    fn test_latex_inline() {
        let latex = Latex::new(r#"a + b"#, 10.0).inline();
        assert!(latex.inline);
    }

    #[test]
    fn test_latex_block() {
        let latex = Latex::new(r#"a + b"#, 10.0).inline().block();
        assert!(!latex.inline);
    }
}
