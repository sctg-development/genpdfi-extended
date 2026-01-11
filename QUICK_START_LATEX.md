# Quick Start - Using LaTeX Formulas in genpdfi_extended

## 30-Second Setup

### 1. Enable Feature
```toml
[dependencies]
genpdfi_extended = { version = "0.3.4", features = ["latex"] }
```

### 2. Import and Use
```rust
use genpdfi_extended::{elements, Document, Alignment};

fn main() {
    // Create document and font (as usual)
    let mut doc = Document::new(font_family);
    
    // Add a LaTeX formula
    doc.push(
        elements::Latex::new(r#"E = mc^2"#, 12.0)
            .with_alignment(Alignment::Center)
    );
    
    // Render
    doc.render_to_file("output.pdf")?;
}
```

### 3. Build & Run
```bash
cargo build --features "latex"
./target/debug/your_app
```

## Common Use Cases

### Display Equation
```rust
doc.push(
    elements::Latex::new(r#"a^2 + b^2 = c^2"#, 14.0)
        .with_alignment(Alignment::Center)
);
```

### Different Sizes
```rust
// Small (8pt)
doc.push(elements::Latex::new(formula, 8.0));

// Normal (12pt) - recommended
doc.push(elements::Latex::new(formula, 12.0));

// Large (16pt)
doc.push(elements::Latex::new(formula, 16.0));
```

### Positioned Formula
```rust
use genpdfi_extended::Position;

doc.push(
    elements::Latex::new(r#"x^2 + y^2 = r^2"#, 12.0)
        .with_position(Position::new(50, 100))
);
```

### Aligned Formulas
```rust
use genpdfi_extended::Alignment;

// Left
.with_alignment(Alignment::Left)

// Center (default)
.with_alignment(Alignment::Center)

// Right
.with_alignment(Alignment::Right)
```

## Common Formulas

### Physics
```
E = mc^2
F = ma
v = \sqrt{\frac{2E_k}{m}}
```

### Mathematics
```
a^2 + b^2 = c^2
x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
\sum_{i=1}^{n} i = \frac{n(n+1)}{2}
```

## Run Examples

```bash
# Basic example
cargo run --example latex_integration --features "images,latex"

# Advanced example
cargo run --example latex_advanced --features "images,latex"
```

## Troubleshooting

### "No variant `Latex` found"
**Solution:** Enable the feature
```bash
cargo build --features "latex"
```

### Formula doesn't render
**Check:**
1. Valid LaTeX syntax
2. Use raw string: `r#"..."#`
3. Size is 8-16

## Learn More

- [LATEX_FEATURE.md](docs/LATEX_FEATURE.md) - Complete guide
- [LATEX_QUICK_REFERENCE.md](docs/LATEX_QUICK_REFERENCE.md) - API reference
- [LATEX_ARCHITECTURE.md](docs/LATEX_ARCHITECTURE.md) - Architecture
- [examples/LATEX_EXAMPLES.md](examples/LATEX_EXAMPLES.md) - Examples guide

---

**Status:** ✅ Ready to use!

Start creating PDFs with mathematical formulas today! 🚀
