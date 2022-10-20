// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use std::path::PathBuf;
use tree_sitter_config::Config as TsConfig;

use crate::loader::LoadError;
use crate::loader::LoadPath;
use crate::loader::Loader;
use crate::loader::DEFAULT_BUILTINS_PATHS;
use crate::loader::DEFAULT_TSG_PATHS;

#[derive(Args)]
pub struct LoadArgs {
    /// The TSG file to use for stack graph construction.
    /// If the file extension is omitted, `.tsg` is implicitly added.
    #[clap(long, value_name = "TSG_PATH")]
    tsg: Option<PathBuf>,

    /// The builtins file to use for stack graph construction.
    /// If the file extension is omitted, the file extension of the language is implicitly added.
    #[clap(long, value_name = "BUILTINS_PATH")]
    builtins: Option<PathBuf>,

    /// The path to look for tree-sitter grammars.
    /// Can be specified multiple times.
    #[clap(long, value_name = "GRAMMAR_PATH")]
    grammar: Vec<PathBuf>,

    /// The scope of the tree-sitter grammar.
    /// See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#basics for details.
    #[clap(long, value_name = "SCOPE")]
    scope: Option<String>,
}

impl LoadArgs {
    pub fn new_loader(&self) -> Result<Loader, LoadError> {
        let tsg_paths = match &self.tsg {
            Some(tsg_path) => vec![LoadPath::Regular(tsg_path.clone())],
            None => DEFAULT_TSG_PATHS.clone(),
        };
        let builtins_paths = match &self.builtins {
            Some(builtins_path) => vec![LoadPath::Regular(builtins_path.clone())],
            None => DEFAULT_BUILTINS_PATHS.clone(),
        };

        let loader = if !self.grammar.is_empty() {
            Loader::from_paths(
                self.grammar.clone(),
                self.scope.clone(),
                tsg_paths,
                builtins_paths,
            )?
        } else {
            let loader_config = TsConfig::load()
                .and_then(|v| v.get())
                .map_err(LoadError::TreeSitter)?;
            Loader::from_config(
                &loader_config,
                self.scope.clone(),
                tsg_paths,
                builtins_paths,
            )?
        };
        Ok(loader)
    }
}
