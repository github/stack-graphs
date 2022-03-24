// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use tree_sitter_config::Config;
use tree_sitter_loader::Loader as TSLoader;
use tree_sitter_stack_graphs::loader::Loader;

#[derive(Args)]
pub struct LoaderArgs {
    /// The TSG file to use for stack graph construction
    #[clap(long)]
    #[clap(name = "PATH")]
    tsg: Option<PathBuf>,

    /// The scope of the tree-sitter grammar to use.
    /// See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#basics for details.
    #[clap(long)]
    scope: Option<String>,
}

impl LoaderArgs {
    pub fn new_loader(&self) -> Result<Loader> {
        let config = Config::load()?;

        let loader_config = config.get()?;
        let mut loader = TSLoader::new()?;
        loader.find_all_languages(&loader_config)?;

        Ok(Loader::new(self.tsg.clone(), self.scope.clone(), loader))
    }
}
