// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use glob::Pattern;
use std::collections::HashMap;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::FileAnalyzer;
use tree_sitter_stack_graphs::LoadError;

use crate::util::*;

pub struct TsConfigAnalyzer {}

impl FileAnalyzer for TsConfigAnalyzer {
    fn build_stack_graph_into<'a>(
        &self,
        graph: &mut StackGraph,
        file: Handle<File>,
        path: &Path,
        source: &str,
        all_paths: &mut dyn Iterator<Item = &'a Path>,
        globals: &HashMap<String, String>,
        _cancellation_flag: &dyn tree_sitter_stack_graphs::CancellationFlag,
    ) -> Result<(), tree_sitter_stack_graphs::LoadError> {
        // read globals
        let proj_name = globals
            .get(crate::PROJECT_NAME_VAR)
            .map(String::as_str)
            .unwrap_or("");

        // parse source
        let tsc = TsConfig::parse_str(path, source).map_err(|_| LoadError::ParseError)?;

        // root node
        let root = StackGraph::root_node();

        // project scope
        let proj_scope_id = graph.new_node_id(file);
        let proj_scope = graph.add_scope_node(proj_scope_id, false).unwrap();
        add_debug_name(graph, proj_scope, "tsconfig.proj_scope");

        // project definition
        let proj_def = add_ns_pop(graph, file, root, PROJ_NS, proj_name, "tsconfig.proj_def");
        add_edge(graph, proj_def, proj_scope, 0);

        // project reference
        let proj_ref = add_ns_push(graph, file, root, PROJ_NS, proj_name, "tsconfig.proj_ref");
        add_edge(graph, proj_scope, proj_ref, 0);

        // root directory
        let pkg_def = add_pop(graph, file, proj_scope, PKG_M_NS, "tsconfig.pkg_def");
        let root_dir_ref = add_module_pushes(
            graph,
            file,
            M_NS,
            &tsc.root_dir(all_paths),
            proj_scope,
            "tsconfig.root_dir.ref",
        );
        add_edge(graph, pkg_def, root_dir_ref, 0);

        // auxiliary root directories, map relative imports to module paths
        for (idx, root_dir) in tsc.root_dirs().iter().enumerate() {
            let root_dir_def = add_pop(
                graph,
                file,
                proj_scope,
                REL_M_NS,
                &format!("tsconfig.root_dirs[{}].def", idx),
            );
            let root_dir_ref = add_module_pushes(
                graph,
                file,
                M_NS,
                root_dir,
                proj_scope,
                &format!("tsconfig.root_dirs[{}].ref", idx),
            );
            add_edge(graph, root_dir_def, root_dir_ref, 0);
        }

        // base URL
        let base_url = tsc.base_url();
        let base_url_def = add_pop(
            graph,
            file,
            proj_scope,
            NON_REL_M_NS,
            "tsconfig.base_url.def",
        );
        let base_url_ref = add_module_pushes(
            graph,
            file,
            M_NS,
            &base_url,
            proj_scope,
            "tsconfig.base_url.ref",
        );
        add_edge(graph, base_url_def, base_url_ref, 0);

        // path mappings
        for (from_idx, (from, tos)) in tsc.paths().iter().enumerate() {
            let is_prefix = from.file_name().map_or(true, |n| n == "*");
            let from = if is_prefix {
                from.parent().unwrap()
            } else {
                &from
            };
            let from_def = add_module_pops(
                graph,
                file,
                NON_REL_M_NS,
                from,
                proj_scope,
                &format!("tsconfig.paths[{}].from_def", from_idx),
            );
            for (to_idx, to) in tos.iter().enumerate() {
                let to = if is_prefix { to.parent().unwrap() } else { &to };
                let to_ref = add_module_pushes(
                    graph,
                    file,
                    M_NS,
                    to,
                    proj_scope,
                    &format!("tsconfig.paths[{}][{}].to_ref", from_idx, to_idx),
                );
                add_edge(graph, from_def, to_ref, 0);
            }
        }

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------

const TS_EXT: &str = "ts";
const TSX_EXT: &str = "tsx";
const JS_EXT: &str = "js";
const JSX_EXT: &str = "jsx";
const D_TS_EXT: &str = "d.ts";

struct TsConfig {
    project_dir: PathBuf,
    tsc: tsconfig::TsConfig,
}

impl TsConfig {
    fn parse_str(path: &Path, source: &str) -> Result<Self, LoadError> {
        let project_dir = path.parent().ok_or(LoadError::ParseError)?.to_path_buf();
        let tsc = tsconfig::TsConfig::parse_str(source).map_err(|_| LoadError::ParseError)?;
        Ok(Self { project_dir, tsc })
    }
}

impl TsConfig {
    /// Returns whether JS files are considered sources.
    ///
    /// See: https://www.typescriptlang.org/tsconfig#allowJs
    pub(self) fn allow_js(&self) -> bool {
        self.tsc
            .compiler_options
            .as_ref()
            .map_or(false, |co| co.allow_js.unwrap_or(false))
    }

    /// Returns the normalized, relative base URL.
    ///
    /// See: https://www.typescriptlang.org/tsconfig#baseUrl
    pub(self) fn base_url(&self) -> PathBuf {
        self.tsc
            .compiler_options
            .as_ref()
            .map_or(PathBuf::new(), |co| {
                co.base_url
                    .as_ref()
                    .and_then(|p| {
                        NormalizedRelativePath::from_str(p)
                            .filter(|p| !p.escapes())
                            .map(|p| p.into_path_buf())
                    })
                    .unwrap_or(PathBuf::default())
            })
    }

    /// Returns whether this is a composite project.
    ///
    /// See: https://www.typescriptlang.org/tsconfig#composite
    pub(self) fn composite(&self) -> bool {
        self.tsc
            .compiler_options
            .as_ref()
            .map_or(false, |co| co.composite.unwrap_or(false))
    }

    /// Returns the exclude patterns for sources.
    ///
    /// See: https://www.typescriptlang.org/tsconfig#exclude
    pub(self) fn exclude(&self) -> Vec<Pattern> {
        self.tsc.exclude.as_ref().map_or(vec![], |patterns| {
            patterns
                .iter()
                .flat_map(|p| self.expand_patterns(p))
                .collect()
        })
    }

    /// Returns listed source files.
    ///
    /// See: https://www.typescriptlang.org/tsconfig#files
    pub(self) fn files(&self) -> Vec<PathBuf> {
        self.tsc
            .files
            .as_ref()
            .map_or(vec![], |e| e.iter().map(PathBuf::from).collect())
    }

    /// Returns if `files` is defined.
    fn has_files(&self) -> bool {
        self.tsc.files.is_some()
    }

    /// Returns the include patterns for sources.
    ///
    /// See: https://www.typescriptlang.org/tsconfig#include
    pub(self) fn include(&self) -> Vec<Pattern> {
        if let Some(patterns) = &self.tsc.include {
            // we have explicit include patterns
            patterns
                .iter()
                .flat_map(|p| self.expand_patterns(p))
                .collect()
        } else if self.has_files() {
            // we have explicit files, so no default patterns
            vec![]
        } else {
            // use default patterns
            self.expand_patterns("**/*")
        }
    }

    /// Expands a pattern without a file extension to patterns for all allowed extensions.
    fn expand_patterns(&self, pattern: &str) -> Vec<Pattern> {
        let mut p = PathBuf::from(pattern);

        // if pattern has a file extension, use as is
        if p.extension().is_some() {
            return Pattern::new(&pattern).map_or(vec![], |p| vec![p]);
        }

        // if pattern has no file name, or the last component is `**` directory component, add a `*` file component
        if p.file_name().map_or(true, |n| n == "**") {
            p.push("*");
        }

        // determine accepted file extensions
        let mut es = vec![TS_EXT, TSX_EXT, D_TS_EXT];
        if self.allow_js() {
            es.extend(&[JS_EXT, JSX_EXT]);
        }

        // compute patterns---invalid patterns are silently ignored
        es.into_iter()
            .filter_map(|e| Pattern::new(p.with_extension(e).to_str().unwrap()).ok())
            .collect()
    }

    /// Returns path mappings.
    pub(self) fn paths(&self) -> HashMap<PathBuf, Vec<PathBuf>> {
        self.tsc
            .compiler_options
            .as_ref()
            .map_or(HashMap::default(), |co| {
                co.paths.as_ref().map_or(HashMap::default(), |ps| {
                    let mut m = HashMap::new();
                    for (key, values) in ps {
                        let from = match NormalizedRelativePath::from_str(key) {
                            Some(from) => from,
                            None => continue,
                        };
                        if from.escapes() {
                            continue;
                        }
                        let is_prefix = from.as_path().file_name().map_or(false, |n| n == "*");
                        let base_url = self.base_url();
                        let tos = values
                            .iter()
                            .filter_map(|v| {
                                let to = match NormalizedRelativePath::from_path(
                                    &base_url.as_path().join(v),
                                ) {
                                    Some(to) => to,
                                    None => return None,
                                };
                                if from.escapes() {
                                    return None;
                                }
                                if is_prefix
                                    && !from.as_path().file_name().map_or(false, |n| n == "*")
                                {
                                    return None;
                                }
                                Some(to.into())
                            })
                            .collect();
                        m.insert(from.into(), tos);
                    }
                    m
                })
            })
    }

    /// Return the root directory of this project.
    ///
    /// The root directory is:
    ///  1. The directory specified by the `compilerOptions.rootDir` property.
    ///  2. The project root, if the `compilerOptions.composite` property is set.
    ///  3. The longest common path of all non-declaration input files.
    ///     Currently the `files`, `include`, and `exclude` properties are ignored for this option.
    ///
    /// Parameters:
    ///  - source_paths: an iterable of source paths. The paths must be relative to the same origin as
    ///                  the tsconfig path, but may include paths outside this project.
    /// See: https://www.typescriptlang.org/tsconfig#rootDir
    pub(self) fn root_dir<'a, PI>(&self, source_paths: PI) -> PathBuf
    where
        PI: IntoIterator<Item = &'a Path>,
    {
        if let Some(root_dir) = self
            .tsc
            .compiler_options
            .as_ref()
            .and_then(|co| {
                co.root_dir
                    .as_ref()
                    .map(|p| NormalizedRelativePath::from_str(&p))
            })
            .flatten()
            .filter(|p| !p.escapes())
        {
            return root_dir.into();
        }

        if self.composite() {
            return PathBuf::default();
        }

        let mut root_dir: Option<PathBuf> = None;
        for input_path in self.input_files(source_paths) {
            if input_path
                .extension()
                .map(|ext| ext == D_TS_EXT)
                .unwrap_or(false)
            {
                continue;
            }

            let input_dir = match input_path.parent() {
                Some(input_dir) => input_dir,
                None => continue,
            };

            root_dir = Some(if let Some(root_dir) = root_dir {
                longest_common_prefix(&root_dir, input_dir).unwrap_or(root_dir)
            } else {
                input_dir.to_path_buf()
            });
        }

        root_dir.unwrap_or(PathBuf::default())
    }

    // Get additional relative root directories. Non relative paths are ignored.
    //
    // See: https://www.typescriptlang.org/tsconfig#rootDirs
    pub(self) fn root_dirs(&self) -> Vec<PathBuf> {
        self.tsc.compiler_options.as_ref().map_or(vec![], |co| {
            co.root_dirs.as_ref().map_or(vec![], |rs| {
                rs.iter()
                    .flat_map(|r| NormalizedRelativePath::from_str(r))
                    .filter(|r| !r.escapes())
                    .map(|r| r.into_path_buf())
                    .collect()
            })
        })
    }

    /// Returns an iterator over the input files of the project, taking `files`, `include`, and `exclude` into account.
    fn input_files<'a, PI>(&self, source_paths: PI) -> impl Iterator<Item = &'a Path>
    where
        PI: IntoIterator<Item = &'a Path>,
    {
        let files = self.files();
        let include = self.include();
        let exclude = self.exclude();

        let project_dir = self.project_dir.clone();
        source_paths.into_iter().filter_map(move |p| {
            // compute relative path in this project
            let p = match p.strip_prefix(&project_dir) {
                Ok(p) => p,
                Err(_) => return None,
            };

            // accept files in the file list
            for file in &files {
                if p == file {
                    return Some(p);
                }
            }

            // reject files not in the include patterns
            if !include.iter().any(|i| i.matches_path(p)) {
                return None;
            }

            // reject files matching exclude patterns
            if exclude.iter().any(|e| e.matches_path(p)) {
                return None;
            }

            // file was included, and not excluded, so accept
            Some(p)
        })
    }
}

// -------------------------------------------------------------------------------------------------

/// Computes the longest common prefix shared with the given path.
fn longest_common_prefix(left: &Path, right: &Path) -> Option<PathBuf> {
    let mut prefix = PathBuf::new();
    let mut left_it = left.components();
    let mut right_it = right.components();
    loop {
        match (left_it.next(), right_it.next()) {
            // prefixes must match
            (Some(sc @ Component::Prefix(sp)), Some(Component::Prefix(op))) if sp == op => {
                prefix.push(sc);
            }
            (Some(Component::Prefix(_)), _) | (_, Some(Component::Prefix(_))) => {
                return None;
            }
            // roots must match
            (Some(sc @ Component::RootDir), Some(Component::RootDir)) => {
                prefix.push(sc);
            }
            (Some(Component::RootDir), _) | (_, Some(Component::RootDir)) => {
                return None;
            }
            // right components may match
            (Some(sc), Some(oc)) if sc == oc => {
                prefix.push(sc);
            }
            // common prefix is done
            (_, _) => break,
        }
    }
    Some(prefix)
}

struct NormalizedRelativePath(PathBuf);

impl NormalizedRelativePath {
    pub(self) fn from_str(path: &str) -> Option<Self> {
        Self::from_path(Path::new(path))
    }

    /// Creates a new normalized, relative path from a path.
    pub(self) fn from_path(path: &Path) -> Option<Self> {
        let mut np = PathBuf::new();
        let mut normal_components = 0usize;
        for c in path.components() {
            match c {
                Component::Prefix(_) => {
                    return None;
                }
                Component::RootDir => {
                    return None;
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if normal_components > 0 {
                        // we can pop a normal component
                        normal_components -= 1;
                        np.pop();
                    } else {
                        // add the `..` to the beginning of this relative path which has no normal components
                        np.push(c);
                    }
                }
                Component::Normal(_) => {
                    normal_components += 1;
                    np.push(c);
                }
            }
        }
        Some(Self(np))
    }

    /// Returns if the relative path escapes to the parent.
    pub(self) fn escapes(&self) -> bool {
        self.0
            .components()
            .next()
            .map_or(false, |c| c == Component::ParentDir)
    }

    pub(self) fn as_path(&self) -> &Path {
        &self.0
    }

    pub(self) fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for NormalizedRelativePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Into<PathBuf> for NormalizedRelativePath {
    fn into(self) -> PathBuf {
        self.0
    }
}
