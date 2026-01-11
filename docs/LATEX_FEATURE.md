# LaTeX Feature Integration

## Overview

The **LaTeX feature** for `genpdfi_extended` allows seamless rendering of mathematical formulas directly in PDF documents using the `Latex` element. Formulas are rendered at high resolution (720 DPI) via [MicroTeX](./microtex_rs) and integrated as SVG images with automatic scaling and positioning.

## Feature Activation

The LaTeX feature is **optional** and must be explicitly enabled in your `Cargo.toml`:

```toml
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["latex"] }
```

Or enable multiple features:

```toml
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["images", "latex"] }
```

## Basic Usage

### Simple Centered Formula

```rust
use genpdfi_extended::{elements, Document, Alignment};

let mut doc = Document::new(font_family);

// Create a centered formula
doc.push(
    elements::Latex::new(r#"a^2 + b^2 = c^2"#, 12.0)
        .with_alignment(Alignment::Center)
);

doc.render_to_file("output.pdf")?;
```

### Builder Methods

The `Latex` element supports method chaining:

```rust
// Basic construction
let formula = elements::Latex::new(formula_string, size_pt);

// Set horizontal alignment
let formula = formula.with_alignment(Alignment::Center);
let formula = formula.with_alignment(Alignment::Left);
let formula = formula.with_alignment(Alignment::Right);

// Set explicit position on page
use genpdfi_extended::Position;
let formula = formula.with_position(Position::new(x, y));

// Enable inline rendering mode
let formula = formula.inline();

// Disable inline mode (default)
let formula = formula.block();
```

## API Reference

### Constructor

```rust
pub fn new(formula: &str, size_pt: f32) -> Self
```

Creates a new LaTeX element with the given formula string and size in pseudo-points.

**Parameters:**
- `formula`: LaTeX source code (e.g., `"E = mc^2"` or `r#"\[x^2 + y^2\]"#`)
- `size_pt`: Font size in "pseudo points" (typically 8-16)

**Example:**
```rust
let formula = elements::Latex::new(r#"E = mc^2"#, 12.0);
```

### with_alignment()

```rust
pub fn with_alignment(mut self, alignment: Alignment) -> Self
```

Sets horizontal alignment for block-rendered formulas.

**Parameters:**
- `Alignment::Left` - Align to left margin
- `Alignment::Center` - Center on page (default)
- `Alignment::Right` - Align to right margin

**Example:**
```rust
let formula = elements::Latex::new(r#"\[\frac{1}{x}\]"#, 12.0)
    .with_alignment(Alignment::Center);
```

### with_position()

```rust
pub fn with_position(mut self, position: Position) -> Self
```

Sets explicit position, overriding alignment. Positions are in points.

**Example:**
```rust
use genpdfi_extended::Position;
let formula = elements::Latex::new(r#"E = mc^2"#, 12.0)
    .with_position(Position::new(50, 100));
```

### inline()

```rust
pub fn inline(mut self) -> Self
```

Enables inline rendering mode. This flag is available for future inline formula integration with text.

**Example:**
```rust
let formula = elements::Latex::new(r#"x^2"#, 10.0).inline();
```

### block()

```rust
pub fn block(mut self) -> Self
```

Disables inline mode (sets `inline = false`). This is the default behavior.

**Example:**
```rust
let formula = elements::Latex::new(r#"\[E = mc^2\]"#, 12.0).block();
```

## Size Specification

The `size_pt` parameter uses **"pseudo points"** - a relative sizing system similar to text font sizes:

| Size | Usage | Example |
|------|-------|---------|
| 8pt  | Superscripts, very small notes | Calibration markers |
| 10pt | Footnotes, inline annotations | Supporting equations |
| 12pt | Body text, standard formulas | Main equations |
| 14pt | Headings, important formulas | Primary results |
| 16pt | Titles, large displays | Large expressions |
| 18pt+ | Very large displays | Emphasis formulas |

**Note:** The actual pixel size is calculated by:
```
scale_factor = (target_height_px / reference_height_px) / 4.5
```

Where the reference formula "m" is rendered at 720 DPI and scaled to match the desired pseudo-point size.

## LaTeX Support

### Supported Syntax

The MicroTeX renderer supports standard LaTeX math mode syntax:

**Basic Operators:**
```latex
+ - * / = < > \leq \geq \neq \pm \mp \cdot \times \div
```

**Superscripts & Subscripts:**
```latex
x^2 y_n z^{n-1}_{n+1}
```

**Greek Letters:**
```latex
\alpha \beta \gamma \delta \epsilon \theta \lambda \pi \sigma \tau \phi \psi \omega
```

**Fractions & Radicals:**
```latex
\frac{a}{b}           % Fraction
\sqrt{x}              % Square root
\sqrt[3]{x}           % Cubic root
```

**Summation & Integration:**
```latex
\sum_{i=1}^{n} i      % Summation
\int_0^{2\pi} f(x) dx % Integral
\lim_{x \to \infty}   % Limit
```

**Trigonometric Functions:**
```latex
\sin(x) \cos(x) \tan(x) \cot(x) \sec(x) \csc(x)
```

**Logarithmic Functions:**
```latex
\log(x) \ln(x) \lg(x)
```

**Matrices & Arrays:**
```latex
\begin{matrix}
a & b \\
c & d
\end{matrix}
```

### Examples

**Physics:**
```rust
// Einstein's equation
elements::Latex::new(r#"E = mc^2"#, 12.0)

// Wave equation
elements::Latex::new(r#"\frac{\partial^2 u}{\partial t^2} = c^2 \nabla^2 u"#, 12.0)
```

**Mathematics:**
```rust
// Pythagorean theorem
elements::Latex::new(r#"a^2 + b^2 = c^2"#, 12.0)

// Quadratic formula
elements::Latex::new(
    r#"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#, 
    12.0
)

// Riemann integral
elements::Latex::new(
    r#"\int_a^b f(x) \, dx = \lim_{n \to \infty} \sum_{i=1}^n f(x_i) \Delta x"#,
    12.0
)
```

**Statistics:**
```rust
// Normal distribution
elements::Latex::new(
    r#"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}"#,
    12.0
)
```

## Complete Example

See [examples/latex_integration.rs](./examples/latex_integration.rs) for a complete working example:

```bash
cargo run --example latex_integration --features "images,latex"
```

This generates a PDF with:
- Basic centered formulas
- Formulas at different sizes (10pt, 12pt, 14pt)
- Complex mathematical expressions
- Multiple equations in document

See [examples/latex_advanced.rs](./examples/latex_advanced.rs) for advanced features:

```bash
cargo run --example latex_advanced --features "images,latex"
```

This demonstrates:
- Block formulas with alignment
- Positioned formulas at specific coordinates
- Multiple size specifications (8pt-16pt)
- Complex mathematical notation

## Technical Implementation

### Rendering Pipeline

1. **Formula Input** → LaTeX source code
2. **MicroTeX Rendering** → SVG at 720 DPI
3. **Scaling Calibration** → Reference "m" formula for scale calculation
4. **SVG Scaling** → Apply calculated scale to SVG dimensions
5. **Image Creation** → Convert SVG string to Image element
6. **PDF Rendering** → Apply positioning and alignment, render to PDF

### DPI & Scaling

- **Rendering DPI:** 720 (high quality output)
- **Adjustment Factor:** 4.5 (empirical calibration for pseudo-point sizing)
- **Reference Formula:** "m" character for consistent scaling

### Error Handling

All LaTeX elements use `genpdfi_extended::error::Error` with `ErrorKind::Internal` for failures:

```rust
pub enum Error {
    Internal(String),
    // ... other variants
}
```

Potential errors:
- Invalid LaTeX syntax in formula
- MicroTeX rendering failure
- SVG parsing/scaling failure
- Invalid size parameters

## Performance Considerations

- **First Rendering:** MicroTeX initialization (slight delay)
- **Per Formula:** SVG generation (~1-50ms depending on complexity)
- **Caching:** Consider caching rendered formulas for repeated use

## Feature Flag Behavior

### With `--features "latex"`

- ✅ `Latex` element available in `elements` module
- ✅ Can create and render LaTeX formulas
- ✅ Tests include LaTeX functionality (82 total tests)

### Without Feature

- ❌ `Latex` element is not compiled
- ✅ All other features work normally
- ✅ Original tests pass (78 tests)
- ✅ Full backward compatibility

## Troubleshooting

### "This example requires the 'latex' feature"

**Solution:** Enable the feature when running:
```bash
cargo run --example latex_integration --features "images,latex"
```

### Formula doesn't render

**Check:**
1. Valid LaTeX syntax (use `r#"..."#` for raw strings)
2. Size parameter is reasonable (8-16 recommended)
3. Formula string doesn't contain unescaped quotes

### PDF is too large

**Optimization:**
- SVG formulas include full font outlines (necessary for quality)
- Consider batching similar-sized formulas
- Use simpler formulas when possible

## Future Enhancements

- [ ] Inline formula support within text flow
- [ ] Formula caching for repeated use
- [ ] Custom DPI settings
- [ ] Direct LaTeX color specification
- [ ] Display mode vs. inline mode semantics

## Architecture

### Module Structure

```
genpdfi_extended/
├── src/
│   └── elements/
│       ├── mod.rs          (exports Latex conditionally)
│       ├── images.rs       (Image element)
│       └── latex.rs        (NEW - Latex element)
├── Cargo.toml              (adds latex feature)
└── examples/
    ├── latex_integration.rs (NEW - basic usage)
    └── latex_advanced.rs    (NEW - advanced features)
```

### Dependencies

- **microtex_rs** (optional, activated by `latex` feature)
  - LaTeX → SVG rendering
  - 720 DPI output
  - Comprehensive math notation support

- **genpdfi_extended** (core)
  - Image element for SVG rendering
  - Error handling infrastructure
  - PDF generation

## References

- [MicroTeX Documentation](./microtex_rs/README.md)
- [genpdfi_extended API](./README.md)
- [LaTeX Math Mode Reference](https://en.wikibooks.org/wiki/LaTeX/Mathematics)

---

**Feature Status:** ✅ Stable and Production-Ready

All LaTeX rendering functionality is complete, tested, and ready for production use.
