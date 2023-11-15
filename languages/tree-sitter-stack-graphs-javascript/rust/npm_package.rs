// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::BuildError;
use tree_sitter_stack_graphs::FileAnalyzer;

use crate::util::*;

pub struct NpmPackageAnalyzer {}

impl FileAnalyzer for NpmPackageAnalyzer {
    fn build_stack_graph_into<'a>(
        &self,
        graph: &mut StackGraph,
        file: Handle<File>,
        _path: &Path,
        source: &str,
        _all_paths: &mut dyn Iterator<Item = &'a Path>,
        globals: &HashMap<String, String>,
        _cancellation_flag: &dyn tree_sitter_stack_graphs::CancellationFlag,
    ) -> Result<(), tree_sitter_stack_graphs::BuildError> {
        // read globals
        let pkg_internal_name = globals
            .get(crate::PROJECT_NAME_VAR)
            .map(String::as_str)
            .unwrap_or_default();

        // parse source
        let npm_pkg: NpmPackage =
            serde_json::from_str(source).map_err(|_| BuildError::ParseError)?;

        let root = StackGraph::root_node();

        // reach package internals from root
        //
        //     [root] -> [pop "GUARD:PKG_INTERNAL"] -> [pop pkg_internal_name]
        //
        let pkg_internal_guard_pop = add_pop(
            graph,
            file,
            root,
            PKG_INTERNAL_GUARD,
            "pkg_internal_guard_pop",
        );
        let pkg_internal_name_pop = add_pop(
            graph,
            file,
            pkg_internal_guard_pop,
            pkg_internal_name,
            "pkg_internal_name_pop",
        );

        // reach package internals via root
        //
        //     [push pkg_internal_name] -> [push "GUARD:PKG_INTERNAL"] -> [root]
        //
        let pkg_internal_guard_push = add_push(
            graph,
            file,
            root,
            PKG_INTERNAL_GUARD,
            "pkg_internal_guard_push",
        );
        let pkg_internal_name_push = add_push(
            graph,
            file,
            pkg_internal_guard_push,
            pkg_internal_name,
            "pkg_internal_name_push",
        );

        // reach exports via package name
        //
        //     [root] -> [pop "GUARD:PKG"] -> [pop PKG_NAME]* -> [push PKG_INTERNAL_NAME] -> [push "GUARD:PKG_INTERNAL"] -> [root]
        //
        if !npm_pkg.name.is_empty() {
            // NOTE Because all modules expose their exports at the top-level, both paths created below are equivalent for
            //      exports of the main module. This means multiple equivalent paths to those exports, which is bad for
            //      performance. At the moment, we have no mechanism to prevent this from happening.

            // reach package internals via package name
            //
            //     [root] -> [pop "GUARD:PKG"] -> [pop pkg_name]* -> [push pkg_internal_name]
            //
            let pkg_guard_pop = add_pop(graph, file, root, PKG_GUARD, "pkg_guard_pop");
            let pkg_name_pop = add_module_pops(
                graph,
                file,
                Path::new(&npm_pkg.name),
                pkg_guard_pop,
                "pkg_name_pop",
            );
            add_edge(graph, pkg_name_pop, pkg_internal_name_push, 0);

            // Common main
            let main = Some(npm_pkg.main)
                .filter(|main| !main.is_empty())
                .and_then(|main| NormalizedRelativePath::from_str(&main))
                .map(|p| p.into_path_buf())
                .unwrap_or(PathBuf::from("index"))
                .with_extension("");
            let main_push =
                add_module_pushes(graph, file, &main, pkg_internal_name_push, "main_push");

            // reach main directly via package name (with precedence)
            //
            //     [pop pkg_name] -1-> [push main]* -> [push pkg_internal_name]
            //
            add_edge(graph, pkg_name_pop, main_push, 0);
        }

        // reach dependencies via package internal name
        //
        //     [pop pkg_internal_name] -> [pop "GUARD:PKG"] -> [pop dep_name]* -> [push dep_name]* -> [push "GUARD:PKG"] -> [root]
        //
        let dep_guard_pop = add_pop(
            graph,
            file,
            pkg_internal_name_pop,
            PKG_GUARD,
            "dep_guard_pop",
        );
        let dep_guard_push = add_push(graph, file, root, PKG_GUARD, "dep_guard_push");
        for (i, (dep_name, _)) in npm_pkg.dependencies.iter().enumerate() {
            if dep_name.is_empty() {
                continue;
            }
            let dep_name_pop = add_module_pops(
                graph,
                file,
                Path::new(dep_name),
                dep_guard_pop,
                &format!("dep_name_pop[{}]", i),
            );
            let dep_name_push = add_module_pushes(
                graph,
                file,
                Path::new(dep_name),
                dep_guard_push,
                &format!("dep_name_push[{}", i),
            );
            add_edge(graph, dep_name_pop, dep_name_push, 0);
        }

        Ok(())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmPackage {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub main: String,
    #[serde(default)]
    pub dependencies: HashMap<String, serde_json::Value>,
}
