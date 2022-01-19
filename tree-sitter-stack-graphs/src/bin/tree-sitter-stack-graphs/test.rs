// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Run tests
#[derive(Parser)]
pub struct Command {
    /// The TSG file to use for stack graph construction.
    #[clap(long)]
    #[clap(name = "PATH")]
    tsg: Option<PathBuf>,

    /// The scope of the tree-sitter grammar to use.
    /// See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#basics for details.
    #[clap(long)]
    scope: Option<String>,

    /// Source paths to analyze.
    #[clap(name = "PATHS")]
    sources: Vec<PathBuf>,
}

impl Command {
    pub fn run(&self) -> Result<()> {
        Err(anyhow!("not implemented"))
    }
}
