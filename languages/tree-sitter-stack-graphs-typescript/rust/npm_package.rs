// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::FileAnalyzer;
use tree_sitter_stack_graphs::LoadError;

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
    ) -> Result<(), tree_sitter_stack_graphs::LoadError> {
        // read globals
        let proj_name = globals.get(crate::PROJECT_NAME_VAR).map(String::as_str);

        // parse source
        let npm_pkg: NpmPackage =
            serde_json::from_str(source).map_err(|_| LoadError::ParseError)?;

        // root node
        let root = StackGraph::root_node();

        // project scope
        let proj_scope = if let Some(proj_name) = proj_name {
            let proj_scope_id = graph.new_node_id(file);
            let proj_scope = graph.add_scope_node(proj_scope_id, false).unwrap();
            add_debug_name(graph, proj_scope, "npm_package.proj_scope");

            // project definition
            let proj_def = add_ns_pop(
                graph,
                file,
                root,
                PROJ_NS,
                proj_name,
                "npm_package.proj_def",
            );
            add_edge(graph, proj_def, proj_scope, 0);

            // project reference
            let proj_ref = add_ns_push(
                graph,
                file,
                root,
                PROJ_NS,
                proj_name,
                "npm_package.proj_ref",
            );
            add_edge(graph, proj_scope, proj_ref, 0);

            proj_scope
        } else {
            root
        };

        // package definition
        let pkg_def = add_module_pops(
            graph,
            file,
            NON_REL_M_NS,
            Path::new(&npm_pkg.name),
            root,
            "npm_package.pkg_def",
        );
        let pkg_ref = add_push(graph, file, proj_scope, PKG_M_NS, "npm_package.pkg_ref");
        add_edge(graph, pkg_def, pkg_ref, 0);

        // dependencies (package references)
        for (i, (pkg_name, _)) in npm_pkg.dependencies.iter().enumerate() {
            let pkg_def = add_module_pops(
                graph,
                file,
                NON_REL_M_NS,
                Path::new(&pkg_name),
                proj_scope,
                &format!("npm_package.dep[{}]", i),
            );
            let pkg_ref = add_module_pushes(
                graph,
                file,
                NON_REL_M_NS,
                Path::new(&pkg_name),
                root,
                &format!("npm_package.dep[{}]", i),
            );
            add_edge(graph, pkg_def, pkg_ref, 0);
        }

        Ok(())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmPackage {
    pub name: String,
    #[serde(default)]
    pub dependencies: HashMap<String, serde_json::Value>,
}
