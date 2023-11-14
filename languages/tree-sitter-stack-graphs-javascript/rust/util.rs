// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::Node;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;

pub const EXPORTS_GUARD: &str = "GUARD:EXPORTS";
pub const DEFAULT_GUARD: &str = "GUARD:DEFAULT";
pub const PKG_GUARD: &str = "GUARD:PKG";
pub const PKG_INTERNAL_GUARD: &str = "GUARD:PKG_INTERNAL";

pub fn add_debug_name(graph: &mut StackGraph, node: Handle<Node>, name: &str) {
    let key = graph.add_string("name");
    let value = graph.add_string(name);
    graph.node_debug_info_mut(node).add(key, value);
}

pub fn add_pop(
    graph: &mut StackGraph,
    file: Handle<File>,
    from: Handle<Node>,
    name: &str,
    debug_name: &str,
) -> Handle<Node> {
    let id = graph.new_node_id(file);
    let sym = graph.add_symbol(name);
    let node = graph.add_pop_symbol_node(id, sym, false).unwrap();
    graph.add_edge(from, node, 0);
    add_debug_name(graph, node, debug_name);
    node
}

pub fn add_push(
    graph: &mut StackGraph,
    file: Handle<File>,
    to: Handle<Node>,
    name: &str,
    debug_name: &str,
) -> Handle<Node> {
    let id = graph.new_node_id(file);
    let sym = graph.add_symbol(name);
    let node = graph.add_push_symbol_node(id, sym, false).unwrap();
    graph.add_edge(node, to, 0);
    add_debug_name(graph, node, debug_name);
    node
}

pub fn add_edge(graph: &mut StackGraph, from: Handle<Node>, to: Handle<Node>, precedence: i32) {
    if from == to {
        return;
    }
    graph.add_edge(from, to, precedence);
}

pub fn add_module_pops(
    graph: &mut StackGraph,
    file: Handle<File>,
    path: &Path,
    mut from: Handle<Node>,
    debug_prefix: &str,
) -> Handle<Node> {
    for (i, c) in path.components().enumerate() {
        match c {
            Component::Normal(name) => {
                from = add_pop(
                    graph,
                    file,
                    from,
                    &name.to_string_lossy(),
                    &format!("{}[{}]", debug_prefix, i),
                );
            }
            _ => {
                eprintln!(
                    "add_module_pops: expecting normalized, non-escaping, relative paths, got {}",
                    path.display()
                )
            }
        }
    }
    from
}

pub fn add_module_pushes(
    graph: &mut StackGraph,
    file: Handle<File>,
    path: &Path,
    mut to: Handle<Node>,
    debug_prefix: &str,
) -> Handle<Node> {
    for (i, c) in path.components().enumerate() {
        match c {
            Component::Normal(name) => {
                to = add_push(
                    graph,
                    file,
                    to,
                    &name.to_string_lossy(),
                    &format!("{}[{}]", debug_prefix, i),
                );
            }
            _ => {
                eprintln!(
                    "add_module_pushes: expecting normalized, non-escaping, relative paths, got {}",
                    path.display()
                )
            }
        }
    }
    to
}

pub struct NormalizedRelativePath(PathBuf);

impl NormalizedRelativePath {
    pub fn from_str(path: &str) -> Option<Self> {
        Self::from_path(Path::new(path))
    }

    /// Creates a new normalized, relative path from a path.
    pub fn from_path(path: &Path) -> Option<Self> {
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

    pub fn into_path_buf(self) -> PathBuf {
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
