// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::path::Path;
use tree_sitter_graph::parse_error::TreeWithParseErrorVec;

pub struct DisplayParseErrorsPretty<'a> {
    pub parse_errors: &'a TreeWithParseErrorVec,
    pub path: &'a Path,
    pub source: &'a str,
    pub max_errors: usize,
}

impl std::fmt::Display for DisplayParseErrorsPretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parse_errors = self.parse_errors.errors();
        for parse_error in parse_errors.iter().take(self.max_errors) {
            write!(f, "{}", parse_error.display_pretty(self.path, &self.source))?;
        }
        if parse_errors.len() > self.max_errors {
            let more_errors = parse_errors.len() - self.max_errors;
            write!(
                f,
                "{} more parse error{} omitted\n",
                more_errors,
                if more_errors > 1 { "s" } else { "" },
            )?;
        }
        Ok(())
    }
}
