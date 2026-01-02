//! Low-level PDF rendering utilities.
//!
//! This module provides low-level abstractions over [`printpdf`][]:  A [`Renderer`][] creates a
//! document with one or more pages with different sizes.  A [`Page`][] has one or more layers, all
//! of the same size.  A [`Layer`][] can be used to access its [`Area`][].
//!
//! An [`Area`][] is a view on a full layer or on a part of a layer.  It can be used to print
//! lines and text.  For more advanced text formatting, you can create a [`TextSection`][] from an
//! [`Area`][].
//!
//! [`printpdf`]: https://docs.rs/printpdf/latest/printpdf
//! [`Renderer`]: struct.Renderer.html
//! [`Page`]: struct.Page.html
//! [`Layer`]: struct.Layer.html
//! [`Area`]: struct.Area.html
//! [`TextSection`]: struct.TextSection.html

use std::cell;
use std::io;
use std::ops;
use std::rc;

use crate::error::{Context as _, Error, ErrorKind};
use crate::fonts;
use crate::style::{Color, LineStyle, Style};
use crate::{Margins, Mm, Position, Size};

/// Compatibility wrapper for a font reference (either builtin or external) to adapt to
/// `printpdf` 0.8 which uses `FontId` for external fonts and `BuiltinFont` for builtin ones.
#[derive(Clone, Debug, PartialEq)]
pub enum IndirectFontRef {
    Builtin(printpdf::BuiltinFont),
    External(printpdf::FontId),
}

impl IndirectFontRef {
    pub fn into_op_font(&self) -> Result<printpdf::FontId, ()> {
        match self {
            IndirectFontRef::External(id) => Ok(id.clone()),
            IndirectFontRef::Builtin(_) => Err(()),
        }
    }
}

#[cfg(feature = "images")]
use crate::{Rotation, Scale};

/// A position relative to the top left corner of a layer.
struct LayerPosition(Position);

impl LayerPosition {
    pub fn from_area(area: &Area<'_>, position: Position) -> Self {
        Self(position + area.origin)
    }
}

/// A position relative to the bottom left corner of a layer (“user space” in PDF terms).
struct UserSpacePosition(Position);

impl UserSpacePosition {
    pub fn from_layer(layer: &Layer<'_>, position: LayerPosition) -> Self {
        Self(Position::new(
            position.0.x,
            layer.page.size.height - position.0.y,
        ))
    }
}

impl From<UserSpacePosition> for printpdf::Point {
    fn from(pos: UserSpacePosition) -> printpdf::Point {
        printpdf::Point::new(pos.0.x.into(), pos.0.y.into())
    }
}

impl ops::Deref for UserSpacePosition {
    type Target = Position;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Renders a PDF document with one or more pages.
///
/// This is a wrapper around a [`printpdf::PdfDocumentReference`][].
///
/// [`printpdf::PdfDocumentReference`]: https://docs.rs/printpdf/0.3.2/printpdf/types/pdf_document/struct.PdfDocumentReference.html
pub struct Renderer {
    doc: printpdf::PdfDocument,
    // optional settings that will be applied when saving
    conformance: Option<printpdf::PdfConformance>,
    creation_date: Option<printpdf::OffsetDateTime>,
    modification_date: Option<printpdf::OffsetDateTime>,
    // invariant: pages.len() >= 1
    pages: Vec<Page>,
}

impl Renderer {
    /// Creates a new PDF document renderer with one page of the given size and the given title.
    ///
    /// # Example
    /// ```
    /// use genpdfi_extended::render::Renderer;
    /// use genpdfi_extended::Size;
    /// use genpdfi_extended::Mm;
    ///
    /// let mut r = Renderer::new(Size::new(210.0, 297.0), "title").expect("renderer");
    /// assert_eq!(r.page_count(), 1);
    /// r.add_page(Size::new(100.0, 100.0));
    /// assert!(r.page_count() >= 2);
    /// let page = r.get_page(0).unwrap();
    /// let layer = page.first_layer();
    /// let area = layer.area();
    /// assert!(area.size().width > Mm::from(0.0));
    /// ```
    pub fn new(size: impl Into<Size>, title: impl AsRef<str>) -> Result<Renderer, Error> {
        let size = size.into();
        let mut doc = printpdf::PdfDocument::new(title.as_ref());
        // create initial layer and page
        let layer = printpdf::Layer::new("Layer 1");
        let layer_id = doc.add_layer(&layer);
        let ops = vec![
            printpdf::Op::BeginLayer {
                layer_id: layer_id.clone(),
            },
            printpdf::Op::EndLayer {
                layer_id: layer_id.clone(),
            },
        ];
        let page = printpdf::PdfPage::new(size.width.into(), size.height.into(), ops);
        doc.pages.push(page);

        let page_ref = doc.pages.len() - 1;
        let page = Page::new(page_ref, layer_id, size);

        Ok(Renderer {
            doc,
            conformance: None,
            creation_date: None,
            modification_date: None,
            pages: vec![page],
        })
    }

    /// Sets the PDF conformance for the generated PDF document.
    pub fn with_conformance(mut self, conformance: printpdf::PdfConformance) -> Self {
        self.conformance = Some(conformance);
        self
    }

    /// Sets the creation date for the generated PDF document.
    pub fn with_creation_date(mut self, date: printpdf::OffsetDateTime) -> Self {
        self.creation_date = Some(date);
        self
    }

    /// Sets the modification date for the generated PDF document.
    pub fn with_modification_date(mut self, date: printpdf::OffsetDateTime) -> Self {
        self.modification_date = Some(date);
        self
    }

    /// Adds a new page with the given size to the document.
    pub fn add_page(&mut self, size: impl Into<Size>) {
        let size = size.into();
        let layer = printpdf::Layer::new("Layer 1");
        let layer_id = self.doc.add_layer(&layer);
        let ops = vec![
            printpdf::Op::BeginLayer {
                layer_id: layer_id.clone(),
            },
            printpdf::Op::EndLayer {
                layer_id: layer_id.clone(),
            },
        ];
        let page = printpdf::PdfPage::new(size.width.into(), size.height.into(), ops);
        self.doc.pages.push(page);
        let page_idx = self.doc.pages.len() - 1;
        self.pages.push(Page::new(page_idx, layer_id, size))
    }

    /// Returns the number of pages in this document.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns a page of this document.
    pub fn get_page(&self, idx: usize) -> Option<&Page> {
        self.pages.get(idx)
    }

    /// Returns a mutable reference to a page of this document.
    pub fn get_page_mut(&mut self, idx: usize) -> Option<&mut Page> {
        self.pages.get_mut(idx)
    }

    /// Returns a mutable reference to the first page of this document.
    pub fn first_page(&self) -> &Page {
        &self.pages[0]
    }

    /// Returns the first page of this document.
    pub fn first_page_mut(&mut self) -> &mut Page {
        &mut self.pages[0]
    }

    /// Returns the last page of this document.
    pub fn last_page(&self) -> &Page {
        &self.pages[self.pages.len() - 1]
    }

    /// Returns a mutable reference to the last page of this document.
    pub fn last_page_mut(&mut self) -> &mut Page {
        let idx = self.pages.len() - 1;
        &mut self.pages[idx]
    }

    /// Loads the builtin font and returns a reference to it.
    pub fn add_builtin_font(
        &self,
        builtin: printpdf::BuiltinFont,
    ) -> Result<IndirectFontRef, Error> {
        // builtins are represented directly
        Ok(IndirectFontRef::Builtin(builtin))
    }

    /// Loads an embedded font from the given data and returns a reference to it.
    pub fn add_embedded_font(&mut self, data: &[u8]) -> Result<IndirectFontRef, Error> {
        let mut warnings = Vec::new();
        let parsed = printpdf::ParsedFont::from_bytes(data, 0, &mut warnings)
            .ok_or_else(|| Error::new("Failed to parse font data", ErrorKind::InvalidFont))?;
        let id = self.doc.add_font(&parsed);
        Ok(IndirectFontRef::External(id))
    }

    /// Writes this PDF document to a writer.
    pub fn write(mut self, w: impl io::Write) -> Result<(), Error> {
        // Assemble pages from our internal representation into the PDF document
        for page in &self.pages {
            let page_idx = page.page_idx;
            let mut new_ops: Vec<printpdf::Op> = Vec::new();
            // borrow layers
            let layers_vec = page.layers.0.borrow();
            for layer_rc in layers_vec.iter() {
                let mut layer = layer_rc.borrow_mut();
                // register layer object in document resources if present
                if let Some(layer_obj) = layer.layer_obj.take() {
                    let id = self.doc.add_layer(&layer_obj);
                    layer.layer_id = id;
                }
                new_ops.push(printpdf::Op::BeginLayer {
                    layer_id: layer.layer_id.clone(),
                });
                new_ops.extend(layer.ops.clone());
                new_ops.push(printpdf::Op::EndLayer {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if page_idx < self.doc.pages.len() {
                self.doc.pages[page_idx].ops = new_ops;
            } else {
                // fallback: push a new page
                let pdf_page = printpdf::PdfPage::new(
                    page.size.width.into(),
                    page.size.height.into(),
                    new_ops,
                );
                self.doc.pages.push(pdf_page);
            }
        }

        let mut warnings = Vec::new();
        let opts = printpdf::serialize::PdfSaveOptions::default();
        // apply conformance, creation and modification date if requested
        if let Some(conf) = self.conformance {
            self.doc.metadata.info.conformance = conf;
        }
        if let Some(date) = self.creation_date {
            self.doc.metadata.info.creation_date = date;
        }
        if let Some(date) = self.modification_date {
            self.doc.metadata.info.modification_date = date;
        }

        // write to buffer
        let mut buf = io::BufWriter::new(w);
        self.doc.save_writer(&mut buf, &opts, &mut warnings);
        Ok(())
    }
}

/// A page of a PDF document.
///
/// This is a wrapper around a [`printpdf::PdfPageReference`][].
///
/// [`printpdf::PdfPageReference`]: https://docs.rs/printpdf/0.3.2/printpdf/types/pdf_page/struct.PdfPageReference.html
pub struct Page {
    page_idx: usize,
    size: Size,
    layers: Layers,
}

impl Page {
    fn new(page_idx: usize, layer_id: printpdf::LayerInternalId, size: Size) -> Page {
        Page {
            page_idx,
            size,
            layers: Layers::new(layer_id),
        }
    }

    /// Adds a new layer with the given name to the page.
    pub fn add_layer(&mut self, name: impl Into<String>) {
        let layer = printpdf::Layer::new(&name.into());
        self.layers.push_with_obj(layer);
    }

    /// Returns the number of layers on this page.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Returns a layer of this page.
    pub fn get_layer(&self, idx: usize) -> Option<Layer<'_>> {
        self.layers.get(idx).map(|l| Layer::new(self, l))
    }

    /// Returns the first layer of this page.
    pub fn first_layer(&self) -> Layer<'_> {
        Layer::new(self, self.layers.first())
    }

    /// Returns the last layer of this page.
    pub fn last_layer(&self) -> Layer<'_> {
        Layer::new(self, self.layers.last())
    }

    fn next_layer(&self, layer: &rc::Rc<cell::RefCell<LayerData>>) -> Layer<'_> {
        let layer = self.layers.next(&layer).unwrap_or_else(|| {
            let name = format!("Layer {}", self.layers.len() + 1);
            let layer = printpdf::Layer::new(&name);
            self.layers.push_with_obj(layer)
        });
        Layer::new(self, layer)
    }
}

#[derive(Debug)]
struct Layers(cell::RefCell<Vec<rc::Rc<cell::RefCell<LayerData>>>>);

impl Layers {
    pub fn new(layer_id: printpdf::LayerInternalId) -> Self {
        Self(
            vec![rc::Rc::from(cell::RefCell::new(LayerData::from_id(
                layer_id,
            )))]
            .into(),
        )
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn first(&self) -> rc::Rc<cell::RefCell<LayerData>> {
        self.0.borrow().first().unwrap().clone()
    }

    pub fn last(&self) -> rc::Rc<cell::RefCell<LayerData>> {
        self.0.borrow().last().unwrap().clone()
    }

    pub fn get(&self, idx: usize) -> Option<rc::Rc<cell::RefCell<LayerData>>> {
        self.0.borrow().get(idx).cloned()
    }

    pub fn push_with_obj(&self, layer_obj: printpdf::Layer) -> rc::Rc<cell::RefCell<LayerData>> {
        let layer_data = rc::Rc::from(cell::RefCell::new(LayerData::from_obj(layer_obj)));
        self.0.borrow_mut().push(layer_data.clone());
        layer_data
    }

    pub fn push_id(&self, layer_id: printpdf::LayerInternalId) -> rc::Rc<cell::RefCell<LayerData>> {
        let layer_data = rc::Rc::from(cell::RefCell::new(LayerData::from_id(layer_id)));
        self.0.borrow_mut().push(layer_data.clone());
        layer_data
    }

    pub fn next(
        &self,
        layer: &cell::RefCell<LayerData>,
    ) -> Option<rc::Rc<cell::RefCell<LayerData>>> {
        // Compare by internal id
        let id = layer.borrow().layer_id.clone();
        self.0
            .borrow()
            .iter()
            .skip_while(|l| l.borrow().layer_id != id)
            .nth(1)
            .cloned()
    }
}

/// A layer of a page of a PDF document.
#[derive(Clone)]
pub struct Layer<'p> {
    page: &'p Page,
    data: rc::Rc<cell::RefCell<LayerData>>,
}

impl<'p> Layer<'p> {
    fn new(page: &'p Page, data: rc::Rc<cell::RefCell<LayerData>>) -> Layer<'p> {
        Layer { page, data }
    }

    /// Returns the underlying layer internal id for this layer.
    pub fn layer(&self) -> printpdf::LayerInternalId {
        self.data.borrow().layer_id.clone()
    }

    /// Returns the next layer of this page.
    ///
    /// If this layer is not the last layer, the existing next layer is used.  If it is the last
    /// layer, a new layer is created and added to the page.
    pub fn next(&self) -> Layer<'p> {
        self.page.next_layer(&self.data)
    }

    /// Returns a drawable area for this layer.
    pub fn area(&self) -> Area<'p> {
        Area::new(self.clone(), Position::default(), self.page.size)
    }

    #[cfg(feature = "images")]
    fn add_image(
        &self,
        image: &image::DynamicImage,
        position: LayerPosition,
        scale: Scale,
        rotation: Rotation,
        dpi: Option<f32>,
    ) {
        // Image embedding is handled at PDF serialization time in printpdf 0.8.
        // For now, this is a no-op (image feature handling will be implemented as needed).
        let _ = (image, position, scale, rotation, dpi);
    }

    fn add_line_shape<I>(&self, points: I)
    where
        I: IntoIterator<Item = LayerPosition>,
    {
        let line_points: Vec<_> = points
            .into_iter()
            .map(|pos| printpdf::LinePoint {
                p: self.transform_position(pos).into(),
                bezier: false,
            })
            .collect();
        let line = printpdf::Line {
            points: line_points,
            is_closed: false,
        };
        self.data
            .borrow_mut()
            .ops
            .push(printpdf::Op::DrawLine { line });
    }

    fn set_fill_color(&self, color: Option<Color>) {
        if self.data.borrow().update_fill_color(color) {
            self.data.borrow_mut().ops.push(printpdf::Op::SetFillColor {
                col: color.unwrap_or(Color::Rgb(0, 0, 0)).into(),
            });
        }
    }

    fn set_outline_thickness(&self, thickness: Mm) {
        if self.data.borrow().update_outline_thickness(thickness) {
            self.data
                .borrow_mut()
                .ops
                .push(printpdf::Op::SetOutlineThickness {
                    pt: printpdf::Pt::from(thickness),
                });
        }
    }

    fn set_outline_color(&self, color: Color) {
        if self.data.borrow().update_outline_color(color) {
            self.data
                .borrow_mut()
                .ops
                .push(printpdf::Op::SetOutlineColor { col: color.into() });
        }
    }

    fn set_text_cursor(&self, cursor: LayerPosition) {
        let cursor = self.transform_position(cursor);
        self.data
            .borrow_mut()
            .ops
            .push(printpdf::Op::SetTextCursor { pos: cursor.into() });
    }

    fn begin_text_section(&self) {
        self.data
            .borrow_mut()
            .ops
            .push(printpdf::Op::StartTextSection);
    }

    fn end_text_section(&self) {
        self.data
            .borrow_mut()
            .ops
            .push(printpdf::Op::EndTextSection);
    }

    fn add_line_break(&self) {
        self.data.borrow_mut().ops.push(printpdf::Op::AddLineBreak);
    }

    fn set_line_height(&self, line_height: Mm) {
        self.data
            .borrow_mut()
            .ops
            .push(printpdf::Op::SetLineHeight {
                lh: printpdf::Pt::from(line_height),
            });
    }

    fn set_font(&self, font: &IndirectFontRef, font_size: u8) {
        match font {
            IndirectFontRef::Builtin(b) => {
                self.data
                    .borrow_mut()
                    .ops
                    .push(printpdf::Op::SetFontSizeBuiltinFont {
                        size: printpdf::Pt(font_size as f32),
                        font: *b,
                    })
            }
            IndirectFontRef::External(id) => {
                self.data.borrow_mut().ops.push(printpdf::Op::SetFontSize {
                    size: printpdf::Pt(font_size as f32),
                    font: id.clone(),
                })
            }
        }
    }

    fn write_positioned_codepoints<P, C>(&self, positions: P, codepoints: C)
    where
        P: IntoIterator<Item = i64>,
        C: IntoIterator<Item = u16>,
    {
        // Position-aware codepoint writing requires knowing which external font is active.
        // This is non-trivial to track in the current abstraction, so it's left as a no-op for now.
        let _ = (
            positions.into_iter().collect::<Vec<_>>(),
            codepoints.into_iter().collect::<Vec<_>>(),
        );
    }
    /// Transforms the given position that is relative to the upper left corner of the layer to a
    /// position that is relative to the lower left corner of the layer (as used by `printpdf`).
    fn transform_position(&self, position: LayerPosition) -> UserSpacePosition {
        UserSpacePosition::from_layer(self, position)
    }

    /// Adds a link annotation to the layer.
    pub fn add_annotation(&mut self, annotation: printpdf::LinkAnnotation) {
        self.data
            .borrow_mut()
            .ops
            .push(printpdf::Op::LinkAnnotation { link: annotation });
    }
}

#[derive(Debug)]
struct LayerData {
    layer_id: printpdf::LayerInternalId,
    layer_obj: Option<printpdf::Layer>,
    ops: Vec<printpdf::Op>,
    fill_color: cell::Cell<Color>,
    outline_color: cell::Cell<Color>,
    outline_thickness: cell::Cell<Mm>,
}

impl LayerData {
    pub fn from_id(layer_id: printpdf::LayerInternalId) -> Self {
        Self {
            layer_id,
            layer_obj: None,
            ops: Vec::new(),
            fill_color: Color::Rgb(0, 0, 0).into(),
            outline_color: Color::Rgb(0, 0, 0).into(),
            outline_thickness: Mm::from(printpdf::Pt(1.0)).into(),
        }
    }

    pub fn from_obj(layer: printpdf::Layer) -> Self {
        Self {
            layer_id: printpdf::LayerInternalId::new(),
            layer_obj: Some(layer),
            ops: Vec::new(),
            fill_color: Color::Rgb(0, 0, 0).into(),
            outline_color: Color::Rgb(0, 0, 0).into(),
            outline_thickness: Mm::from(printpdf::Pt(1.0)).into(),
        }
    }

    pub fn update_fill_color(&self, color: Option<Color>) -> bool {
        let color = color.unwrap_or(Color::Rgb(0, 0, 0));
        self.fill_color.replace(color) != color
    }

    pub fn update_outline_color(&self, color: Color) -> bool {
        self.outline_color.replace(color) != color
    }

    pub fn update_outline_thickness(&self, thickness: Mm) -> bool {
        self.outline_thickness.replace(thickness) != thickness
    }
}

/// A view on an area of a PDF layer that can be drawn on.
///
/// This struct provides access to the drawing methods of a [`printpdf::PdfLayerReference`][].  It
/// is defined by the layer that is drawn on and the origin and the size of the area.
///
/// [`printpdf::PdfLayerReference`]: https://docs.rs/printpdf/0.3.2/printpdf/types/pdf_layer/struct.PdfLayerReference.html
#[derive(Clone)]
pub struct Area<'p> {
    layer: Layer<'p>,
    origin: Position,
    size: Size,
}

impl<'p> Area<'p> {
    fn new(layer: Layer<'p>, origin: Position, size: Size) -> Area<'p> {
        Area {
            layer,
            origin,
            size,
        }
    }

    /// Returns a copy of this area on the next layer of the page.
    ///
    /// If this area is not on the last layer, the existing next layer is used.  If it is on the
    /// last layer, a new layer is created and added to the page.
    pub fn next_layer(&self) -> Self {
        let layer = self.layer.next();
        Self {
            layer,
            origin: self.origin,
            size: self.size,
        }
    }

    /// Reduces the size of the drawable area by the given margins.
    pub fn add_margins(&mut self, margins: impl Into<Margins>) {
        let margins = margins.into();
        self.origin.x += margins.left;
        self.origin.y += margins.top;
        self.size.width -= margins.left + margins.right;
        self.size.height -= margins.top + margins.bottom;
    }

    /// Returns the size of this area.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Adds the given offset to the area, reducing the drawable area.
    pub fn add_offset(&mut self, offset: impl Into<Position>) {
        let offset = offset.into();
        self.origin.x += offset.x;
        self.origin.y += offset.y;
        self.size.width -= offset.x;
        self.size.height -= offset.y;
    }

    /// Sets the size of this area.
    pub fn set_size(&mut self, size: impl Into<Size>) {
        self.size = size.into();
    }

    /// Sets the width of this area.
    pub fn set_width(&mut self, width: Mm) {
        self.size.width = width;
    }

    /// Sets the height of this area.
    pub fn set_height(&mut self, height: Mm) {
        self.size.height = height;
    }

    /// Splits this area horizontally using the given weights.
    ///
    /// The returned vector has the same number of elements as the provided slice.  The width of
    /// the *i*-th area is *width \* weights[i] / total_weight*, where *width* is the width of this
    /// area, and *total_weight* is the sum of all given weights.
    pub fn split_horizontally(&self, weights: &[usize]) -> Vec<Area<'p>> {
        let total_weight: usize = weights.iter().sum();
        let factor = self.size.width / total_weight as f32;
        let widths = weights.iter().map(|weight| factor * *weight as f32);
        let mut offset = Mm(0.0);
        let mut areas = Vec::new();
        for width in widths {
            let mut area = self.clone();
            area.origin.x += offset;
            area.size.width = width;
            areas.push(area);
            offset += width;
        }
        areas
    }

    /// Inserts an image into the document.
    ///
    /// *Only available if the `images` feature is enabled.*
    ///
    /// The position is assumed to be relative to the upper left hand corner of the area.
    /// Your position will need to compensate for rotation/scale/dpi. Using [`Image`][]'s
    /// render functionality will do this for you and is the recommended way to
    /// insert an image into an Area.
    ///
    /// [`Image`]: ../elements/struct.Image.html
    #[cfg(feature = "images")]
    pub fn add_image(
        &self,
        image: &image::DynamicImage,
        position: Position,
        scale: Scale,
        rotation: Rotation,
        dpi: Option<f32>,
    ) {
        self.layer
            .add_image(image, self.position(position), scale, rotation, dpi);
    }

    /// Draws a line with the given points and the given line style.
    ///
    /// The points are relative to the upper left corner of the area.
    pub fn draw_line<I>(&self, points: I, line_style: LineStyle)
    where
        I: IntoIterator<Item = Position>,
    {
        self.layer.set_outline_thickness(line_style.thickness());
        self.layer.set_outline_color(line_style.color());
        self.layer
            .add_line_shape(points.into_iter().map(|pos| self.position(pos)));
    }

    /// Tries to draw the given string at the given position and returns `true` if the area was
    /// large enough to draw the string.
    ///
    /// The font cache must contain the PDF font for the font set in the style.  The position is
    /// relative to the upper left corner of the area.
    pub fn print_str<S: AsRef<str>>(
        &self,
        font_cache: &fonts::FontCache,
        position: Position,
        style: Style,
        s: S,
    ) -> Result<bool, Error> {
        if let Some(mut section) =
            self.text_section(font_cache, position, style.metrics(font_cache))
        {
            section.print_str(s, style)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Creates a new text section at the given position if the text section fits in this area.
    ///
    /// The given style is only used to calculate the line height of the section.  The position is
    /// relative to the upper left corner of the area.  The font cache must contain the PDF font
    /// for all fonts printed with the text section.
    pub fn text_section<'f>(
        &self,
        font_cache: &'f fonts::FontCache,
        position: Position,
        metrics: fonts::Metrics,
    ) -> Option<TextSection<'f, 'p>> {
        let mut area = self.clone();
        area.add_offset(position);
        TextSection::new(font_cache, area, metrics)
    }

    /// Returns a position relative to the top left corner of this area.
    fn position(&self, position: Position) -> LayerPosition {
        LayerPosition::from_area(self, position)
    }

    /// Adds a clickable link to the document.
    ///
    /// The font cache must contain the PDF font for the font set in the style.  The position is
    /// relative to the upper left corner of the area.
    pub fn add_link<S: AsRef<str>>(
        &self,
        font_cache: &fonts::FontCache,
        position: Position,
        style: Style,
        text: S,
        uri: S,
    ) -> Result<bool, Error> {
        if let Some(mut section) =
            self.text_section(font_cache, position, style.metrics(font_cache))
        {
            section.add_link(text, uri, style)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// A text section that is drawn on an area of a PDF layer.
pub struct TextSection<'f, 'p> {
    font_cache: &'f fonts::FontCache,
    area: Area<'p>,
    is_first: bool,
    metrics: fonts::Metrics,
    font: Option<(IndirectFontRef, u8)>,
    current_x_offset: Mm,
    cumulative_kerning: Mm,
}

impl<'f, 'p> TextSection<'f, 'p> {
    fn new(
        font_cache: &'f fonts::FontCache,
        area: Area<'p>,
        metrics: fonts::Metrics,
    ) -> Option<TextSection<'f, 'p>> {
        if metrics.glyph_height > area.size.height {
            return None;
        }

        area.layer.begin_text_section();
        area.layer.set_line_height(metrics.line_height);

        Some(TextSection {
            font_cache,
            area,
            is_first: true,
            metrics,
            font: None,
            current_x_offset: Mm(0.0),
            cumulative_kerning: Mm(0.0),
        })
    }

    fn set_text_cursor(&self, x_offset: Mm) {
        let cursor = self
            .area
            .position(Position::new(x_offset, self.metrics.ascent));
        self.area.layer.set_text_cursor(cursor);
    }

    fn set_font(&mut self, font: &IndirectFontRef, font_size: u8) {
        let font_is_set = self
            .font
            .as_ref()
            .map(|(font, font_size)| (font, *font_size))
            .map(|data| data == (font, font_size))
            .unwrap_or_default();
        if !font_is_set {
            self.font = Some((font.clone(), font_size));
            self.area.layer.set_font(font, font_size);
        }
    }

    /// Tries to add a new line and returns `true` if the area was large enough to fit the new
    /// line.
    #[must_use]
    pub fn add_newline(&mut self) -> bool {
        if self.metrics.line_height > self.area.size.height {
            false
        } else {
            self.area.layer.add_line_break();
            self.area.add_offset((0, self.metrics.line_height));
            true
        }
    }

    /// Prints the given string with the given style.
    ///
    /// The font cache for this text section must contain the PDF font for the given style.
    pub fn print_str(&mut self, s: impl AsRef<str>, style: Style) -> Result<(), Error> {
        let font = style.font(self.font_cache);
        let s = s.as_ref();

        if self.is_first {
            if let Some(first_c) = s.chars().next() {
                let x_offset = style.char_left_side_bearing(self.font_cache, first_c) * -1.0;
                self.set_text_cursor(x_offset);
            }
            self.is_first = false;
        }

        let pdf_font = self
            .font_cache
            .get_pdf_font(font)
            .expect("Could not find PDF font in font cache")
            .clone();
        self.area.layer.set_fill_color(style.color());
        self.set_font(&pdf_font, style.font_size());

        // For built-in fonts, emit text as whole words/strings to avoid character-by-character spacing
        if font.is_builtin() {
            let items = vec![printpdf::TextItem::Text(s.to_string())];
            if let IndirectFontRef::Builtin(b) = pdf_font {
                self.area
                    .layer
                    .data
                    .borrow_mut()
                    .ops
                    .push(printpdf::Op::WriteTextBuiltinFont { items, font: b });
            }
        } else {
            // For embedded fonts, we still need precise positioning for proper kerning
            let kerning_positions = font.kerning(self.font_cache, s.chars());
            let positions: Vec<i64> = kerning_positions
                .clone()
                .into_iter()
                .map(|pos| (-pos * 1000.0) as i64)
                .collect();
            let codepoints = font.glyph_ids(&self.font_cache, s.chars());
            if let IndirectFontRef::External(fid) = pdf_font {
                self.area.layer.data.borrow_mut().ops.push(
                    printpdf::Op::WriteCodepointsWithKerning {
                        font: fid,
                        cpk: positions
                            .into_iter()
                            .zip(codepoints.into_iter())
                            .zip(s.chars())
                            .map(|((pos, cp), ch)| (pos, cp, ch))
                            .collect(),
                    },
                );
            }
        }

        // Update position tracking
        let text_width = style.text_width(self.font_cache, s);
        self.current_x_offset += text_width;

        // For built-in fonts, we don't need kerning tracking since PDF viewers handle it
        if !font.is_builtin() {
            let kerning_positions = font.kerning(self.font_cache, s.chars());
            let kerning_sum = Mm(kerning_positions.iter().sum::<f32>());
            self.cumulative_kerning += kerning_sum;
        }

        Ok(())
    }

    /// Adds a clickable link with the given text, URI, and style.
    ///
    /// The font cache for this text section must contain the PDF font for the given style.
    pub fn add_link(
        &mut self,
        text: impl AsRef<str>,
        uri: impl AsRef<str>,
        style: Style,
    ) -> Result<(), Error> {
        let font = style.font(self.font_cache);
        let text = text.as_ref();
        let uri = uri.as_ref();

        let kerning_positions: Vec<f32> = font.kerning(self.font_cache, text.chars());

        // Get current cursor position, including all accumulated offsets
        let current_pos = self.area.position(Position::new(
            self.current_x_offset + self.cumulative_kerning,
            0.0,
        ));

        let pdf_pos = self.area.layer.transform_position(current_pos);
        let text_width = style.text_width(self.font_cache, text);
        let left = pdf_pos.x.0;
        let bottom = pdf_pos.y.0 - font.ascent(style.font_size()).0;
        let width = text_width.0;
        let top = pdf_pos.y.0 + font.descent(style.font_size()).0;
        let height = top - bottom;
        let rect = printpdf::Rect {
            x: printpdf::Pt(left),
            y: printpdf::Pt(bottom),
            width: printpdf::Pt(width),
            height: printpdf::Pt(height),
        };

        let annotation = printpdf::LinkAnnotation::new(
            rect,
            printpdf::Actions::uri(uri.to_string()),
            Some(printpdf::BorderArray::Solid([0.0, 0.0, 0.0])), // No border
            Some(printpdf::ColorArray::Transparent),             // Transparent color
            None,
        );
        self.area.layer.add_annotation(annotation);

        // Handle first character positioning
        if self.is_first {
            if let Some(first_c) = text.chars().next() {
                let x_offset = style.char_left_side_bearing(self.font_cache, first_c) * -1.0;
                self.set_text_cursor(x_offset);
            }
            self.is_first = false;
        }

        let positions: Vec<i64> = kerning_positions
            .clone()
            .into_iter()
            .map(|pos| (-pos * 1000.0) as i64)
            .collect();

        let codepoints: Vec<u16> = if font.is_builtin() {
            encode_win1252(text)?
        } else {
            font.glyph_ids(&self.font_cache, text.chars())
        };

        let pdf_font = self
            .font_cache
            .get_pdf_font(font)
            .expect("Could not find PDF font in font cache")
            .clone();

        self.area.layer.set_fill_color(style.color());
        self.set_font(&pdf_font, style.font_size());

        // For built-in fonts, emit text as whole words/strings to avoid character-by-character spacing
        if font.is_builtin() {
            // Emit as a single WriteTextBuiltinFont op
            let items = vec![printpdf::TextItem::Text(text.to_string())];
            if let IndirectFontRef::Builtin(b) = pdf_font {
                self.area
                    .layer
                    .data
                    .borrow_mut()
                    .ops
                    .push(printpdf::Op::WriteTextBuiltinFont { items, font: b });
            }
        } else {
            // For embedded fonts, emit positioned codepoints with kerning
            if let IndirectFontRef::External(fid) = pdf_font {
                let cpk: Vec<(i64, u16, char)> = positions
                    .into_iter()
                    .zip(codepoints.into_iter())
                    .zip(text.chars())
                    .map(|((pos, cp), ch)| (pos, cp, ch))
                    .collect();
                self.area
                    .layer
                    .data
                    .borrow_mut()
                    .ops
                    .push(printpdf::Op::WriteCodepointsWithKerning { font: fid, cpk });
            }
        }

        // Update position tracking
        self.current_x_offset += text_width;

        // For built-in fonts, we don't need kerning tracking since PDF viewers handle it
        if !font.is_builtin() {
            let kerning_sum = Mm(kerning_positions.iter().sum::<f32>());
            self.cumulative_kerning += kerning_sum;
        }

        Ok(())
    }
}

impl<'f, 'p> Drop for TextSection<'f, 'p> {
    fn drop(&mut self) {
        self.area.layer.end_text_section();
    }
}

/// Encodes the given string using the Windows-1252 encoding for use with built-in PDF fonts,
/// returning an error if it contains unsupported characters.
fn encode_win1252(s: &str) -> Result<Vec<u16>, Error> {
    // Implement Windows-1252 encoding locally to avoid depending on lopdf internal API changes.
    // Map Unicode characters to single-byte Windows-1252 values where possible.
    let mut out: Vec<u16> = Vec::with_capacity(s.len());
    for c in s.chars() {
        let b = match c as u32 {
            0x00..=0x7F => Some(c as u8),
            0xA0..=0xFF => Some(c as u8),
            0x20AC => Some(0x80), // EURO SIGN
            0x201A => Some(0x82),
            0x0192 => Some(0x83),
            0x201E => Some(0x84),
            0x2026 => Some(0x85),
            0x2020 => Some(0x86),
            0x2021 => Some(0x87),
            0x02C6 => Some(0x88),
            0x2030 => Some(0x89),
            0x0160 => Some(0x8A),
            0x2039 => Some(0x8B),
            0x0152 => Some(0x8C),
            0x017D => Some(0x8E),
            0x2018 => Some(0x91),
            0x2019 => Some(0x92),
            0x201C => Some(0x93),
            0x201D => Some(0x94),
            0x2022 => Some(0x95),
            0x2013 => Some(0x96),
            0x2014 => Some(0x97),
            0x02DC => Some(0x98),
            0x2122 => Some(0x99),
            0x0161 => Some(0x9A),
            0x203A => Some(0x9B),
            0x0153 => Some(0x9C),
            0x017E => Some(0x9E),
            0x0178 => Some(0x9F),
            _ => None,
        };
        if let Some(b) = b {
            out.push(u16::from(b));
        } else {
            return Err(Error::new(
                format!(
                    "Tried to print a string with characters that are not supported by the Windows-1252 encoding with a built-in font: {}",
                    s
                ),
                ErrorKind::UnsupportedEncoding,
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_win1252_ok() {
        let s = "Hello, world!";
        let res = encode_win1252(s).expect("should encode ascii");
        assert_eq!(res.len(), s.chars().count());
    }

    #[test]
    fn test_encode_win1252_err() {
        let s = "Hello ☺";
        let res = encode_win1252(s);
        assert!(res.is_err());
    }

    #[test]
    fn test_renderer_and_page_layers_area_methods() {
        // Create renderer
        let mut r = Renderer::new(Size::new(210.0, 297.0), "test").expect("renderer");
        assert_eq!(r.page_count(), 1);
        r.add_page(Size::new(100.0, 100.0));
        assert_eq!(r.page_count(), 2);

        // Access page and layers: add layer via mutable borrow
        {
            let mut page_mut = r.get_page_mut(0).expect("page mut");
            assert_eq!(page_mut.layer_count(), 1);
            page_mut.add_layer("L2");
            assert!(page_mut.layer_count() >= 2);
        }

        // Get a layer and area via an immutable borrow
        let page = r.get_page(0).expect("page");
        let layer = page.first_layer();
        let mut area = layer.area();
        let orig_size = area.size();
        // Add margins and offsets
        area.add_margins(Margins::from((1.0f32, 2.0f32, 3.0f32, 4.0f32)));
        assert!(area.size().width.0 < orig_size.width.0);
        area.add_offset(Position::new(1.0, 1.0));
        assert!(area.size().width.0 < orig_size.width.0);

        // Set size and dimensions
        area.set_size(Size::new(50.0, 50.0));
        assert_eq!(area.size(), Size::new(50.0, 50.0));
        area.set_width(Mm::from(20.0));
        area.set_height(Mm::from(10.0));
        assert_eq!(area.size().width, Mm::from(20.0));
        assert_eq!(area.size().height, Mm::from(10.0));

        // split horizontally
        let areas = area.split_horizontally(&[1usize, 2usize]);
        assert_eq!(areas.len(), 2);

        // next layer from area
        let next_area = area.next_layer();
        assert_eq!(next_area.size(), area.size());
    }

    #[test]
    fn test_add_link_builtin_and_embedded() {
        use crate::fonts::{FontCache, FontData, FontFamily};
        use crate::style::Style;
        use crate::Context;

        // Renderer
        let mut r = Renderer::new(Size::new(210.0, 297.0), "test").expect("renderer");

        // Built-in font path
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd =
            FontData::new(data.clone(), Some(printpdf::BuiltinFont::Helvetica)).expect("font data");
        let family = FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let mut cache = FontCache::new(family);
        cache.load_pdf_fonts(&mut r).expect("load fonts");
        let context = Context::new(cache);

        // Embedded font path
        let fd2 = FontData::new(data, None).expect("font data");
        let family2 = FontFamily {
            regular: fd2.clone(),
            bold: fd2.clone(),
            italic: fd2.clone(),
            bold_italic: fd2.clone(),
        };
        let mut cache2 = FontCache::new(family2);
        cache2.load_pdf_fonts(&mut r).expect("load fonts");
        let context2 = Context::new(cache2);

        // Now access renderer page and layer
        let page = r.first_page();
        let layer = page.first_layer();

        // normal area should support link
        let area = layer.area();
        let style = Style::new().with_font_family(context.font_cache.default_font_family());
        assert!(area
            .add_link(
                &context.font_cache,
                Position::default(),
                style,
                "Hello",
                "http://example.com"
            )
            .unwrap());

        // too small area should return false
        let mut small_area = layer.area();
        small_area.set_size(Size::new(1.0, 1.0));
        let style = Style::new().with_font_family(context.font_cache.default_font_family());
        assert!(!small_area
            .add_link(
                &context.font_cache,
                Position::default(),
                style,
                "X",
                "http://x"
            )
            .unwrap());

        let style2 = Style::new().with_font_family(context2.font_cache.default_font_family());
        assert!(area
            .add_link(
                &context2.font_cache,
                Position::default(),
                style2,
                "Hi",
                "http://example.com"
            )
            .unwrap());
    }

    #[test]
    fn test_area_print_str_returns_false_when_too_small() {
        use crate::fonts::{FontCache, FontData, FontFamily};
        use crate::style::Style;

        let mut r = Renderer::new(Size::new(210.0, 297.0), "test").expect("renderer");

        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = FontData::new(data, None).expect("font data");
        let family = FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let mut cache = FontCache::new(family);
        cache.load_pdf_fonts(&mut r).expect("load fonts");

        let area = r.first_page().first_layer().area();
        let mut small_area = area.clone();
        small_area.set_size(Size::new(10.0, 0.1));
        let style = Style::new().with_font_family(cache.default_font_family());
        let res = small_area
            .print_str(&cache, Position::default(), style, "Hello")
            .unwrap();
        assert!(!res);
    }

    #[test]
    fn test_text_section_add_newline() {
        use crate::fonts::{FontCache, FontData, FontFamily};
        use crate::style::Style;

        let mut r = Renderer::new(Size::new(210.0, 297.0), "test").expect("renderer");
        let area = r.first_page().first_layer().area();

        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = FontData::new(data, None).expect("font data");
        let family = FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let cache = FontCache::new(family);

        let style = Style::new().with_font_family(cache.default_font_family());
        let metrics = style.metrics(&cache);

        let mut area2 = area.clone();
        area2.set_size(Size::new(100.0, metrics.line_height.0 + 1.0));
        let mut section = area2
            .text_section(&cache, Position::default(), metrics)
            .expect("should create section");
        assert!(section.add_newline());
    }
}
