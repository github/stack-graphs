// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2024, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use regex::Regex;

/// A stack of dialects as selected by the directives
#[derive(Debug, Default)]
struct DialectStack(Vec<HashSet<String>>);

impl DialectStack {
    fn push(&mut self, values: Vec<String>) -> Result<()> {
        // ensure that the new values are a subset of the current scope
        if let Some(current) = self.0.last() {
            if !values.iter().all(|v| current.contains(v)) {
                bail!("Directive values are not a subset of the current scope");
            }
        }

        self.0.push(values.into_iter().collect());

        Ok(())
    }

    fn pop(&mut self) -> Result<()> {
        if let Some(_) = self.0.pop() {
            Ok(())
        } else {
            Err(anyhow!("Directive stack is empty"))
        }
    }

    fn contains(&self, query: &str) -> bool {
        if let Some(current) = self.0.last() {
            current.contains(query)
        } else {
           true
        }
    }
}

/// preprocess the input file, removing lines that are not for the selected dialect
fn preprocess(input: impl std::io::Read, mut output: impl std::io::Write, dialect: &str) -> anyhow::Result<()> {
    // Matches:   ; # dialect typescript tsx
    let directive_start = Regex::new(r";[ \t]*#[ \t]*dialect[ \t]+([a-zA-Z\t ]+)").unwrap();

    // Matches:   ; # end
    let directirve_end = Regex::new(r";[ \t]*#[ \t]*end").unwrap();

    let input = std::io::read_to_string(input)?;

    let mut stack = DialectStack::default();

    for line in input.lines() {
        if let Some(captures) = directive_start.captures(line) {
            let directive = captures.get(1).unwrap().as_str();
            let dialects = directive.split_whitespace().map(|s| s.to_string()).collect();
            stack.push(dialects)?;
            output.write_all(line.as_bytes())?;
        } else if directirve_end.is_match(line) {
            stack.pop()?;
            output.write_all(line.as_bytes())?;
        } else {
            if stack.contains(dialect) {
                output.write_all(line.as_bytes())?;
            }
        }
        // a new line is always written so that removed lines are padded to preserve line numbers
        output.write(b"\n")?;
    }

    Ok(())
}

const TSG_SOURCE: &str = "src/stack-graphs.tsg";
const DIALECTS: [&str; 2] = ["typescript", "tsx"];

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    for dialect in DIALECTS {
        let input = std::fs::File::open(TSG_SOURCE).unwrap();

        let out_filename = Path::new(&out_dir).join(format!("stack-graphs-{dialect}.tsg"));
        let output = std::fs::File::create(out_filename).unwrap();

        preprocess(input, output, dialect).unwrap();
    }

    println!("cargo:rerun-if-changed={TSG_SOURCE}");
    println!("cargo:rerun-if-changed=build.rs");
}
