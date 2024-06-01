// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines file loader for stack graph languages

use ini::Ini;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use stack_graphs::graph::StackGraph;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tree_sitter::Language;
use tree_sitter_graph::ast::File as TsgFile;
use tree_sitter_graph::Variables;
use tree_sitter_loader::Config as TsConfig;
use tree_sitter_loader::LanguageConfiguration as TSLanguageConfiguration;
use tree_sitter_loader::Loader as TsLoader;

use crate::CancellationFlag;
use crate::FileAnalyzer;
use crate::StackGraphLanguage;
use crate::FILE_PATH_VAR;

const BUILTINS_FILENAME: &str = "<builtins>";

pub static DEFAULT_TSG_PATHS: Lazy<Vec<LoadPath>> =
    Lazy::new(|| vec![LoadPath::Grammar("queries/stack-graphs".into())]);
pub static DEFAULT_BUILTINS_PATHS: Lazy<Vec<LoadPath>> =
    Lazy::new(|| vec![LoadPath::Grammar("queries/builtins".into())]);

/// Data type that holds all information to recognize and analyze files for a language
pub struct LanguageConfiguration {
    pub language: Language,
    pub scope: Option<String>,
    pub content_regex: Option<Regex>,
    pub file_types: Vec<String>,
    pub sgl: StackGraphLanguage,
    pub builtins: StackGraph,
    pub special_files: FileAnalyzers,
    /// Can be set to true if the stack graph rules ensure that there can be no similar
    /// paths in a file, in which case it is safe to turn of similar path detection. If
    /// incorrectly set to true, performance of path finding suffers from exponential
    /// blow up.
    pub no_similar_paths_in_file: bool,
}

impl LanguageConfiguration {
    /// Build a language configuration from tsg and builtins sources. The tsg path
    /// is kept for informational use only, see [`StackGraphLanguage::from_source`][].
    pub fn from_sources<'a>(
        language: Language,
        scope: Option<String>,
        content_regex: Option<Regex>,
        file_types: Vec<String>,
        tsg_path: PathBuf,
        tsg_source: &'a str,
        builtins_source: Option<(PathBuf, &'a str)>,
        builtins_config: Option<&str>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<Self, LoadError<'a>> {
        let sgl = StackGraphLanguage::from_source(language, tsg_path.clone(), tsg_source).map_err(
            |err| LoadError::SglParse {
                inner: err,
                tsg_path,
                tsg: Cow::from(tsg_source),
            },
        )?;
        let mut builtins = StackGraph::new();
        if let Some((builtins_path, builtins_source)) = builtins_source {
            let mut builtins_globals = Variables::new();

            builtins_globals
                .add(FILE_PATH_VAR.into(), BUILTINS_FILENAME.into())
                .expect("failed to add file path variable");

            if let Some(builtins_config) = builtins_config {
                Loader::load_globals_from_config_str(builtins_config, &mut builtins_globals)?;
            }
            let file = builtins.add_file(BUILTINS_FILENAME).unwrap();
            sgl.build_stack_graph_into(
                &mut builtins,
                file,
                builtins_source,
                &builtins_globals,
                cancellation_flag,
            )
            .map_err(|err| LoadError::Builtins {
                inner: err,
                source_path: builtins_path,
                source: Cow::from(builtins_source),
                tsg_path: sgl.tsg_path.clone(),
                tsg: Cow::from(tsg_source),
            })?;
        }
        Ok(LanguageConfiguration {
            language,
            scope,
            content_regex,
            file_types,
            sgl,
            builtins,
            special_files: FileAnalyzers::new(),
            no_similar_paths_in_file: false,
        })
    }

    // Extracted from tree_sitter_loader::Loader::language_configuration_for_file_name
    fn best_for_file<'a>(
        languages: &'a Vec<LanguageConfiguration>,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> std::io::Result<Option<&'a LanguageConfiguration>> {
        let mut best_score = -1isize;
        let mut best = None;
        for language in languages {
            if let Some(score) =
                matches_file(&language.file_types, &language.content_regex, path, content)?
            {
                if score > best_score {
                    best_score = score;
                    best = Some(language);
                }
            }
        }
        Ok(best)
    }

    pub fn matches_file(
        &self,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> std::io::Result<bool> {
        matches_file(&self.file_types, &self.content_regex, path, content).map(|l| l.is_some())
    }
}

#[derive(Clone, Default)]
pub struct FileAnalyzers {
    file_analyzers: HashMap<String, Arc<dyn FileAnalyzer + Send + Sync>>,
}

impl FileAnalyzers {
    pub fn new() -> Self {
        FileAnalyzers {
            file_analyzers: HashMap::new(),
        }
    }

    pub fn with(
        mut self,
        file_name: String,
        analyzer: impl FileAnalyzer + Send + Sync + 'static,
    ) -> Self {
        self.file_analyzers.insert(file_name, Arc::new(analyzer));
        self
    }

    pub fn add(
        &mut self,
        file_name: String,
        analyzer: impl FileAnalyzer + Send + Sync + 'static,
    ) -> &mut Self {
        self.file_analyzers.insert(file_name, Arc::new(analyzer));
        self
    }

    pub fn get(&self, file_name: &str) -> Option<Arc<dyn FileAnalyzer + Send + Sync>> {
        self.file_analyzers.get(file_name).cloned()
    }
}

/// A load path specifies a file to load from, either as a regular path or relative to the grammar location.
#[derive(Clone, Debug)]
pub enum LoadPath {
    Regular(PathBuf),
    Grammar(PathBuf),
}

impl LoadPath {
    fn get_for_grammar(&self, grammar_path: &Path) -> PathBuf {
        match self {
            Self::Regular(path) => path.clone(),
            Self::Grammar(path) => grammar_path.join(path),
        }
    }
}

/// The loader is created from either a tree-sitter configuration or a list of search paths, and an
/// optional scope and search paths for stack graphs definitions and builtins; or a list of language
/// configurations.
///
/// The loader is called with a file path and optional file content and tries to find the language for
/// that file. The loader will search for tree-sitter languages in the given search paths, or in current
/// directory and the paths defined in the tree-sitter configuration. If a scope is provided, it will be
/// used to restrict the discovered languages to those with a matching scope. If no languages were found
/// at all, an error is raised. Otherwise, a language matching the file path and content is returned, if
/// it exists among the discovered languages.
///
/// The paths for stack graphs definitions and builtins can be regular or relative to the grammar directory.
/// Paths may omit file extensions, in which case any supported file extension will be tried. The first path
/// that exists will be selected. It is considered an error if no stack graphs definitions is found. Builtins
/// are always optional.
///
/// Previously loaded languages are cached in the loader, so subsequent loads are fast.
pub struct Loader(LoaderImpl);

enum LoaderImpl {
    Paths(PathLoader),
    Provided(LanguageConfigurationsLoader),
}

impl Loader {
    pub fn from_paths(
        paths: Vec<PathBuf>,
        scope: Option<String>,
        tsg_paths: Vec<LoadPath>,
        builtins_paths: Vec<LoadPath>,
    ) -> Result<Self, LoadError<'static>> {
        Ok(Self(LoaderImpl::Paths(PathLoader {
            loader: SupplementedTsLoader::new()?,
            paths,
            scope,
            tsg_paths,
            builtins_paths,
            cache: Vec::new(),
        })))
    }

    pub fn from_tree_sitter_configuration(
        config: &TsConfig,
        scope: Option<String>,
        tsg_paths: Vec<LoadPath>,
        builtins_paths: Vec<LoadPath>,
    ) -> Result<Self, LoadError<'static>> {
        Ok(Self(LoaderImpl::Paths(PathLoader {
            loader: SupplementedTsLoader::new()?,
            paths: PathLoader::config_paths(config)?,
            scope,
            tsg_paths,
            builtins_paths,
            cache: Vec::new(),
        })))
    }

    pub fn from_language_configurations(
        configurations: Vec<LanguageConfiguration>,
        scope: Option<String>,
    ) -> Result<Self, LoadError<'static>> {
        let configurations = configurations
            .into_iter()
            .filter(|lc| scope.is_none() || lc.scope == scope)
            .collect();
        Ok(Self(LoaderImpl::Provided(LanguageConfigurationsLoader {
            configurations,
        })))
    }

    /// Load a Tree-sitter language for the given file. Loading is based on the loader configuration and the given file path.
    /// Most users should use [`Self::load_for_file`], but this method can be useful if only the underlying Tree-sitter language
    /// is necessary, as it will not attempt to load the TSG file.
    pub fn load_tree_sitter_language_for_file(
        &mut self,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> Result<Option<tree_sitter::Language>, LoadError<'static>> {
        match &mut self.0 {
            LoaderImpl::Paths(loader) => loader.load_tree_sitter_language_for_file(path, content),
            LoaderImpl::Provided(loader) => {
                loader.load_tree_sitter_language_for_file(path, content)
            }
        }
    }

    /// Load a stack graph language for the given file. Loading is based on the loader configuration and the given file path.
    pub fn load_for_file<'a>(
        &'a mut self,
        path: &Path,
        content: &mut dyn ContentProvider,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<FileLanguageConfigurations<'a>, LoadError<'static>> {
        match &mut self.0 {
            LoaderImpl::Paths(loader) => loader.load_for_file(path, content, cancellation_flag),
            LoaderImpl::Provided(loader) => loader.load_for_file(path, content),
        }
    }

    pub fn load_globals_from_config_path(
        path: &Path,
        globals: &mut Variables,
    ) -> Result<(), LoadError<'static>> {
        let conf = Ini::load_from_file(path)?;
        Self::load_globals_from_config(&conf, globals)
    }

    pub fn load_globals_from_config_str(
        config: &str,
        globals: &mut Variables,
    ) -> Result<(), LoadError<'static>> {
        if config.is_empty() {
            return Ok(());
        }
        let conf = Ini::load_from_str(config).map_err(ini::Error::Parse)?;
        Self::load_globals_from_config(&conf, globals)
    }

    fn load_tsg<'a>(
        language: Language,
        tsg_source: Cow<'a, str>,
    ) -> Result<TsgFile, LoadError<'a>> {
        let tsg = TsgFile::from_str(language, &tsg_source).map_err(|err| LoadError::TsgParse {
            inner: err,
            tsg_path: PathBuf::from("<unknown tsg path>"),
            tsg: Cow::from(tsg_source),
        })?;
        Ok(tsg)
    }

    fn load_builtins_into<'a>(
        sgl: &StackGraphLanguage,
        path: &Path,
        source: Cow<'a, str>,
        config: &str,
        graph: &mut StackGraph,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), LoadError<'a>> {
        let file_name = path.to_string_lossy();
        let file: stack_graphs::arena::Handle<stack_graphs::graph::File> =
            graph.add_file(&file_name).unwrap();
        let mut globals = Variables::new();

        globals
            .add(FILE_PATH_VAR.into(), BUILTINS_FILENAME.into())
            .expect("failed to add file path variable");

        Self::load_globals_from_config_str(&config, &mut globals)?;
        sgl.build_stack_graph_into(graph, file, &source, &globals, cancellation_flag)
            .map_err(|err| LoadError::Builtins {
                inner: err,
                source_path: path.to_path_buf(),
                source,
                tsg_path: sgl.tsg_path.to_path_buf(),
                tsg: sgl.tsg_source.clone(),
            })?;
        return Ok(());
    }

    fn load_globals_from_config(
        conf: &Ini,
        globals: &mut Variables,
    ) -> Result<(), LoadError<'static>> {
        if let Some(globals_section) = conf.section(Some("globals")) {
            for (name, value) in globals_section.iter() {
                globals.add(name.into(), value.into()).map_err(|_| {
                    LoadError::Reader(
                        format!("Duplicate global variable {} in config", name).into(),
                    )
                })?;
            }
        }
        Ok(())
    }
}

/// Struct holding the language configurations for a file.
#[derive(Default)]
pub struct FileLanguageConfigurations<'a> {
    /// The file's primary language. The language configuration's `StackGraphLanguage` should be used to process the file.
    pub primary: Option<&'a LanguageConfiguration>,
    /// Any secondary languages, which have special file analyzers for the file.
    pub secondary: Vec<(
        &'a LanguageConfiguration,
        Arc<dyn FileAnalyzer + Send + Sync>,
    )>,
}

impl FileLanguageConfigurations<'_> {
    pub fn has_some(&self) -> bool {
        self.primary.is_some() || !self.secondary.is_empty()
    }

    pub fn no_similar_paths_in_file(&self) -> bool {
        let mut no_similar_paths_in_file = true;
        if let Some(lc) = &self.primary {
            no_similar_paths_in_file &= lc.no_similar_paths_in_file;
        }
        for (lc, _) in &self.secondary {
            no_similar_paths_in_file &= lc.no_similar_paths_in_file;
        }
        return no_similar_paths_in_file;
    }
}

#[derive(Debug, Error)]
pub enum LoadError<'a> {
    #[error("{0}")]
    Cancelled(&'static str),
    #[error(transparent)]
    Config(#[from] ini::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{inner}")]
    SglParse {
        #[source]
        inner: crate::LanguageError,
        tsg_path: PathBuf,
        tsg: Cow<'a, str>,
    },
    #[error("No languages found {0}")]
    NoLanguagesFound(String),
    #[error("No TSG file found")]
    NoTsgFound,
    #[error(transparent)]
    Reader(Box<dyn std::error::Error + Send + Sync>),
    #[error("{inner}")]
    Builtins {
        #[source]
        inner: crate::BuildError,
        source_path: PathBuf,
        source: Cow<'a, str>,
        tsg_path: PathBuf,
        tsg: Cow<'a, str>,
    },
    #[error("{inner}")]
    TsgParse {
        inner: tree_sitter_graph::ParseError,
        tsg_path: PathBuf,
        tsg: Cow<'a, str>,
    },
    #[error(transparent)]
    TreeSitter(anyhow::Error),
}

impl LoadError<'_> {
    pub fn display_pretty<'a>(&'a self) -> impl std::fmt::Display + 'a {
        DisplayLoadErrorPretty { error: self }
    }
}

struct DisplayLoadErrorPretty<'a> {
    error: &'a LoadError<'a>,
}

impl std::fmt::Display for DisplayLoadErrorPretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error {
            LoadError::Builtins {
                inner,
                source_path,
                source,
                tsg_path,
                tsg,
            } => write!(
                f,
                "{}",
                inner.display_pretty(source_path, source, tsg_path, tsg)
            )?,
            LoadError::SglParse {
                inner,
                tsg_path,
                tsg,
            } => write!(f, "{}", inner.display_pretty(tsg_path, tsg))?,
            LoadError::TsgParse {
                inner,
                tsg_path,
                tsg,
            } => write!(f, "{}", inner.display_pretty(tsg_path, tsg))?,
            err => writeln!(f, "{}", err)?,
        }
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// provided languages loader

struct LanguageConfigurationsLoader {
    configurations: Vec<LanguageConfiguration>,
}

impl LanguageConfigurationsLoader {
    /// Load a Tree-sitter language for the given file. Loading is based on the loader configuration and the given file path.
    /// Most users should use [`Self::load_for_file`], but this method can be useful if only the underlying Tree-sitter language
    /// is necessary, as it will not attempt to load the TSG file.
    pub fn load_tree_sitter_language_for_file(
        &mut self,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> Result<Option<tree_sitter::Language>, LoadError<'static>> {
        for configuration in self.configurations.iter() {
            if configuration.matches_file(path, content)? {
                return Ok(Some(configuration.language));
            }
        }
        Ok(None)
    }

    /// Load a stack graph language for the given file. Loading is based on the loader configuration and the given file path.
    pub fn load_for_file<'a>(
        &'a mut self,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> Result<FileLanguageConfigurations<'a>, LoadError<'static>> {
        let primary = LanguageConfiguration::best_for_file(&self.configurations, path, content)?;
        let mut secondary = Vec::new();
        for language in self.configurations.iter() {
            if let Some(fa) = path
                .file_name()
                .and_then(|file_name| language.special_files.get(&file_name.to_string_lossy()))
            {
                secondary.push((language, fa));
            }
        }
        Ok(FileLanguageConfigurations { primary, secondary })
    }
}

// ------------------------------------------------------------------------------------------------
// path based loader

struct PathLoader {
    loader: SupplementedTsLoader,
    paths: Vec<PathBuf>,
    scope: Option<String>,
    tsg_paths: Vec<LoadPath>,
    builtins_paths: Vec<LoadPath>,
    cache: Vec<(Language, LanguageConfiguration)>,
}

impl PathLoader {
    // Adopted from tree_sitter_loader::Loader::load
    fn config_paths(config: &TsConfig) -> Result<Vec<PathBuf>, LoadError<'static>> {
        if config.parser_directories.is_empty() {
            eprintln!("Warning: You have not configured any parser directories!");
            eprintln!("Please run `tree-sitter init-config` and edit the resulting");
            eprintln!("configuration file to indicate where we should look for");
            eprintln!("language grammars.");
            eprintln!("");
        }
        let mut paths = Vec::new();
        for parser_container_dir in &config.parser_directories {
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
        Ok(paths)
    }

    pub fn load_tree_sitter_language_for_file(
        &mut self,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> Result<Option<tree_sitter::Language>, LoadError<'static>> {
        if let Some(selected_language) = self.select_language_for_file(path, content)? {
            return Ok(Some(selected_language.language));
        }
        Ok(None)
    }

    pub fn load_for_file<'a>(
        &'a mut self,
        path: &Path,
        content: &mut dyn ContentProvider,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<FileLanguageConfigurations<'a>, LoadError<'static>> {
        let selected_language = self.select_language_for_file(path, content)?;
        let language = match selected_language {
            Some(selected_language) => selected_language.clone(),
            None => return Ok(FileLanguageConfigurations::default()),
        };
        // the borrow checker is a hard master...
        let index = self.cache.iter().position(|e| &e.0 == &language.language);
        let index = match index {
            Some(index) => index,
            None => {
                let tsg = self.load_tsg_from_paths(&language)?;
                let sgl = StackGraphLanguage::new(language.language, tsg);

                let mut builtins = StackGraph::new();
                self.load_builtins_from_paths_into(
                    &language,
                    &sgl,
                    &mut builtins,
                    cancellation_flag,
                )?;

                let lc = LanguageConfiguration {
                    language: language.language,
                    scope: language.scope,
                    content_regex: language.content_regex,
                    file_types: language.file_types,
                    sgl,
                    builtins,
                    special_files: FileAnalyzers::new(),
                    // always detect similar paths, we don't know the language configuration when loading from the file system
                    no_similar_paths_in_file: false,
                };
                self.cache.push((language.language, lc));

                self.cache.len() - 1
            }
        };
        let lc = &self.cache[index].1;
        Ok(FileLanguageConfigurations {
            primary: Some(lc),
            secondary: Vec::default(),
        })
    }

    // Select language for the given file, considering paths and scope fields
    fn select_language_for_file(
        &mut self,
        file_path: &Path,
        file_content: &mut dyn ContentProvider,
    ) -> Result<Option<&SupplementedLanguage>, LoadError<'static>> {
        // The borrow checker is not smart enough to realize that the early returns
        // ensure any references from the self.select_* call (which require a mutable
        // borrow) do not outlive the match. Therefore, we use a raw self_ptr and unsafe
        // dereferencing to make those calls.
        let self_ptr = self as *mut Self;
        let mut found_languages = false;
        for path in &self.paths {
            found_languages |= match unsafe { &mut *self_ptr }.select_language_for_file_from_path(
                &path,
                file_path,
                file_content,
            ) {
                Ok(Some(language)) => return Ok(Some(language)),
                Ok(None) => true,
                Err(LoadError::NoLanguagesFound(_)) => false,
                Err(err) => return Err(err),
            };
        }
        if !found_languages {
            return Err(LoadError::NoLanguagesFound(format!(
                "in {}{}",
                self.paths.iter().map(|p| p.display()).format(":"),
                self.scope
                    .as_ref()
                    .map_or(String::default(), |s| format!(" for scope {}", s)),
            )));
        }
        Ok(None)
    }

    // Select language from the given path for the given file, considering scope field
    fn select_language_for_file_from_path(
        &mut self,
        language_path: &Path,
        file_path: &Path,
        file_content: &mut dyn ContentProvider,
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
            SupplementedLanguage::best_for_file(languages, file_path, file_content)?
        {
            return Ok(Some(language));
        };
        Ok(None)
    }

    // Load the TSG file for the given language and path
    fn load_tsg_from_paths(
        &self,
        language: &SupplementedLanguage,
    ) -> Result<TsgFile, LoadError<'static>> {
        for tsg_path in &self.tsg_paths {
            let mut tsg_path = tsg_path.get_for_grammar(&language.root_path);
            if tsg_path.extension().is_none() {
                tsg_path.set_extension("tsg");
            }
            if tsg_path.exists() {
                let tsg_source = std::fs::read_to_string(tsg_path)?;
                return Loader::load_tsg(language.language, Cow::from(tsg_source));
            }
        }
        return Err(LoadError::NoTsgFound);
    }

    // Builtins are loaded from queries/builtins.EXT and an optional queries/builtins.cfg configuration.
    // In the future, we may extend this to support builtins spread over multiple files queries/builtins/NAME.EXT
    // and optional corresponding configuration files queries/builtins/NAME.cfg.
    fn load_builtins_from_paths_into(
        &self,
        language: &SupplementedLanguage,
        sgl: &StackGraphLanguage,
        graph: &mut StackGraph,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), LoadError<'static>> {
        for builtins_path in &self.builtins_paths {
            let mut builtins_path = builtins_path.get_for_grammar(&language.root_path);
            if builtins_path.exists() && !builtins_path.is_dir() {
                return Self::load_builtins_from_path_into(
                    sgl,
                    &builtins_path,
                    graph,
                    cancellation_flag,
                );
            }
            for extension in &language.file_types {
                builtins_path.set_extension(extension);
                if builtins_path.exists() && !builtins_path.is_dir() {
                    return Self::load_builtins_from_path_into(
                        sgl,
                        &builtins_path,
                        graph,
                        cancellation_flag,
                    );
                }
            }
        }
        Ok(())
    }

    fn load_builtins_from_path_into(
        sgl: &StackGraphLanguage,
        builtins_path: &Path,
        graph: &mut StackGraph,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), LoadError<'static>> {
        let source = std::fs::read_to_string(builtins_path)?;
        let mut config_path = builtins_path.to_path_buf();
        config_path.set_extension("cfg");
        let config = if config_path.exists() {
            std::fs::read_to_string(builtins_path)?
        } else {
            "".into()
        };
        Loader::load_builtins_into(
            sgl,
            builtins_path,
            Cow::from(source),
            &config,
            graph,
            cancellation_flag,
        )
    }
}

// ------------------------------------------------------------------------------------------------
// tree_sitter_loader supplements

// Wraps a tree_sitter_loader::Loader
struct SupplementedTsLoader(TsLoader, HashMap<PathBuf, Vec<SupplementedLanguage>>);

impl SupplementedTsLoader {
    pub fn new() -> Result<Self, LoadError<'static>> {
        let loader = TsLoader::new().map_err(LoadError::TreeSitter)?;
        Ok(Self(loader, HashMap::new()))
    }

    pub fn languages_at_path(
        &mut self,
        path: &Path,
        scope: Option<&str>,
    ) -> Result<Vec<&SupplementedLanguage>, LoadError> {
        if !self.1.contains_key(path) {
            let languages = self
                .0
                .languages_at_path(&path)
                .map_err(LoadError::TreeSitter)?;
            let configurations = self
                .0
                .find_language_configurations_at_path(&path)
                .map_err(LoadError::TreeSitter)?;
            let languages = languages
                .into_iter()
                .zip(configurations.into_iter())
                .map(SupplementedLanguage::from)
                .filter(|language| scope.map_or(true, |scope| language.matches_scope(scope)))
                .collect::<Vec<_>>();
            self.1.insert(path.to_path_buf(), languages);
        }
        Ok(self.1[path].iter().map(|l| l).collect())
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
    pub fn matches_file(
        &self,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> std::io::Result<Option<isize>> {
        matches_file(&self.file_types, &self.content_regex, path, content)
    }

    // Extracted from tree_sitter_loader::Loader::language_configuration_for_file_name
    pub fn best_for_file<'a>(
        languages: Vec<&'a SupplementedLanguage>,
        path: &Path,
        content: &mut dyn ContentProvider,
    ) -> std::io::Result<Option<&'a SupplementedLanguage>> {
        let mut best_score = -1isize;
        let mut best = None;
        for language in languages {
            if let Some(score) = language.matches_file(path, content)? {
                if score > best_score {
                    best_score = score;
                    best = Some(language);
                }
            }
        }
        Ok(best)
    }
}

impl From<(Language, &TSLanguageConfiguration<'_>)> for SupplementedLanguage {
    fn from((language, config): (Language, &TSLanguageConfiguration)) -> Self {
        Self {
            scope: config.scope.clone(),
            content_regex: config.content_regex.clone(),
            file_types: config.file_types.clone(),
            root_path: config.root_path.clone(),
            language,
        }
    }
}

// Extracted from tree_sitter_loader::Loader::language_configuration_for_file_name
pub fn matches_file(
    file_types: &Vec<String>,
    content_regex: &Option<Regex>,
    path: &Path,
    content: &mut dyn ContentProvider,
) -> std::io::Result<Option<isize>> {
    // Check path extension
    if !path
        .extension()
        .and_then(OsStr::to_str)
        .map_or(false, |ext| file_types.iter().any(|ft| ft == ext))
    {
        return Ok(None);
    }

    // Apply content regex
    let content = content.get(path)?;
    if let (Some(file_content), Some(content_regex)) = (content, &content_regex) {
        // If the language configuration has a content regex, assign
        // a score based on the length of the first match.
        if let Some(mat) = content_regex.find(&file_content) {
            let score = (mat.end() - mat.start()) as isize;
            return Ok(Some(score));
        } else {
            return Ok(None);
        }
    }

    Ok(Some(0isize))
}

pub trait ContentProvider {
    fn get(&mut self, path: &Path) -> std::io::Result<Option<&str>>;
}

/// FileReader reads files from the filesystem and caches the most recently read file.
pub struct FileReader {
    cache: Option<(PathBuf, String)>,
}

impl FileReader {
    pub fn new() -> Self {
        Self { cache: None }
    }

    pub fn get(&mut self, path: &Path) -> std::io::Result<&str> {
        if self.cache.as_ref().map_or(true, |(p, _)| p != path) {
            let content = std::fs::read_to_string(path)?;
            self.cache = Some((path.to_path_buf(), content));
        }
        Ok(&self.cache.as_ref().unwrap().1)
    }
}

impl ContentProvider for FileReader {
    fn get(&mut self, path: &Path) -> std::io::Result<Option<&str>> {
        self.get(path).map(Some)
    }
}

impl ContentProvider for Option<&str> {
    fn get(&mut self, _path: &Path) -> std::io::Result<Option<&str>> {
        Ok(self.clone())
    }
}
