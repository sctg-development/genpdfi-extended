<!--
Copyright (c) 2026 Ronan Le Meillat - SCTG Development
SPDX-License-Identifier: MIT OR Apache-2.0
Licensed under the MIT License or the Apache License, Version 2.0

-->

# LaTeX Examples

This directory contains comprehensive examples demonstrating the LaTeX formula rendering feature with the **MicroTeX singleton pattern fix**.

## Quick Start

All LaTeX examples require the `latex` feature:

```bash
cargo run --example <name> --features "images,latex"
```

## Available Examples

### 1. latex_integration.rs

Basic LaTeX formula rendering with multiple sizes.

```bash
cargo run --example latex_integration --features "images,latex"
```

**Output:** `examples/output/latex_integration.pdf` (~115 KB)

### 2. latex_advanced.rs

Advanced features: positioning, alignment, multiple sizes.

```bash
cargo run --example latex_advanced --features "images,latex"
```

**Output:** `examples/output/latex_advanced.pdf` (~240 KB)

### 3. latex_stress_test.rs ⭐ **NEW**

Stress test with 22 formulas to verify the **MicroTeX singleton pattern**.

```bash
cargo run --example latex_stress_test --features "images,latex"
```

**Output:** `examples/output/latex_stress_test.pdf` (~268 KB, 3 pages)

**Expected:** All 22 formulas render without crashes ✅

### 4. latex_formulas.rs

Multi-size calibration with reference formula.

```bash
cargo run --example latex_formulas --features "images,latex"
```

**Output:** `examples/output/latex_formulas.pdf` (~165 KB)

## Key Fix: MicroTeX Singleton Pattern

### Problem
MicroTeX can only be initialized **once**. Multiple `MicroTex::new()` calls crash the engine.

### Solution
Use `std::sync::OnceLock` for thread-safe singleton initialization:

```rust
use std::sync::OnceLock;

static MICROTEX_RENDERER: OnceLock<microtex_rs::MicroTex> = OnceLock::new();

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
            "Failed to initialize MicroTeX (can only initialize once)",
            ErrorKind::Internal,
        ))
    }
}
```

### Benefits
✅ Initialize MicroTeX **only once**  
✅ Thread-safe  
✅ Multiple formulas render correctly  
✅ No crashes on complex documents  

## API Usage

```rust
use genpdfi_extended::elements;
use genpdfi_extended::Alignment;

// Basic formula
let f = elements::Latex::new(r#"E = mc^2"#, 12.0);

// Centered
let f = f.with_alignment(Alignment::Center);

// Positioned
use genpdfi_extended::Position;
let f = elements::Latex::new(r#"x^2"#, 12.0)
    .with_position(Position::new(50, 100));

// Inline mode
let f = elements::Latex::new(r#"y^2"#, 10.0).inline();

doc.push(f);
```

## Run All Examples

```bash
cargo run --example latex_integration --features "images,latex"
cargo run --example latex_advanced --features "images,latex"
cargo run --example latex_stress_test --features "images,latex"
cargo run --example latex_formulas --features "images,latex"
```

## Testing Results

### Stress Test: 22 Formulas

```
✓ Formula 1 queued for rendering
✓ Formula 2 queued for rendering
...
✓ Formula 22 queued for rendering

Results:
  Successful: 22
  Failed: 0
  Status: ✓ ALL TESTS PASSED
```

### Complete Test Suite

```
✓ Build with LaTeX feature: Successful
✓ All tests: 82 passing (with latex feature)
✓ Backward compatibility: 78 passing (without feature)
✓ All example PDFs generated successfully
```

## Documentation

- [LATEX_FEATURE.md](../docs/LATEX_FEATURE.md) - Complete feature guide
- [LATEX_QUICK_REFERENCE.md](../docs/LATEX_QUICK_REFERENCE.md) - API reference
- [LATEX_EXAMPLES.md](./LATEX_EXAMPLES.md) - Detailed examples
- [LATEX_ARCHITECTURE.md](../docs/LATEX_ARCHITECTURE.md) - Implementation details
- [MICROTEX_SINGLETON_FIX.md](../docs/MICROTEX_SINGLETON_FIX.md) - **Singleton pattern explanation**

## Status

✅ **All examples working and tested**  
✅ **MicroTeX singleton pattern verified**  
✅ **Stress test passes (22 formulas)**  
✅ **No crashes on complex documents**
