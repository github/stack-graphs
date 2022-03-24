// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter::Language;
use tree_sitter_config::Config as TsConfig;
use tree_sitter_graph::ast::File as TsgFile;
use tree_sitter_stack_graphs::loader::Loader;

#[derive(Args)]
pub struct LoaderArgs {
    /// The TSG file to use for stack graph construction
    #[clap(long)]
    #[clap(name = "TSG_PATH")]
    tsg: Option<PathBuf>,

    /// The path to look for tree-sitter grammars.
    #[clap(long)]
    #[clap(name = "GRAMMAR_PATH")]
    grammar: Option<PathBuf>,

    /// The scope of the tree-sitter grammar.
    /// See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#basics for details.
    #[clap(long)]
    #[clap(name = "SCOPE")]
    scope: Option<String>,
}

impl LoaderArgs {
    pub fn new_loader(&self) -> Result<Loader> {
        let tsg_path = self.tsg.clone();
        let tsg = move |language| {
            if let Some(tsg_path) = &tsg_path {
                Self::load_tsg_from_path(language, &tsg_path).map(Some)
            } else {
                Ok(None)
            }
        };

        let loader_config = TsConfig::load()?.get()?;
        let loader = Loader::from_config(
            &loader_config,
            self.grammar.clone(),
            self.scope.clone(),
            tsg,
        )?;
        Ok(loader)
    }

    fn load_tsg_from_path(language: Language, tsg_path: &Path) -> Result<TsgFile> {
        let tsg_source = std::fs::read(tsg_path)
            .with_context(|| format!("Failed to read {}", tsg_path.display()))?;
        let tsg_source = String::from_utf8(tsg_source)?;
        let tsg = TsgFile::from_str(language, &tsg_source)
            .with_context(|| format!("Failed to parse {}", tsg_path.display()))?;
        return Ok(tsg);
    }
}
