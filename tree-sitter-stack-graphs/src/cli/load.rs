// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use std::path::PathBuf;
use tree_sitter_config::Config as TsConfig;

use crate::loader::LanguageConfiguration;
use crate::loader::LoadError;
use crate::loader::LoadPath;
use crate::loader::Loader;
use crate::loader::DEFAULT_BUILTINS_PATHS;
use crate::loader::DEFAULT_TSG_PATHS;

#[derive(Args)]
pub struct PathLoaderArgs {
    /// The TSG file to use for stack graph construction.
    /// If the file extension is omitted, `.tsg` is implicitly added.
    #[clap(long, value_name = "TSG_PATH")]
    pub tsg: Option<PathBuf>,

    /// The builtins file to use for stack graph construction.
    /// If the file extension is omitted, the file extension of the language is implicitly added.
    #[clap(long, value_name = "BUILTINS_PATH")]
    pub builtins: Option<PathBuf>,

    /// The path to look for tree-sitter grammars.
    /// Can be specified multiple times.
    #[clap(long, value_name = "GRAMMAR_PATH")]
    pub grammar: Vec<PathBuf>,

    /// The scope of the tree-sitter grammar.
    /// See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#basics for details.
    #[clap(long, value_name = "SCOPE")]
    pub scope: Option<String>,
}

impl PathLoaderArgs {
    pub fn new() -> Self {
        Self {
            tsg: None,
            builtins: None,
            grammar: Vec::new(),
            scope: None,
        }
    }

    pub fn get(&self) -> Result<Loader, LoadError<'static>> {
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
            let loader_config = TsConfig::load(None)
                .and_then(|v| v.get())
                .map_err(LoadError::TreeSitter)?;
            Loader::from_tree_sitter_configuration(
                &loader_config,
                self.scope.clone(),
                tsg_paths,
                builtins_paths,
            )?
        };
        Ok(loader)
    }
}

/// CLI arguments for creating a path based loader.
#[derive(Args)]
pub struct LanguageConfigurationsLoaderArgs {
    /// The scope of the tree-sitter grammar.
    /// See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#basics for details.
    #[clap(long, value_name = "SCOPE")]
    scope: Option<String>,
}

impl LanguageConfigurationsLoaderArgs {
    pub fn new() -> Self {
        Self { scope: None }
    }

    pub fn get(
        &self,
        configurations: Vec<LanguageConfiguration>,
    ) -> Result<Loader, LoadError<'static>> {
        let loader = Loader::from_language_configurations(configurations, self.scope.clone())?;
        Ok(loader)
    }
}
