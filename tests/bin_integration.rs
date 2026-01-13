// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;

const COMPLEX_PDF: &[u8] = include_bytes!("./mathml-AF-complex test.pdf");

#[test]
fn analyze_binary_detects_mathml_in_complex_pdf() -> Result<(), Box<dyn std::error::Error>> {
    // write embedded pdf to temp file
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(COMPLEX_PDF)?;
    let path = tmp.path().to_str().unwrap().to_string();

    // run the binary
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("analyze_pdf"));
    cmd.arg(&path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PDF ANALYSIS"))
        .stdout(predicate::str::contains("DOCUMENT INFORMATION"))
        .stdout(predicate::str::contains("EMBEDDED FONTS"))
        .stdout(
            predicate::str::contains("MATHML AND SPECIAL CONTENT")
                .or(predicate::str::contains("Potentially contains MathML"))
                .or(predicate::str::contains("Stream potentially MathML"))
                .or(predicate::str::contains(
                    "Stream content potentially contains MathML",
                ))
                .or(predicate::str::contains("MathML Stream/EmbeddedFile")),
        );

    Ok(())
}
