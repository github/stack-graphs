// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Context as _;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tree_sitter::Language;
use tree_sitter_graph::ast::File;
use tree_sitter_graph::functions::Functions;
use tree_sitter_loader::LanguageConfiguration;
use tree_sitter_loader::Loader as TSLoader;

use crate::StackGraphLanguage;

pub struct Loader {
    tsg: Option<PathBuf>,
    scope: Option<String>,
    loader: TSLoader,
    cache: Vec<(Language, StackGraphLanguage)>,
}

impl Loader {
    pub fn new(tsg: Option<PathBuf>, scope: Option<String>, loader: TSLoader) -> Self {
        Loader {
            tsg,
            scope,
            loader,
            cache: Vec::new(),
        }
    }

    pub fn load_for_source_path(
        &mut self,
        source_path: &Path,
    ) -> Result<&mut StackGraphLanguage, LoadError> {
        let current_dir = std::env::current_dir().map_err(LoadError::other)?;
        let scope = self.scope.clone();
        let (language, path) = self.select_language(source_path, &current_dir, scope.as_deref())?;
        // the borrow checker is a hard master...
        let index = self.cache.iter().position(|e| &e.0 == &language);
        let index = match index {
            Some(index) => index,
            None => {
                let functions = Functions::stdlib();
                let tsg = self.load_tsg_for_language(language, &path)?;
                let sgl =
                    StackGraphLanguage::new(language, tsg, functions).map_err(LoadError::other)?;
                self.cache.push((language, sgl));
                self.cache.len() - 1
            }
        };
        let sgl = &mut self.cache[index].1;
        Ok(sgl)
    }

    // This is a modified version of tree_sitter_loader::Loader::select_language that also returns the language path.
    // TODO: Some version of this should be upstreamed to tree-sitter-loader
    fn select_language(
        &mut self,
        path: &Path,
        current_dir: &Path,
        scope: Option<&str>,
    ) -> Result<(Language, PathBuf), LoadError> {
        if let Some(scope) = scope {
            if let Some((lang, config)) = self
                .loader
                .language_configuration_for_scope(scope)
                .with_context(|| format!("Failed to load language for scope '{}'", scope))?
            {
                Ok((lang, config.root_path.clone()))
            } else {
                return Err(LoadError::UnknownLanguageScope(scope.to_string()));
            }
        } else if let Some((lang, config)) = self
            .loader
            .language_configuration_for_file_name(path)
            .with_context(|| format!("Failed to load language for file name {}", &path.display()))?
        {
            Ok((lang, config.root_path.clone()))
        } else if let Some((lang, config)) = self
            .language_configurations_at_path(&current_dir)
            .with_context(|| "Failed to load language in current directory")?
            .first()
            .cloned()
        {
            Ok((lang, config.root_path.clone()))
        } else {
            Err(LoadError::NoLanguageFound)
        }
    }

    // This is a stand in for a missing tree_sitter_loader::Loader::languages_at_path method
    // TODO: Some version of this should be upstreamed to tree-sitter-loader
    fn language_configurations_at_path(
        &mut self,
        path: &Path,
    ) -> Result<Vec<(Language, &LanguageConfiguration<'_>)>, LoadError> {
        let languages = self.loader.languages_at_path(path)?;
        let configurations = self.loader.find_language_configurations_at_path(path)?;
        let result = languages
            .iter()
            .cloned()
            .zip(configurations.iter())
            .into_iter()
            .collect();
        Ok(result)
    }

    fn load_tsg_for_language(&self, language: Language, path: &Path) -> Result<File, LoadError> {
        let lang_tsg_path = path.join("queries/stack-graphs.tsg");
        let tsg_path = if let Some(path) = &self.tsg {
            path
        } else if lang_tsg_path.exists() {
            &lang_tsg_path
        } else {
            return Err(LoadError::NoTsgFound);
        };

        let tsg_source = std::fs::read(tsg_path)
            .with_context(|| format!("Failed to read {}", tsg_path.display()))?;
        let tsg_source = String::from_utf8(tsg_source).map_err(LoadError::other)?;
        let tsg = File::from_str(language, &tsg_source)
            .with_context(|| format!("Failed to parse {}", tsg_path.display()))?;

        Ok(tsg)
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("No language found")]
    NoLanguageFound,
    #[error("No TSG file found")]
    NoTsgFound,
    #[error("Unknown language scope {0}")]
    UnknownLanguageScope(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl LoadError {
    fn other<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Other(error.into())
    }
}
