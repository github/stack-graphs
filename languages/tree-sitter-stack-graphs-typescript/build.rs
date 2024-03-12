// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2024, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::path::Path;

use anyhow::{bail, Result};
use regex::Regex;

const TSG_SOURCE: &str = "src/stack-graphs.tsg";
const DIALECTS: [&str; 2] = ["typescript", "tsx"];

/// preprocess the input file, removing lines that are not for the selected dialect
fn preprocess(
    input: impl std::io::Read,
    mut output: impl std::io::Write,
    dialect: &str,
) -> Result<()> {
    // Matches:   ; #dialect typescript
    let directive_start = Regex::new(r";\s*#dialect\s+(\w+)").unwrap();

    // Matches:   ; #end
    let directive_end = Regex::new(r";\s*#end").unwrap();

    let input = std::io::read_to_string(input)?;

    // If the filter is None or Some(true), the lines are written to the output
    let mut filter: Option<bool> = None;

    for (mut line_no, line) in input.lines().enumerate() {
        // Line numbers are one based
        line_no += 1;

        if let Some(captures) = directive_start.captures(line) {
            let directive = captures.get(1).unwrap().as_str();
            if !DIALECTS.contains(&directive) {
                bail!("Line {line_no}: unknown dialect: {directive}");
            }

            filter = Some(dialect == directive);
            output.write_all(line.as_bytes())?;
        } else if directive_end.is_match(line) {
            if filter.is_none() {
                bail!("Line {line_no}: unmatched directive end");
            }

            filter = None;
            output.write_all(line.as_bytes())?;
        } else if filter.unwrap_or(true) {
            output.write_all(line.as_bytes())?;
        }
        // a new line is always written so that removed lines are padded to preserve line numbers
        output.write(b"\n")?;
    }

    Ok(())
}

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR is not set");

    for dialect in DIALECTS {
        let input = std::fs::File::open(TSG_SOURCE).expect("Failed to open stack-graphs.tsg");

        let out_filename = Path::new(&out_dir).join(format!("stack-graphs-{dialect}.tsg"));
        let output = std::fs::File::create(out_filename).expect("Failed to create output file");

        preprocess(input, output, dialect).expect("Failed to preprocess stack-graphs.tsg");
    }

    println!("cargo:rerun-if-changed={TSG_SOURCE}");
    println!("cargo:rerun-if-changed=build.rs");
}
