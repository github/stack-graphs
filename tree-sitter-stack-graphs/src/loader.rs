// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Context;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tree_sitter::Language;
use tree_sitter_graph::ast::File as TsgFile;
use tree_sitter_graph::functions::Functions;
use tree_sitter_loader::Config as TsConfig;
use tree_sitter_loader::LanguageConfiguration;
use tree_sitter_loader::Loader as TsLoader;

use crate::StackGraphLanguage;

pub struct Loader {
    loader: SupplementedTsLoader,
    path: Option<PathBuf>,
    scope: Option<String>,
    tsg: Box<dyn Fn(Language) -> anyhow::Result<Option<TsgFile>>>,
    cache: Vec<(Language, StackGraphLanguage)>,
}

impl Loader {
    pub fn from_config(
        config: &TsConfig,
        path: Option<PathBuf>,
        scope: Option<String>,
        tsg: impl Fn(Language) -> anyhow::Result<Option<TsgFile>> + 'static,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            loader: SupplementedTsLoader::from_config(config)?,
            path,
            scope,
            tsg: Box::new(tsg),
            cache: Vec::new(),
        })
    }

    pub fn load_for_file(
        &mut self,
        path: &Path,
        content: Option<&str>,
    ) -> Result<Option<&mut StackGraphLanguage>, LoadError> {
        let selected_language = self.select_language_for_file(path, content)?;
        let language = match selected_language {
            Some(selected_language) => selected_language.clone(),
            None => return Ok(None),
        };
        // the borrow checker is a hard master...
        let index = self.cache.iter().position(|e| &e.0 == &language.language);
        let index = match index {
            Some(index) => index,
            None => {
                let mut functions = Functions::stdlib();
                crate::functions::add_path_functions(&mut functions);
                let tsg = self.load_tsg_for_language(&language)?;
                let sgl = StackGraphLanguage::new(language.language, tsg, functions)
                    .map_err(LoadError::other)?;
                self.cache.push((language.language, sgl));

                self.cache.len() - 1
            }
        };
        let sgl = &mut self.cache[index].1;
        Ok(Some(sgl))
    }

    // Select language for the given file, considering path and scope fields
    fn select_language_for_file(
        &mut self,
        file_path: &Path,
        file_content: Option<&str>,
    ) -> Result<Option<&SupplementedLanguage>, LoadError> {
        if let Some(path) = self.path.to_owned() {
            self.select_language_for_file_from_path(&path, file_path, file_content)
        } else {
            self.select_language_for_file_auto(file_path, file_content)
        }
    }

    // Select language from the current directory or the system for the given file, considering scope field
    fn select_language_for_file_auto(
        &mut self,
        file_path: &Path,
        file_content: Option<&str>,
    ) -> Result<Option<&SupplementedLanguage>, LoadError> {
        // The borrow checker is not smart enough to realize that the early returns
        // ensure any references from the self.select_* calls (which require a mutable
        // borrow) do not outlive the match. Therefor, we use a raw self_ptr and unsafe
        // dereferencing to make those calls.
        let self_ptr = self as *mut Self;
        let found_any_in_cwd = match unsafe { &mut *self_ptr }.select_language_for_file_from_path(
            &std::env::current_dir()?,
            file_path,
            file_content,
        ) {
            Ok(Some(language)) => return Ok(Some(language)),
            Ok(None) => true,
            Err(LoadError::NoLanguagesFound(_)) => false,
            Err(err) => return Err(err),
        };
        let found_any_in_system = match unsafe { &mut *self_ptr }
            .select_language_for_file_from_system(file_path, file_content)
        {
            Ok(Some(language)) => return Ok(Some(language)),
            Ok(None) => true,
            Err(LoadError::NoLanguagesFound(_)) => false,
            err => return err,
        };
        if !(found_any_in_cwd || found_any_in_system) {
            return Err(LoadError::NoLanguagesFound(format!(
                "in current directory or system{}",
                self.scope
                    .as_ref()
                    .map_or(String::default(), |s| format!(" for scope {}", s)),
            )));
        }
        Ok(None)
    }

    // Select language from the system for the given file, considering scope field
    fn select_language_for_file_from_system(
        &mut self,
        file_path: &Path,
        file_content: Option<&str>,
    ) -> Result<Option<&SupplementedLanguage>, LoadError> {
        let scope = self.scope.as_deref();
        let languages = self.loader.all_languages(scope)?;
        if languages.is_empty() {
            return Err(LoadError::NoLanguagesFound(format!(
                "on system{}",
                scope.map_or(String::default(), |s| format!(" for scope {}", s)),
            )));
        }
        if let Some(language) =
            SupplementedLanguage::best_for_file(languages, file_path, file_content)
        {
            return Ok(Some(language));
        };
        Ok(None)
    }

    // Select language from the given path for the given file, considering scope field
    fn select_language_for_file_from_path(
        &mut self,
        language_path: &Path,
        file_path: &Path,
        file_content: Option<&str>,
    ) -> Result<Option<&SupplementedLanguage>, LoadError> {
        let scope = self.scope.as_deref();
        let languages = self.loader.languages_at_path(language_path, scope)?;
        if languages.is_empty() {
            return Err(LoadError::NoLanguagesFound(format!(
                "at {}{}",
                language_path.display(),
                scope.map_or(String::default(), |s| format!(" for scope {}", s)),
            )));
        }
        if let Some(language) =
            SupplementedLanguage::best_for_file(languages, file_path, file_content)
        {
            return Ok(Some(language));
        };
        Ok(None)
    }

    // Load the TSG file for the given language and path
    fn load_tsg_for_language(&self, language: &SupplementedLanguage) -> Result<TsgFile, LoadError> {
        if let Some(tsg) = (self.tsg)(language.language)? {
            return Ok(tsg);
        }

        let tsg_path = language.root_path.join("queries/stack-graphs.tsg");
        if tsg_path.exists() {
            let tsg_source = std::fs::read(tsg_path.clone())
                .with_context(|| format!("Failed to read {}", tsg_path.display()))?;
            let tsg_source = String::from_utf8(tsg_source).map_err(LoadError::other)?;
            let tsg = TsgFile::from_str(language.language, &tsg_source)
                .with_context(|| format!("Failed to parse {}", tsg_path.display()))?;
            return Ok(tsg);
        }

        return Err(LoadError::NoTsgFound);
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("No languages found {0}")]
    NoLanguagesFound(String),
    #[error("No TSG file found")]
    NoTsgFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> Self {
        Self::Other(err.into())
    }
}

impl LoadError {
    fn other<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Other(error.into())
    }
}
// ------------------------------------------------------------------------------------------------
// tree_sitter_loader supplements

// Wraps a tree_sitter_loader::Loader
struct SupplementedTsLoader(
    TsLoader,
    HashMap<PathBuf, Vec<SupplementedLanguage>>,
    Vec<SupplementedLanguage>,
);

impl SupplementedTsLoader {
    pub fn from_config(config: &TsConfig) -> anyhow::Result<Self> {
        let loader = TsLoader::new()?;
        let mut result = Self(loader, HashMap::new(), Vec::new());
        result.load_system_languages(&config.parser_directories)?;
        Ok(result)
    }

    // Adopted from tree_sitter_loader::Loader::load
    fn load_system_languages(&mut self, parser_directories: &Vec<PathBuf>) -> anyhow::Result<()> {
        if parser_directories.is_empty() {
            eprintln!("Warning: You have not configured any parser directories!");
            eprintln!("Please run `tree-sitter init-config` and edit the resulting");
            eprintln!("configuration file to indicate where we should look for");
            eprintln!("language grammars.");
            eprintln!("");
        }
        let mut paths = Vec::new();
        for parser_container_dir in parser_directories {
            if let Ok(entries) = std::fs::read_dir(parser_container_dir) {
                for entry in entries {
                    let entry = entry?;
                    if let Some(parser_dir_name) = entry.file_name().to_str() {
                        if parser_dir_name.starts_with("tree-sitter-") {
                            paths.push(parser_container_dir.join(parser_dir_name));
                        }
                    }
                }
            }
        }
        let languages = self
            .languages_at_paths(&paths, None)?
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        self.2.extend(languages.into_iter());
        Ok(())
    }

    pub fn languages_at_path(
        &mut self,
        path: &Path,
        scope: Option<&str>,
    ) -> anyhow::Result<Vec<&SupplementedLanguage>> {
        self.languages_at_paths(&vec![path.to_path_buf()], scope)
    }

    pub fn all_languages(
        &mut self,
        scope: Option<&str>,
    ) -> anyhow::Result<Vec<&SupplementedLanguage>> {
        Ok(self
            .2
            .iter()
            .filter(|language| scope.map_or(true, |scope| language.matches_scope(scope)))
            .collect())
    }

    fn languages_at_paths(
        &mut self,
        paths: &Vec<PathBuf>,
        scope: Option<&str>,
    ) -> anyhow::Result<Vec<&SupplementedLanguage>> {
        for path in paths {
            if !self.1.contains_key(path) {
                let languages = self.0.languages_at_path(&path)?;
                let configurations = self.0.find_language_configurations_at_path(&path)?;
                let languages = languages
                    .into_iter()
                    .zip(configurations.into_iter())
                    .map(SupplementedLanguage::from)
                    .collect::<Vec<_>>();
                self.1.insert(path.clone(), languages);
            }
        }
        let mut result = Vec::new();
        for path in paths {
            let languages = &self.1[path];
            result.extend(
                languages
                    .iter()
                    .filter(|language| scope.map_or(true, |scope| language.matches_scope(scope))),
            );
        }
        Ok(result)
    }
}

#[derive(Clone, Debug)]
struct SupplementedLanguage {
    pub language: Language,
    pub scope: Option<String>,
    pub content_regex: Option<Regex>,
    pub file_types: Vec<String>,
    pub root_path: PathBuf,
}

impl SupplementedLanguage {
    pub fn matches_scope(&self, scope: &str) -> bool {
        self.scope.as_ref().map_or(false, |s| s == scope)
    }

    // Extracted from tree_sitter_loader::Loader::language_configuration_for_file_name
    pub fn matches_file(&self, path: &Path, content: Option<&str>) -> Option<isize> {
        // Check path extension
        if !path
            .extension()
            .and_then(OsStr::to_str)
            .map_or(false, |ext| self.file_types.iter().any(|ft| ft == ext))
        {
            return None;
        }

        // Apply content regex
        if let (Some(file_content), Some(content_regex)) = (content, &self.content_regex) {
            // If the language configuration has a content regex, assign
            // a score based on the length of the first match.
            if let Some(mat) = content_regex.find(&file_content) {
                let score = (mat.end() - mat.start()) as isize;
                return Some(score);
            } else {
                return None;
            }
        }

        Some(0isize)
    }

    // Extracted from tree_sitter_loader::Loader::language_configuration_for_file_name
    pub fn best_for_file<'a>(
        languages: Vec<&'a SupplementedLanguage>,
        path: &Path,
        content: Option<&str>,
    ) -> Option<&'a SupplementedLanguage> {
        let mut best_score = -1isize;
        let mut best = None;
        for language in languages {
            if let Some(score) = language.matches_file(path, content) {
                if score > best_score {
                    best_score = score;
                    best = Some(language);
                }
            }
        }
        best
    }
}

impl From<(Language, &LanguageConfiguration<'_>)> for SupplementedLanguage {
    fn from((language, config): (Language, &LanguageConfiguration)) -> Self {
        Self {
            scope: config.scope.clone(),
            content_regex: config.content_regex.clone(),
            file_types: config.file_types.clone(),
            root_path: config.root_path.clone(),
            language,
        }
    }
}
