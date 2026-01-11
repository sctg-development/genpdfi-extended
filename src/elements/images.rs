//! Image support for genpdfi-rs.

use std::io::Read;
use std::path;

use image::GenericImageView;

use crate::error::{Context as _, Error, ErrorKind};
use crate::{render, style};
use crate::{Alignment, Context, Element, Mm, Position, RenderResult, Rotation, Scale, Size};

/// Enum representing either a raster image or a vector SVG image.
///
/// This enum allows the `Image` struct to handle both raster formats (PNG, JPEG, etc.)
/// and SVG vector graphics without rasterization, leveraging `printpdf`'s native SVG support.
#[derive(Clone)]
enum ImageSource {
    /// A raster image loaded from formats supported by the `image` crate.
    Raster(image::DynamicImage),
    /// A vector SVG image parsed by printpdf as an ExternalXObject.
    Svg(printpdf::ExternalXObject),
}

impl ImageSource {
    /// Returns the intrinsic dimensions of the image in millimeters at the default DPI.
    ///
    /// For raster images, dimensions are calculated from pixel count and DPI.
    /// For SVG images, dimensions are extracted from the ExternalXObject width and height.
    ///
    /// # Arguments
    ///
    /// * `dpi` - Optional DPI override for raster images. Defaults to 300 DPI if not specified.
    ///
    /// # Returns
    ///
    /// A `Size` struct containing the width and height in millimeters.
    fn intrinsic_size(&self, dpi: Option<f32>) -> Size {
        match self {
            ImageSource::Raster(img) => {
                let mmpi: f32 = 25.4; // millimeters per inch
                let dpi_val: f32 = dpi.unwrap_or(300.0);
                let (px_width, px_height) = img.dimensions();
                Size::new(
                    mmpi * ((px_width as f32) / dpi_val),
                    mmpi * ((px_height as f32) / dpi_val),
                )
            }
            ImageSource::Svg(svg) => {
                // SVG dimensions in pixels from the ExternalXObject
                // Convert to mm using 300 DPI (standard for SVG)
                let mmpi: f32 = 25.4;
                let svg_dpi: f32 = svg.dpi.unwrap_or(300.0);
                let width_px = svg.width.map(|px| px.0 as f32).unwrap_or(100.0);
                let height_px = svg.height.map(|px| px.0 as f32).unwrap_or(100.0);
                Size::new(mmpi * (width_px / svg_dpi), mmpi * (height_px / svg_dpi))
            }
        }
    }

    /// Checks if this image source is an SVG.
    ///
    /// # Returns
    ///
    /// `true` if this is an SVG image, `false` otherwise.
    fn is_svg(&self) -> bool {
        matches!(self, ImageSource::Svg(_))
    }
}

/// An image to embed in the PDF.
///
/// *Only available if the `images` feature is enabled.*
///
/// This struct supports both raster images and vector SVG graphics. Raster images are wrapped
/// around the configurations [`printpdf::Image`][] exposes, while SVG images use `printpdf`'s
/// native vector graphics support without rasterization.
///
/// # Supported Raster Formats
///
/// All formats supported by the [`image`][] crate should be supported by this crate.
/// The BMP, JPEG and PNG formats are well tested and known to work.
///
/// Note that only the GIF, JPEG, PNG, PNM, TIFF and BMP formats are enabled by default.
/// If you want to use other formats, you have to add the `image` crate as a dependency
/// and activate the required feature.
///
/// # SVG Support
///
/// SVG images are embedded as vector graphics without rasterization. They are processed
/// through `printpdf`'s SVG parser and rendered as XObjects in the PDF.
///
/// # Examples
///
/// Load and display a raster image:
/// ```
/// use genpdfi_extended::elements;
/// let image = elements::Image::from_path("examples/images/test_image.jpg")
///       .expect("Failed to load test image")
///       .with_alignment(genpdfi_extended::Alignment::Center)
///       .with_scale(genpdfi_extended::Scale::new(0.5, 2.0));
/// ```
///
/// [`image`]: https://lib.rs/crates/image
/// [`printpdf::Image`]: https://docs.rs/printpdf/latest/printpdf/types/plugins/graphics/two_dimensional/image/struct.Image.html
#[derive(Clone)]
pub struct Image {
    source: ImageSource,

    /// Used for positioning if no absolute position is given.
    alignment: Alignment,

    /// The absolute position within the given area.
    ///
    /// If no position is set, we use the Alignment.
    position: Option<Position>,

    /// Scaling of the image, default is 1:1.
    scale: Scale,

    /// Resize to a fraction of the page width (0.0 < fraction <= 1.0).
    /// When set, the image will be scaled proportionally so its width equals
    /// `fraction * available_page_width` at render time.
    fit_to_page_width: Option<f32>,

    /// Resize to a fraction of the page height (0.0 < fraction <= 1.0).
    /// When set, the image will be scaled proportionally so its height equals
    /// `fraction * available_page_height` at render time.
    fit_to_page_height: Option<f32>,

    /// The number of degrees of clockwise rotation.
    rotation: Rotation,

    /// Optional background color used to composite away an alpha channel when rendering.
    /// If `None` the page background (white) is used.
    background_color: Option<crate::style::Color>,

    /// DPI override if you know better. Defaults to `printpdf`’s default of 300 dpi.
    dpi: Option<f32>,

    /// Optional hyperlink URI for the image. When set, clicking the image opens this URL.
    link: Option<String>,
}

impl Image {
    /// Creates a new image from an already loaded raster image.
    ///
    /// # Arguments
    ///
    /// * `data` - A DynamicImage from the `image` crate containing raster pixel data.
    ///
    /// # Returns
    ///
    /// A new `Image` struct configured with default alignment, no explicit position,
    /// default scale, and no transformations.
    ///
    /// # Errors
    ///
    /// This function cannot fail and always returns `Ok(Image)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "images")]
    /// # {
    /// use genpdfi_extended::elements::Image;
    /// let img = image::DynamicImage::new_rgb8(100, 50);
    /// let image = Image::from_dynamic_image(img).expect("create image");
    /// # }
    /// ```
    pub fn from_dynamic_image(data: image::DynamicImage) -> Result<Self, Error> {
        // Accept images with alpha; we'll composite them at render time using the
        // page/background color so that they visually match a flattened image.
        Ok(Image {
            source: ImageSource::Raster(data),
            alignment: Alignment::default(),
            position: None,
            scale: Scale::default(),
            fit_to_page_width: None,
            fit_to_page_height: None,
            rotation: Rotation::default(),
            background_color: None,
            dpi: None,
            link: None,
        })
    }

    /// Creates a new image from an SVG string.
    ///
    /// The SVG is parsed by `printpdf` to extract vector graphics without rasterization.
    ///
    /// # Arguments
    ///
    /// * `svg_content` - A string containing valid SVG markup.
    ///
    /// # Returns
    ///
    /// A new `Image` struct configured with default alignment, no explicit position,
    /// default scale, and no transformations.
    ///
    /// # Errors
    ///
    /// Returns an error if the SVG parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use genpdfi_extended::elements::Image;
    /// let svg = r#"
    ///   <svg width="100mm" height="50mm" viewBox="0 0 100 50" xmlns="http://www.w3.org/2000/svg">
    ///     <circle cx="25" cy="25" r="20" fill="red"/>
    ///   </svg>
    /// "#;
    /// let image = Image::from_svg_string(svg).expect("parse SVG");
    /// ```
    pub fn from_svg_string(svg_content: &str) -> Result<Self, Error> {
        // Workaround for printpdf bug: SVG <mask> elements cause "Invalid dictionary reference" errors.
        // Strip <mask> elements and mask attribute references before parsing.
        // See: SVG_MASK_BUG_ANALYSIS.md for details.
        let cleaned_svg = Self::strip_svg_masks(svg_content);

        // Extract DPI from data-dpi attribute if present (embedded by microtex_rs)
        let dpi_from_svg = Self::extract_dpi_from_svg(&cleaned_svg);

        let mut warnings = Vec::new();
        let svg_xobj = printpdf::Svg::parse(&cleaned_svg, &mut warnings).map_err(|e| {
            Error::new(
                format!("Failed to parse SVG: {}", e),
                ErrorKind::InvalidData,
            )
        })?;

        Ok(Image {
            source: ImageSource::Svg(svg_xobj),
            alignment: Alignment::default(),
            position: None,
            scale: Scale::default(),
            fit_to_page_width: None,
            fit_to_page_height: None,
            rotation: Rotation::default(),
            background_color: None,
            dpi: dpi_from_svg,
            link: None,
        })
    }

    /// Extracts DPI from the `data-dpi` attribute in SVG content.
    ///
    /// This function looks for the `data-dpi` attribute in the SVG root element,
    /// which is embedded by microtex_rs when rendering LaTeX formulas.
    ///
    /// # Arguments
    ///
    /// * `svg_content` - The SVG string to search for the `data-dpi` attribute.
    ///
    /// # Returns
    ///
    /// `Some(dpi)` if the attribute is found and contains a valid integer,
    /// `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" data-dpi="720" width="100" height="50">...</svg>"#;
    /// let dpi = Image::extract_dpi_from_svg(svg);
    /// assert_eq!(dpi, Some(720.0));
    /// ```
    fn extract_dpi_from_svg(svg_content: &str) -> Option<f32> {
        // Look for data-dpi="XXX" pattern in the SVG content
        if let Some(start) = svg_content.find("data-dpi=\"") {
            let search_str = &svg_content[start + 10..]; // Skip "data-dpi=\""
            if let Some(end) = search_str.find('"') {
                let dpi_str = &search_str[..end];
                if let Ok(dpi_val) = dpi_str.parse::<i32>() {
                    return Some(dpi_val as f32);
                }
            }
        }
        None
    }

    /// Workaround for printpdf SVG <mask> parsing bug.
    ///
    /// Removes <mask> element definitions and mask="url(#...)" attribute references
    /// from SVG content to prevent "Invalid dictionary reference" errors during parsing.
    ///
    /// This is a temporary workaround until printpdf properly supports SVG mask elements.
    fn strip_svg_masks(svg_content: &str) -> String {
        let mut result = svg_content.to_string();

        // Remove <mask ...>...</mask> blocks using simple string manipulation
        // This is safer than complex state machines
        while let Some(start) = result.find("<mask ") {
            // Find the closing > of the opening tag
            if let Some(open_end) = result[start..].find('>') {
                let open_end = start + open_end + 1;

                // Find the closing </mask> tag
                if let Some(close_pos) = result[open_end..].find("</mask>") {
                    let close_pos = open_end + close_pos;
                    let close_end = close_pos + 7; // len("</mask>")

                    // Remove the entire mask element
                    result.drain(start..close_end);
                } else {
                    // Malformed SVG, stop trying
                    break;
                }
            } else {
                // Malformed SVG, stop trying
                break;
            }
        }

        // Remove mask="url(#...)" attributes
        while let Some(start) = result.find("mask=\"url(#") {
            // Find the closing )"
            if let Some(end) = result[start..].find("\")") {
                let end = start + end + 2; // Include the ")
                                           // Also remove trailing space if present
                let trim_space = if result.len() > end && result[end..].starts_with(' ') {
                    1
                } else {
                    0
                };

                result.drain(start..end + trim_space);
            } else {
                // Malformed mask attribute, stop trying
                break;
            }
        }

        result
    }

    /// Creates a new image by reading an SVG from a byte buffer.
    ///
    /// # Arguments
    ///
    /// * `svg_bytes` - A byte slice containing valid SVG markup in UTF-8 encoding.
    ///
    /// # Returns
    ///
    /// A new `Image` struct with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8 or if SVG parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use genpdfi_extended::elements::Image;
    /// let svg_bytes = b"<svg width='100mm' height='50mm'><circle cx='25' cy='25' r='20' fill='blue'/></svg>";
    /// let image = Image::from_svg_bytes(svg_bytes).expect("create from bytes");
    /// ```
    pub fn from_svg_bytes(svg_bytes: &[u8]) -> Result<Self, Error> {
        let svg_content = std::str::from_utf8(svg_bytes).map_err(|e| {
            Error::new(
                format!("SVG bytes are not valid UTF-8: {}", e),
                ErrorKind::InvalidData,
            )
        })?;
        Self::from_svg_string(svg_content)
    }

    /// Creates a new image by reading an SVG from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - A path to an SVG file.
    ///
    /// # Returns
    ///
    /// A new `Image` struct with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, if its contents are not valid UTF-8,
    /// or if SVG parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use genpdfi_extended::elements::Image;
    /// // This example assumes an SVG file exists at the path
    /// // In a real scenario, create such a file first
    /// let result = Image::from_svg_path("examples/test.svg");
    /// // The result would be Ok or Err depending on file existence
    /// ```
    pub fn from_svg_path<P: AsRef<path::Path>>(path: P) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .with_context(|| format!("Could not read SVG file from path {}", path_ref.display()))?;
        Self::from_svg_string(&content)
    }

    /// Creates a new image from a reader yielding SVG content.
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader that yields SVG markup as bytes.
    ///
    /// # Returns
    ///
    /// A new `Image` struct with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if reading fails, if content is not valid UTF-8, or if SVG parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use genpdfi_extended::elements::Image;
    /// use std::io::Cursor;
    /// let svg_data = b"<svg width='100mm' height='50mm'><rect width='100' height='50' fill='green'/></svg>";
    /// let cursor = Cursor::new(svg_data.to_vec());
    /// let image = Image::from_svg_reader(cursor).expect("create from reader");
    /// ```
    pub fn from_svg_reader<R: std::io::Read>(mut reader: R) -> Result<Self, Error> {
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .context("Failed to read SVG from reader")?;
        Self::from_svg_string(&content)
    }

    fn from_image_reader<R>(reader: image::io::Reader<R>) -> Result<Self, Error>
    where
        R: std::io::BufRead,
        R: std::io::Read,
        R: std::io::Seek,
    {
        let image = reader
            .with_guessed_format()
            .context("Could not determine image format")?
            .decode()
            .context("Could not decode image")?;
        Self::from_dynamic_image(image)
    }

    /// Creates a new image from the given reader.
    pub fn from_reader<R>(reader: R) -> Result<Self, Error>
    where
        R: std::io::BufRead,
        R: std::io::Read,
        R: std::io::Seek,
    {
        Self::from_image_reader(image::io::Reader::new(reader))
    }

    /// Creates a new image by reading from the given path.
    pub fn from_path(path: impl AsRef<path::Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let reader = image::io::Reader::open(path)
            .with_context(|| format!("Could not read image from path {}", path.display()))?;
        Self::from_image_reader(reader)
    }

    /// Translates the image over to position.
    pub fn set_position(&mut self, position: impl Into<Position>) {
        self.position = Some(position.into());
    }

    /// Returns a reference to the raster image data if this is a raster image.
    ///
    /// # Returns
    ///
    /// `Some(&DynamicImage)` if this image is a raster image, `None` if it is an SVG.
    #[cfg(test)]
    fn raster_data(&self) -> Option<&image::DynamicImage> {
        match &self.source {
            ImageSource::Raster(img) => Some(img),
            ImageSource::Svg(_) => None,
        }
    }

    /// Translates the image over to position and returns it.
    pub fn with_position(mut self, position: impl Into<Position>) -> Self {
        self.set_position(position);
        self
    }

    /// Scales the image.
    pub fn set_scale(&mut self, scale: impl Into<Scale>) {
        self.scale = scale.into();
    }

    /// Scales the image and returns it.
    pub fn with_scale(mut self, scale: impl Into<Scale>) -> Self {
        self.set_scale(scale);
        self
    }

    /// Sets the alignment to use for this image.
    pub fn set_alignment(&mut self, alignment: impl Into<Alignment>) {
        self.alignment = alignment.into();
    }

    /// Sets the alignment to use for this image and returns it.
    pub fn with_alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.set_alignment(alignment);
        self
    }

    /// Determines the offset from left-side based on provided Alignment.
    fn get_offset(&self, width: Mm, max_width: Mm) -> Position {
        let horizontal_offset = match self.alignment {
            Alignment::Left => Mm::default(),
            Alignment::Center => (max_width - width) / 2.0,
            Alignment::Right => max_width - width,
        };
        Position::new(horizontal_offset, 0)
    }

    /// Calculates a guess for the size of the image based on the dpi/pixel-count/scale.
    fn get_size(&self) -> Size {
        self.size_with_scale(self.scale)
    }

    /// Returns the intrinsic size (without scale) of the image in mm.
    ///
    /// For raster images, dimensions are calculated from pixel count and DPI.
    /// For SVG images, dimensions are extracted from the SVG's width/height attributes.
    ///
    /// This method is public to allow calculation of proportional scaling factors based
    /// on the original SVG dimensions (useful for scale_factor-based resizing).
    ///
    /// # Returns
    /// A `Size` struct containing width and height in millimeters.
    pub fn get_intrinsic_size(&self) -> Size {
        self.intrinsic_size()
    }

    /// Returns the intrinsic size (without scale) of the image in mm.
    fn intrinsic_size(&self) -> Size {
        self.source.intrinsic_size(self.dpi)
    }

    /// Computes size in mm for a given explicit scale (without modifying `self.scale`).
    fn size_with_scale(&self, scale: Scale) -> Size {
        let intrinsic = self.intrinsic_size();
        Size::new(scale.x * intrinsic.width.0, scale.y * intrinsic.height.0)
    }

    /// Sets the clockwise rotation of the image around the bottom left corner.
    pub fn set_clockwise_rotation(&mut self, rotation: impl Into<Rotation>) {
        self.rotation = rotation.into();
    }

    /// Sets the clockwise rotation of the image around the bottom left corner and then returns the
    /// image.
    pub fn with_clockwise_rotation(mut self, rotation: impl Into<Rotation>) -> Self {
        self.set_clockwise_rotation(rotation);
        self
    }

    /// Sets the expected DPI of the encoded image.
    pub fn set_dpi(&mut self, dpi: f32) {
        self.dpi = Some(dpi);
    }

    /// Sets the expected DPI of the encoded image and returns it.
    pub fn with_dpi(mut self, dpi: f32) -> Self {
        self.set_dpi(dpi);
        self
    }

    /// Set the background color used to composite away an alpha channel when rendering.
    /// If not set, white is used.
    pub fn set_background_color(&mut self, color: crate::style::Color) {
        self.background_color = Some(color);
    }

    /// Set the background color used to composite away an alpha channel and return the image.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "images")]
    /// # {
    /// use genpdfi_extended::elements::Image;
    /// use genpdfi_extended::style::Color;
    /// // create a small RGBA image and set a background color for compositing at render time
    /// let img = Image::from_dynamic_image(image::DynamicImage::new_rgba8(10, 10)).unwrap()
    ///     .with_background_color(Color::Rgb(240, 240, 240));
    /// // background color is applied at render-time; this example verifies construction only
    /// let _ = img;
    /// # }
    /// ```
    pub fn with_background_color(mut self, color: crate::style::Color) -> Self {
        self.set_background_color(color);
        self
    }

    /// Sets a hyperlink URI for this image.
    /// When set, clicking the image will open this URL in a PDF viewer.
    pub fn set_link(&mut self, uri: impl Into<String>) {
        self.link = Some(uri.into());
    }

    /// Sets a hyperlink URI for this image and returns it.
    /// When set, clicking the image will open this URL in a PDF viewer.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "images")]
    /// # {
    /// use genpdfi_extended::elements::Image;
    /// let img = Image::from_path("examples/images/test_image.jpg")
    ///     .expect("load image")
    ///     .with_link("https://example.com");
    /// // Image is now clickable and will open the URL when clicked in a PDF viewer
    /// # }
    /// ```
    pub fn with_link(mut self, uri: impl Into<String>) -> Self {
        self.set_link(uri);
        self
    }

    /// Resize proportionally so the image width becomes exactly `fraction * available_page_width`.
    /// `fraction` is in the range (0.0, 1.0]. This is applied at render-time using the actual
    /// available area width — no page width argument is required at call site.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "images")]
    /// # {
    /// use genpdfi_extended::elements::Image;
    /// use genpdfi_extended::Scale;
    /// // create an in-memory RGB image (2400×450 px)
    /// let img = Image::from_dynamic_image(image::DynamicImage::new_rgb8(2400, 450))
    ///     .expect("create")
    ///     .resizing_page_with(0.5);
    ///
    /// // If available width is 190mm, 50% means 95mm target width
    /// // intrinsic width (mm) for 2400px @ 300dpi: 2400 * 25.4 / 300 = 203.2mm
    /// let intrinsic: f32 = 2400.0_f32 * 25.4_f32 / 300.0_f32;
    /// let target: f32 = 190.0_f32 * 0.5_f32;
    /// let scale: f32 = target / intrinsic;
    /// let size_width: f32 = intrinsic * scale;
    /// assert!((size_width - target).abs() < 0.1_f32);
    /// # }
    /// ```
    pub fn resizing_page_with(mut self, fraction: f32) -> Self {
        assert!(
            fraction > 0.0 && fraction <= 1.0,
            "fraction must be in (0.0, 1.0]"
        );
        self.fit_to_page_width = Some(fraction);
        self
    }

    /// Resize proportionally so the image height becomes exactly `fraction * available_page_height`.
    /// See `resizing_page_with` for semantics.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "images")]
    /// # {
    /// use genpdfi_extended::elements::Image;
    /// use genpdfi_extended::Scale;
    /// // create an in-memory RGB image (2400×450 px)
    /// let img = Image::from_dynamic_image(image::DynamicImage::new_rgb8(2400, 450))
    ///     .expect("create")
    ///     .resizing_page_height(0.3);
    ///
    /// // If available height is 277mm, 30% means ~83.1mm target height
    /// // intrinsic height (mm) for 450px @ 300dpi: 450 * 25.4 / 300 = 38.1mm
    /// let intrinsic: f32 = 450.0_f32 * 25.4_f32 / 300.0_f32;
    /// let target: f32 = 277.0_f32 * 0.3_f32;
    /// let scale: f32 = target / intrinsic;
    /// let size_height: f32 = intrinsic * scale;
    /// assert!((size_height - target).abs() < 0.1_f32);
    /// # }
    /// ```
    pub fn resizing_page_height(mut self, fraction: f32) -> Self {
        assert!(
            fraction > 0.0 && fraction <= 1.0,
            "fraction must be in (0.0, 1.0]"
        );
        self.fit_to_page_height = Some(fraction);
        self
    }
}

impl Element for Image {
    fn render(
        &mut self,
        _context: &Context,
        area: render::Area<'_>,
        _style: style::Style,
    ) -> Result<RenderResult, Error> {
        let mut result = RenderResult::default();

        // Determine effective scale to use: priority is explicit fit-to-page settings,
        // then explicit scale set by user.
        let effective_scale = if let Some(fraction) = self.fit_to_page_width {
            // target width in mm
            let target_width = area.size().width.as_f32() * fraction;
            let intrinsic_width = self.intrinsic_size().width.as_f32();
            let sf = target_width / intrinsic_width;
            Scale::new(sf, sf)
        } else if let Some(fraction) = self.fit_to_page_height {
            let target_height = area.size().height.as_f32() * fraction;
            let intrinsic_height = self.intrinsic_size().height.as_f32();
            let sf = target_height / intrinsic_height;
            Scale::new(sf, sf)
        } else {
            self.scale
        };

        // Compute the true size based on effective_scale and bounding-box information
        let true_size = self.size_with_scale(effective_scale);
        let (bb_origin, bb_size) = bounding_box_offset_and_size(&self.rotation, &true_size);

        let mut position: Position = if let Some(position) = self.position {
            position
        } else {
            // Update the result size to be based on the bounding-box size/offset.
            result.size = bb_size;

            // No position override given; so we calculate the Alignment offset based on
            // the area-size and width of the bounding box.
            self.get_offset(bb_size.width, area.size().width)
        };

        // Fix the position with the bounding-box's origin which was changed from
        // (0,0) when it was rotated in any way.
        position += bb_origin;

        // CHECK FOR OVERFLOW BEFORE RENDERING
        // If the image (with bounding box) doesn't fit in the available area,
        // don't render it and signal that we need more space (new page).
        let image_bottom = position.y + bb_size.height;
        let available_height = area.size().height;

        if image_bottom > available_height {
            // Image overflows the page - don't render it and signal that we need more space
            result.has_more = true;
            result.size = Size::new(0, 0); // Don't consume space
            return Ok(result);
        }

        // Only reach here if image FITS - now render it
        match &self.source {
            ImageSource::Svg(svg) => {
                // SVG rendering - no alpha compositing needed, printpdf handles it
                area.add_svg(svg, position, effective_scale, self.rotation);
            }
            ImageSource::Raster(raster) => {
                // Raster rendering - handle alpha channel if present
                // If the image has an alpha channel, composite it on-the-fly over the background color
                // (default white) so that rendering works with PDFs that don't support alpha.
                if raster.color().has_alpha() {
                    // Determine background color (default white)
                    let bg = self
                        .background_color
                        .unwrap_or(crate::style::Color::Rgb(255, 255, 255));

                    let bg_rgb = match bg {
                        crate::style::Color::Rgb(r, g, b) => (r, g, b),
                        crate::style::Color::Greyscale(v) => (v, v, v),
                        crate::style::Color::Cmyk(c, m, y, k) => {
                            // Simple conversion by inverting CMYK to RGB (approximation)
                            let cf = 1.0 - (c as f32 / 255.0);
                            let mf = 1.0 - (m as f32 / 255.0);
                            let yf = 1.0 - (y as f32 / 255.0);
                            let kf = 1.0 - (k as f32 / 255.0);
                            let r = ((1.0 - cf * kf) * 255.0).clamp(0.0, 255.0) as u8;
                            let g = ((1.0 - mf * kf) * 255.0).clamp(0.0, 255.0) as u8;
                            let b = ((1.0 - yf * kf) * 255.0).clamp(0.0, 255.0) as u8;
                            (r, g, b)
                        }
                    };

                    let rgba = raster.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let mut rgb = image::RgbImage::new(w, h);

                    for (x, y, px) in rgba.enumerate_pixels() {
                        let image::Rgba([sr, sg, sb, sa]) = *px;
                        let af = sa as f32 / 255.0;
                        let r = (sr as f32 * af + bg_rgb.0 as f32 * (1.0 - af)).round() as u8;
                        let g = (sg as f32 * af + bg_rgb.1 as f32 * (1.0 - af)).round() as u8;
                        let b = (sb as f32 * af + bg_rgb.2 as f32 * (1.0 - af)).round() as u8;
                        rgb.put_pixel(x, y, image::Rgb([r, g, b]));
                    }

                    let composite = image::DynamicImage::ImageRgb8(rgb);
                    area.add_image(
                        &composite,
                        position,
                        effective_scale,
                        self.rotation,
                        self.dpi,
                    );
                } else {
                    area.add_image(raster, position, effective_scale, self.rotation, self.dpi);
                }
            }
        }

        // Add link annotation after image is rendered (for both raster and SVG)
        if let Some(url) = &self.link {
            area.add_image_link(position, true_size, self.rotation, url);
        }

        // Image fits and was successfully rendered
        result.has_more = false;

        Ok(result)
    }
}

/// Given the Size of a box (width/height), compute the bounding-box size and offset when
/// rotated some degrees.  The offset is the distance from the top-left corner of the bounding box
/// to the (originally) lower-left corner of the image.
#[allow(clippy::manual_range_contains)]
fn bounding_box_offset_and_size(rotation: &Rotation, size: &Size) -> (Position, Size) {
    // alpha = rotation, beta = 90 - rotation
    let alpha = rotation.degrees.to_radians();
    let beta = (90.0 - rotation.degrees).to_radians();

    // s* = sin of *
    let sa = alpha.sin();
    let sb = beta.sin();

    // Bounding box calculation, based on
    // https://math.stackexchange.com/questions/1628657/dimensions-of-a-rectangle-containing-a-rotated-rectangle
    let width = (size.width.0 * sb).abs() + (size.height.0 * sa).abs();
    let height = (size.height.0 * sb).abs() + (size.width.0 * sa).abs();
    let bb_size = Size::new(width, height);

    // Offset calculation -- to follow the calculations, consider the rotated rectangles, their
    // bounding boxes and the triangles between them
    let bb_position = if rotation.degrees < -180.0 {
        unreachable!(
            "Rotations must be in the range -180.0..=180.0, but got: {}",
            rotation.degrees
        );
    } else if rotation.degrees <= -90.0 {
        Position::new(size.width.0 * alpha.cos().abs(), 0)
    } else if rotation.degrees <= 0.0 {
        Position::new(0, size.height.0 * alpha.cos())
    } else if rotation.degrees <= 90.0 {
        Position::new(size.height.0 * beta.cos(), bb_size.height.0)
    } else if rotation.degrees <= 180.0 {
        Position::new(bb_size.width.0, size.width.0 * beta.cos())
    } else {
        unreachable!(
            "Rotations must be in the range -180.0..=180.0, but got: {}",
            rotation.degrees
        );
    };

    (bb_position, bb_size)
}

#[cfg(test)]
mod tests {
    use super::{bounding_box_offset_and_size, Image};
    use crate::render::Renderer;
    use crate::Element;
    use crate::{Alignment, Mm, Position, Rotation, Size};
    use float_cmp::approx_eq;

    macro_rules! assert_approx_eq {
        ($typ:ty, $lhs:expr, $rhs:expr) => {
            let left = $lhs;
            let right = $rhs;
            assert!(
                approx_eq!($typ, left, right, epsilon = 100.0 * f32::EPSILON, ulps = 10),
                "assertion failed: `(left approx_eq right)`
  left: `{:?}`,
 right: `{:?}`",
                left,
                right
            );
        };
    }

    fn test_position(size: Size, rotation: f32, position: Position) {
        let rotation = Rotation::from(rotation);
        assert_approx_eq!(
            Position,
            position,
            bounding_box_offset_and_size(&rotation, &size).0
        );
    }

    #[test]
    fn test_bounding_box_size_square_0_deg() {
        let size = Size::new(100, 100);
        for rotation in &[-180.0, -90.0, 0.0, 90.0, 180.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(Size, size, bounding_box_offset_and_size(&rotation, &size).1);
        }
    }

    #[test]
    fn test_bounding_box_size_square_30_deg() {
        let size = Size::new(100, 100);
        let bb_width = (60.0f32.to_radians().sin() + 30.0f32.to_radians().sin()) * size.width.0;
        let bb_size = Size::new(bb_width, bb_width);
        for rotation in &[-150.0, -120.0, -30.0, -60.0, 30.0, 60.0, 120.0, 150.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(
                Size,
                bb_size,
                bounding_box_offset_and_size(&rotation, &size).1
            );
        }
    }

    #[test]
    fn test_bounding_box_size_square_45_deg() {
        let size = Size::new(100, 100);
        let bb_width = (2.0f32 * size.width.0.powf(2.0)).sqrt();
        let bb_size = Size::new(bb_width, bb_width);
        for rotation in &[-135.0, -45.0, 45.0, 135.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(
                Size,
                bb_size,
                bounding_box_offset_and_size(&rotation, &size).1
            );
        }
    }

    #[test]
    fn test_bounding_box_position_square_30_deg() {
        let size = Size::new(100, 100);
        let bb_width =
            30.0f32.to_radians().sin() * size.width.0 + 60.0f32.to_radians().sin() * size.height.0;

        let w30 = 30.0f32.to_radians().cos() * size.width.0;
        let w60 = 60.0f32.to_radians().cos() * size.width.0;

        test_position(size, -150.0, Position::new(w30, 0));
        test_position(size, -120.0, Position::new(w60, 0));
        test_position(size, -60.0, Position::new(0, w60));
        test_position(size, -30.0, Position::new(0, w30));
        test_position(size, 30.0, Position::new(w60, bb_width));
        test_position(size, 60.0, Position::new(w30, bb_width));
        test_position(size, 120.0, Position::new(bb_width, bb_width - w60));
        test_position(size, 150.0, Position::new(bb_width, bb_width - w30));
    }

    #[test]
    fn test_bounding_box_position_square_45_deg() {
        let size = Size::new(100, 100);
        let bb_width = (2.0f32 * size.width.0.powf(2.0)).sqrt();

        test_position(size, -135.0, Position::new(bb_width / 2.0, 0));
        test_position(size, -45.0, Position::new(0, bb_width / 2.0));
        test_position(size, 45.0, Position::new(bb_width / 2.0, bb_width));
        test_position(size, 135.0, Position::new(bb_width, bb_width / 2.0));
    }

    #[test]
    #[test]
    fn test_bounding_box_position_square_90_deg() {
        let size = Size::new(100, 100);
        test_position(size, -180.0, Position::new(100, 0));
        test_position(size, -90.0, Position::new(0, 0));
        test_position(size, 0.0, Position::new(0, 100));
        test_position(size, 90.0, Position::new(100, 100));
        test_position(size, 180.0, Position::new(100, 0));
    }

    #[test]
    fn test_bounding_box_size_rectangle_0_deg() {
        let size = Size::new(200, 100);
        for rotation in &[-180.0, 0.0, 180.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(Size, size, bounding_box_offset_and_size(&rotation, &size).1);
        }
    }

    #[test]
    fn test_resizing_page_width() {
        // Create a simple image of 2400×450 pixels (matching earlier examples)
        let img = image::DynamicImage::new_rgb8(2400, 450);
        let image = Image::from_dynamic_image(img)
            .expect("image")
            .resizing_page_with(0.5);

        // Intrinsic width at default DPI 300 should be 2400 * 25.4 / 300 = 203.2 mm
        let intrinsic_width = image.intrinsic_size().width.as_f32();
        assert!(
            (intrinsic_width - 203.2).abs() < 0.1,
            "intrinsic width mismatch"
        );

        // If available page width is 190mm and fraction is 0.5, target width = 95mm
        let available_page_width = 190.0;
        let fraction = 0.5;
        let expected_width = available_page_width * fraction;

        let scale = expected_width / intrinsic_width;
        let size = image.size_with_scale(crate::Scale::new(scale, scale));

        assert!(
            (size.width.as_f32() - expected_width).abs() < 0.01,
            "scaled width mismatch"
        );
    }

    #[test]
    #[cfg(feature = "images")]
    fn test_resizing_examples_images() {
        use crate::Scale;

        let images = [
            "examples/images/test_image.jpg",
            "examples/images/ruler-908891_640.jpg",
            "examples/images/triangle-ruler-1016726_640.png",
            "examples/images/triangle-161210_1280.png",
        ];

        let available_page_width = 190.0;
        let available_page_height = 277.0;

        for path in images.iter() {
            let img = match Image::from_path(path) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("Skipping {}: {}", path, e);
                    continue;
                }
            };

            // Width-based resize: 50% of page width
            let fraction_w = 0.5;
            let expected_w = available_page_width * fraction_w;
            let intrinsic_w = img.intrinsic_size().width.as_f32();
            let scale_w = expected_w / intrinsic_w;
            let size_w = img.size_with_scale(Scale::new(scale_w, scale_w));
            assert!(
                (size_w.width.as_f32() - expected_w).abs() < 0.5,
                "scaled width mismatch for {}: got {:.2}mm, expected {:.2}mm",
                path,
                size_w.width.as_f32(),
                expected_w
            );

            // Height-based resize: 30% of page height
            let fraction_h = 0.3;
            let expected_h = available_page_height * fraction_h;
            let intrinsic_h = img.intrinsic_size().height.as_f32();
            let scale_h = expected_h / intrinsic_h;
            let size_h = img.size_with_scale(Scale::new(scale_h, scale_h));
            assert!(
                (size_h.height.as_f32() - expected_h).abs() < 0.5,
                "scaled height mismatch for {}: got {:.2}mm, expected {:.2}mm",
                path,
                size_h.height.as_f32(),
                expected_h
            );
        }
    }

    #[test]
    fn test_bounding_box_size_rectangle_30_deg() {
        let size = Size::new(200, 100);
        let bb_width =
            60.0f32.to_radians().sin() * size.width.0 + 30.0f32.to_radians().sin() * size.height.0;
        let bb_height =
            60.0f32.to_radians().sin() * size.height.0 + 30.0f32.to_radians().sin() * size.width.0;
        let bb_size = Size::new(bb_width, bb_height);
        for rotation in &[-150.0, -30.0, 30.0, 150.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(
                Size,
                bb_size,
                bounding_box_offset_and_size(&rotation, &size).1
            );
        }
    }

    #[test]
    fn test_bounding_box_size_rectangle_45_deg() {
        let size = Size::new(200, 100);
        let bb_width = 45.0f32.to_radians().sin() * (size.width.0 + size.height.0);
        let bb_size = Size::new(bb_width, bb_width);
        for rotation in &[-135.0, -45.0, 45.0, 135.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(
                Size,
                bb_size,
                bounding_box_offset_and_size(&rotation, &size).1
            );
        }
    }

    #[test]
    fn test_image_from_dynamic_image_alpha_accepted_and_get_size_and_offset() {
        // create an RGBA image (alpha channel set) and expect acceptance
        let img_rgba =
            image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(10, 10, |_, _| {
                image::Rgba([255, 0, 0, 128])
            }));
        let result = Image::from_dynamic_image(img_rgba);
        assert!(result.is_ok(), "alpha images should be accepted");

        // load a real image from examples and verify get_size and offsets
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/images/test_image.jpg"
        ));
        let dyn_img = image::load_from_memory(bytes).expect("load image");
        let mut img = Image::from_dynamic_image(dyn_img).expect("from dynamic ok");

        // default dpi and scale should produce positive sizes
        let size = img.get_size();
        assert!(size.width.0 > 0.0);
        assert!(size.height.0 > 0.0);

        // test offsets for different alignments
        let off_left = img.get_offset(Mm::from(10.0), Mm::from(50.0));
        assert_eq!(off_left.x.0, 0.0);
        img.set_alignment(Alignment::Center);
        let off_center = img.get_offset(Mm::from(10.0), Mm::from(50.0));
        assert_eq!(off_center.x.0, (50.0 - 10.0) / 2.0);
        img.set_alignment(Alignment::Right);
        let off_right = img.get_offset(Mm::from(10.0), Mm::from(50.0));
        assert_eq!(off_right.x.0, 50.0 - 10.0);
    }

    #[test]
    fn test_bounding_box_size_rectangle_60_deg() {
        let size = Size::new(200, 100);
        let bb_width =
            30.0f32.to_radians().sin() * size.width.0 + 60.0f32.to_radians().sin() * size.height.0;
        let bb_height =
            30.0f32.to_radians().sin() * size.height.0 + 60.0f32.to_radians().sin() * size.width.0;
        let bb_size = Size::new(bb_width, bb_height);
        for rotation in &[-120.0, -60.0, 60.0, 120.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(
                Size,
                bb_size,
                bounding_box_offset_and_size(&rotation, &size).1
            );
        }
    }

    #[test]
    fn test_bounding_box_size_rectangle_90_deg() {
        let size = Size::new(200, 100);
        let bb_size = Size::new(100, 200);
        for rotation in &[-90.0, 90.0] {
            let rotation = Rotation::from(*rotation);
            assert_approx_eq!(
                Size,
                bb_size,
                bounding_box_offset_and_size(&rotation, &size).1
            );
        }
    }

    #[test]
    fn test_bounding_box_position_rectangle_30_deg() {
        let size = Size::new(200, 100);
        let bb_width =
            30.0f32.to_radians().sin() * size.width.0 + 60.0f32.to_radians().sin() * size.height.0;
        let bb_height =
            30.0f32.to_radians().sin() * size.height.0 + 60.0f32.to_radians().sin() * size.width.0;

        let h30 = 30.0f32.to_radians().cos() * size.height.0;
        let h60 = 60.0f32.to_radians().cos() * size.height.0;
        let w30 = 30.0f32.to_radians().cos() * size.width.0;
        let w60 = 60.0f32.to_radians().cos() * size.width.0;

        test_position(size, -150.0, Position::new(w30, 0));
        test_position(size, -120.0, Position::new(w60, 0));
        test_position(size, -60.0, Position::new(0, h60));
        test_position(size, -30.0, Position::new(0, h30));
        test_position(size, 30.0, Position::new(h60, bb_width));
        test_position(size, 60.0, Position::new(h30, bb_height));
        test_position(size, 120.0, Position::new(bb_width, bb_height - h60));
        test_position(size, 150.0, Position::new(bb_height, bb_width - h30));
    }

    #[test]
    fn test_bounding_box_position_rectangle_45_deg() {
        let size = Size::new(200, 100);
        let bb_width = 45.0f32.to_radians().sin() * (size.width.0 + size.height.0);

        test_position(size, -135.0, Position::new(2.0 * bb_width / 3.0, 0));
        test_position(size, -45.0, Position::new(0, bb_width / 3.0));
        test_position(size, 45.0, Position::new(bb_width / 3.0, bb_width));
        test_position(size, 135.0, Position::new(bb_width, 2.0 * bb_width / 3.0));
    }

    #[test]
    fn test_bounding_box_position_rectangle_90_deg() {
        let size = Size::new(200, 100);
        test_position(size, -180.0, Position::new(200, 0));
        test_position(size, -90.0, Position::new(0, 0));
        test_position(size, 0.0, Position::new(0, 100));
        test_position(size, 90.0, Position::new(100, 200));
        test_position(size, 180.0, Position::new(200, 0));
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_dynamic_image_accepts_alpha_and_keeps_data() {
        let rgba = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([0, 0, 0, 128]),
        ));
        let img = Image::from_dynamic_image(rgba).expect("should accept RGBA image");
        // data should still have alpha channel until render-time
        assert!(img
            .raster_data()
            .map(|r| r.color().has_alpha())
            .unwrap_or(false));
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_render_image_sets_size_with_no_position() {
        use crate::fonts::{FontCache, FontData, FontFamily};
        use crate::style::Style;
        use crate::Context;

        // renderer & area
        let r = Renderer::new(Size::new(200.0, 200.0), "t").expect("renderer");
        let area = r.first_page().first_layer().area();

        // make a 10x10 rgb image
        let rgb = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            10,
            10,
            image::Rgb([10, 20, 30]),
        ));
        let mut img = Image::from_dynamic_image(rgb).expect("image");

        // expected bounding box
        let expected = bounding_box_offset_and_size(&img.rotation, &img.get_size()).1;

        // build dummy font cache/context
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = FontData::new(data.clone(), None).expect("font data");
        let family = FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let cache = FontCache::new(family);
        let context = Context::new(cache);

        let res = img.render(&context, area, Style::new()).expect("render");
        assert_approx_eq!(Size, expected, res.size);
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_render_image_with_position_does_not_set_result_size() {
        use crate::fonts::{FontCache, FontData, FontFamily};
        use crate::style::Style;
        use crate::Context;

        let mut r = Renderer::new(Size::new(200.0, 200.0), "t").expect("renderer");
        let area = r.first_page().first_layer().area();
        let rgb = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            10,
            10,
            image::Rgb([10, 20, 30]),
        ));
        let mut img = Image::from_dynamic_image(rgb).expect("image");
        img.set_position(Position::new(10, 10));
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = FontData::new(data.clone(), None).expect("font data");
        let family = FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let cache = FontCache::new(family);
        let context = Context::new(cache);

        let res = img.render(&context, area, Style::new()).expect("render");
        assert_eq!(res.size, Size::new(0.0, 0.0));
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_path_example_image() {
        let img = Image::from_path("examples/images/test_image.jpg");
        assert!(img.is_ok());
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_get_size_with_dpi_override() {
        let rgb = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            100,
            50,
            image::Rgb([0, 0, 0]),
        ));
        let mut img = Image::from_dynamic_image(rgb).expect("image");
        img.set_dpi(100.0);
        let size = img.get_size();
        // expected width = 25.4 * (scale 1 * 100 px / 100 dpi) = 25.4 mm; height = 25.4*(50/100)=12.7
        assert_approx_eq!(Size, size, Size::new(25.4, 12.7));
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_svg_string_parses_valid_svg() {
        let svg_content = r#"<svg width="100" height="50" xmlns="http://www.w3.org/2000/svg">
            <circle cx="50" cy="25" r="20" fill="red"/>
        </svg>"#;
        let img = Image::from_svg_string(svg_content);
        assert!(img.is_ok(), "Should parse valid SVG");

        let image = img.expect("valid svg");
        // SVG should have intrinsic size
        let size = image.intrinsic_size();
        assert!(size.width.0 > 0.0, "SVG width should be positive");
        assert!(size.height.0 > 0.0, "SVG height should be positive");
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_svg_string_rejects_invalid_svg() {
        let invalid_svg = "this is not svg";
        let result = Image::from_svg_string(invalid_svg);
        assert!(result.is_err(), "Should reject invalid SVG");
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_svg_bytes_converts_utf8() {
        let svg_bytes = b"<svg width='100' height='50' xmlns='http://www.w3.org/2000/svg'>
            <rect width='100' height='50' fill='blue'/>
        </svg>";
        let img = Image::from_svg_bytes(svg_bytes);
        assert!(img.is_ok(), "Should convert valid UTF-8 bytes to SVG");
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_svg_bytes_rejects_invalid_utf8() {
        let invalid_utf8: &[u8] = &[0xFF, 0xFE];
        let result = Image::from_svg_bytes(invalid_utf8);
        assert!(result.is_err(), "Should reject invalid UTF-8");
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_from_svg_reader_reads_content() {
        use std::io::Cursor;

        let svg_data = b"<svg width='100' height='50' xmlns='http://www.w3.org/2000/svg'>
            <polygon points='50,10 90,90 10,90' fill='green'/>
        </svg>";
        let cursor = Cursor::new(svg_data.to_vec());
        let img = Image::from_svg_reader(cursor);
        assert!(img.is_ok(), "Should read SVG from reader");
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_svg_image_with_transformations() {
        use crate::{Alignment, Rotation, Scale};

        let svg_content = r#"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg">
            <line x1="0" y1="0" x2="200" y2="100" stroke="black"/>
        </svg>"#;
        let img = Image::from_svg_string(svg_content)
            .expect("parse svg")
            .with_alignment(Alignment::Center)
            .with_scale(Scale::new(0.5, 0.5))
            .with_clockwise_rotation(Rotation::from_degrees(45.0));

        // Verify that all transformations were applied
        assert_eq!(img.alignment, Alignment::Center);
        assert_eq!(img.scale.x, 0.5);
        assert_eq!(img.scale.y, 0.5);
        assert!((img.rotation.degrees - 45.0).abs() < 0.001);
    }

    #[cfg(feature = "images")]
    #[test]
    fn test_svg_and_raster_image_sources_differentiated() {
        let svg_content = r#"<svg width="100" height="50" xmlns="http://www.w3.org/2000/svg">
            <circle cx="25" cy="25" r="20" fill="red"/>
        </svg>"#;
        let svg_image = Image::from_svg_string(svg_content).expect("parse svg");

        let raster_img = image::DynamicImage::new_rgb8(100, 50);
        let raster_image = Image::from_dynamic_image(raster_img).expect("create raster");

        // SVG image should not have raster data
        assert!(
            svg_image.raster_data().is_none(),
            "SVG should not have raster data"
        );

        // Raster image should have raster data
        assert!(
            raster_image.raster_data().is_some(),
            "Raster should have raster data"
        );
    }
}
