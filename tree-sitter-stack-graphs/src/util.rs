// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use std::path::Path;
use tree_sitter_graph::parse_error::TreeWithParseErrorVec;

pub fn map_parse_errors(
    test_path: &Path,
    parse_errors: &TreeWithParseErrorVec,
    source: &str,
    prefix: &str,
    max_errors: usize,
) -> anyhow::Error {
    let mut error = String::new();
    let parse_errors = parse_errors.errors();
    for parse_error in parse_errors.iter().take(max_errors) {
        let line = parse_error.node().start_position().row;
        let column = parse_error.node().start_position().column;
        error.push_str(&format!(
            "{}{}:{}:{}: {}\n",
            prefix,
            test_path.display(),
            line + 1,
            column + 1,
            parse_error.display(&source, false)
        ));
    }
    if parse_errors.len() > max_errors {
        let more_errors = parse_errors.len() - max_errors;
        error.push_str(&format!(
            "  {} more parse error{} omitted\n",
            more_errors,
            if more_errors > 1 { "s" } else { "" },
        ));
    }
    anyhow!(error)
}
