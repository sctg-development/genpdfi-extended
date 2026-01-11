# LaTeX Feature Usage Guide

## Quick Reference

### Enable Feature in Your Project

```toml
# Cargo.toml
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["latex"] }
```

### Basic Usage

```rust
use genpdfi_extended::elements;
use genpdfi_extended::Alignment;

// Simple formula
let formula = elements::Latex::new(r#"E = mc^2"#, 12.0);

// Centered formula
let formula = elements::Latex::new(r#"a^2 + b^2 = c^2"#, 12.0)
    .with_alignment(Alignment::Center);

// Add to document
doc.push(formula);
```

## API Summary

| Method | Purpose | Example |
|--------|---------|---------|
| `new(formula, size)` | Create Latex element | `Latex::new(r#"x^2"#, 12.0)` |
| `with_alignment(align)` | Set horizontal alignment | `.with_alignment(Alignment::Center)` |
| `with_position(pos)` | Set exact position | `.with_position(Position::new(20, 30))` |
| `inline()` | Enable inline mode | `.inline()` |
| `block()` | Disable inline mode | `.block()` |

## Common Patterns

### Display Math (Block)

```rust
// Default behavior - formula on own line
doc.push(
    elements::Latex::new(r#"\frac{\partial f}{\partial x}"#, 12.0)
        .with_alignment(Alignment::Center)
);
```

### Sized Formula

```rust
// Small formula
doc.push(elements::Latex::new(r#"x^2"#, 8.0));

// Large formula
doc.push(elements::Latex::new(r#"E = mc^2"#, 16.0));
```

### Positioned Formula

```rust
use genpdfi_extended::Position;

doc.push(
    elements::Latex::new(r#"a + b = c"#, 12.0)
        .with_position(Position::new(50, 100))
);
```

### Method Chaining

```rust
elements::Latex::new(r#"x^2 + y^2 = r^2"#, 14.0)
    .with_alignment(Alignment::Right)
    // Can chain multiple methods
```

## LaTeX Syntax Reference

### Operators

| Syntax | Output |
|--------|--------|
| `+` `-` `*` `/` | Basic arithmetic |
| `\times` `\div` `\cdot` | Multiplication, division, dot product |
| `=` `<` `>` `\leq` `\geq` | Comparisons |

### Superscripts & Subscripts

| Syntax | Output |
|--------|--------|
| `x^2` | x squared |
| `x_n` | x subscript n |
| `x^{2n}` | x to power 2n |
| `x_{n+1}` | x subscript n+1 |

### Fractions

| Syntax | Output |
|--------|--------|
| `\frac{a}{b}` | Fraction a/b |
| `\dfrac{a}{b}` | Display fraction |
| `\tfrac{a}{b}` | Text fraction |

### Radicals

| Syntax | Output |
|--------|--------|
| `\sqrt{x}` | Square root |
| `\sqrt[3]{x}` | Cube root |
| `\sqrt[n]{x}` | Nth root |

### Greek Letters

```
\alpha \beta \gamma \delta \epsilon \zeta \eta \theta
\iota \kappa \lambda \mu \nu \xi \omicron \pi
\rho \sigma \tau \upsilon \phi \chi \psi \omega
```

### Functions

```
\sin \cos \tan \cot \sec \csc
\sinh \cosh \tanh \coth
\arcsin \arccos \arctan
\log \ln \lg
\exp \det \dim
```

### Calculus

| Syntax | Output |
|--------|--------|
| `\int` | Integral |
| `\int_a^b` | Definite integral |
| `\sum_{i=1}^{n}` | Summation |
| `\prod_{i=1}^{n}` | Product |
| `\lim_{x \to a}` | Limit |
| `\frac{df}{dx}` | Derivative |
| `\partial f` | Partial derivative |

### Symbols

| Syntax | Output |
|--------|--------|
| `\infty` | Infinity |
| `\pm` `\mp` | Plus-minus, minus-plus |
| `\approx` `\sim` | Approximately, similar |
| `\rightarrow` `\leftarrow` | Arrows |
| `\in` `\notin` | Set membership |
| `\subset` `\supset` | Set operations |

### Accents

```
\hat{x}     \bar{x}     \tilde{x}     \vec{x}
\dot{x}     \ddot{x}    \acute{x}     \grave{x}
```

## Real-World Examples

### Physics

```rust
// Einstein's mass-energy equivalence
elements::Latex::new(r#"E = mc^2"#, 14.0)
    .with_alignment(Alignment::Center)

// Kinetic energy
elements::Latex::new(r#"E_k = \frac{1}{2}mv^2"#, 12.0)

// Wave equation
elements::Latex::new(
    r#"\frac{\partial^2 u}{\partial t^2} = c^2 \nabla^2 u"#, 
    12.0
)
```

### Mathematics

```rust
// Pythagorean theorem
elements::Latex::new(r#"a^2 + b^2 = c^2"#, 12.0)

// Quadratic formula
elements::Latex::new(
    r#"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#,
    12.0
)

// Euler's formula
elements::Latex::new(r#"e^{i\pi} + 1 = 0"#, 12.0)

// Binomial theorem
elements::Latex::new(
    r#"(x + y)^n = \sum_{k=0}^{n} \binom{n}{k} x^k y^{n-k}"#,
    12.0
)
```

### Statistics

```rust
// Normal distribution PDF
elements::Latex::new(
    r#"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}"#,
    12.0
)

// Z-score
elements::Latex::new(r#"z = \frac{x - \mu}{\sigma}"#, 12.0)

// Correlation coefficient
elements::Latex::new(
    r#"r = \frac{\sum (x_i - \bar{x})(y_i - \bar{y})}{\sqrt{\sum(x_i - \bar{x})^2} \sqrt{\sum(y_i - \bar{y})^2}}"#,
    12.0
)
```

### Chemistry

```rust
// Water molecule
elements::Latex::new(r#"H_2O"#, 12.0)

// Chemical equation
elements::Latex::new(r#"2H_2 + O_2 \rightarrow 2H_2O"#, 12.0)

// Equilibrium constant
elements::Latex::new(
    r#"K_c = \frac{[C]^c [D]^d}{[A]^a [B]^b}"#,
    12.0
)
```

## Size Guidelines

| Size | Use Case | Notes |
|------|----------|-------|
| 8pt | Exponents in exponents, very small text | Rarely used |
| 10pt | Footnotes, small annotations | For reference |
| 12pt | Body text, standard formulas | Default, recommended |
| 14pt | Section headers, important results | Common for emphasis |
| 16pt | Chapter titles, major display | Large expressions |
| 18pt+ | Full-page displays | Very large, eye-catching |

## Alignment Reference

```rust
use genpdfi_extended::Alignment;

// Left aligned
.with_alignment(Alignment::Left)

// Center aligned (default for most)
.with_alignment(Alignment::Center)

// Right aligned
.with_alignment(Alignment::Right)

// Explicit position (overrides alignment)
.with_position(Position::new(x, y))
```

## Error Handling

```rust
// The render() method is part of the Element trait
// and should be called automatically when adding to document

// If you need to handle rendering errors:
match formula.render(context) {
    Ok(rendered) => { /* use rendered */ },
    Err(e) => eprintln!("LaTeX error: {}", e),
}
```

## Performance Tips

1. **Pre-compile formulas:** Create formulas once, reuse as needed
2. **Use appropriate sizes:** Don't use extreme sizes (8pt, 18pt+)
3. **Simplify complex formulas:** Break large formulas into parts
4. **Batch similar sizes:** Formulas at same size may share rendering code

## Troubleshooting

### "Feature not enabled"

**Error:** LaTeX element not found

**Solution:**
```toml
# Add to Cargo.toml
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["latex"] }
```

### "Invalid LaTeX syntax"

**Error:** Formula doesn't render

**Check:**
- Use raw strings: `r#"formula"#`
- Escape backslashes: `\\` → `\`
- Valid MicroTeX syntax (see syntax reference above)

**Test:**
```rust
// Simple formula first
elements::Latex::new(r#"x^2"#, 12.0)

// Then add complexity
elements::Latex::new(r#"x^2 + y^2 = r^2"#, 12.0)
```

### "Formula rendering slow"

**Optimization:**
- Reduce formula complexity
- Avoid nested subscripts/superscripts
- Use standard sizes (10, 12, 14)

## Testing

Run with feature enabled:

```bash
# Build
cargo build --features "latex"

# Test
cargo test --lib --features "latex"

# Run examples
cargo run --example latex_integration --features "images,latex"
```

## Advanced Topics

### Custom Rendering Context

For advanced users, the `Element` trait can be implemented with custom context.

### Feature Combinations

```toml
# With images feature
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["images", "latex"] }

# Just LaTeX (no images)
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["latex"] }
```

## Resources

- **MicroTeX Docs:** See `microtex_rs/README.md`
- **Examples:** `examples/latex_integration.rs`, `examples/latex_advanced.rs`
- **Full Feature Docs:** `docs/LATEX_FEATURE.md`
- **LaTeX Reference:** [Wikibooks LaTeX/Mathematics](https://en.wikibooks.org/wiki/LaTeX/Mathematics)

## Changelog

### Version 0.3.4 (Initial Release)

- ✅ Basic LaTeX formula rendering
- ✅ Sizing in pseudo-points
- ✅ Alignment control
- ✅ Positioning support
- ✅ Inline/block mode flags
- ✅ Full test coverage
- ✅ Complete documentation and examples

---

**Status:** ✅ Production Ready

Start using LaTeX formulas in your PDFs today!
