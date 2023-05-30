// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::Node;
use std::path::Component;
use std::path::Path;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;

pub const M_NS: &str = "%M";
pub const NON_REL_M_NS: &str = "%NonRelM";
pub const PROJ_NS: &str = "%Proj";
pub const REL_M_NS: &str = "%RelM";
pub const PKG_M_NS: &str = "%PkgM";

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

pub fn add_ns_pop(
    graph: &mut StackGraph,
    file: Handle<File>,
    from: Handle<Node>,
    ns: &str,
    name: &str,
    debug_prefix: &str,
) -> Handle<Node> {
    let ns_node = add_pop(graph, file, from, ns, &format!("{}.ns", debug_prefix));
    let pop_node = add_pop(graph, file, ns_node, name, debug_prefix);
    pop_node
}

pub fn add_ns_push(
    graph: &mut StackGraph,
    file: Handle<File>,
    to: Handle<Node>,
    ns: &str,
    name: &str,
    debug_prefix: &str,
) -> Handle<Node> {
    let ns_node = add_push(graph, file, to, ns, &format!("{}.ns", debug_prefix));
    let push_node = add_push(graph, file, ns_node, name, debug_prefix);
    push_node
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
    ns: &str,
    path: &Path,
    from: Handle<Node>,
    debug_prefix: &str,
) -> Handle<Node> {
    let ns_node = add_pop(graph, file, from, ns, &format!("{}.ns", debug_prefix));
    let mut node = ns_node;
    for (i, c) in path.components().enumerate() {
        match c {
            Component::Normal(name) => {
                node = add_pop(
                    graph,
                    file,
                    node,
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
    node
}

pub fn add_module_pushes(
    graph: &mut StackGraph,
    file: Handle<File>,
    ns: &str,
    path: &Path,
    to: Handle<Node>,
    debug_prefix: &str,
) -> Handle<Node> {
    let ns_node = add_push(graph, file, to, ns, &format!("{}.ns", debug_prefix));
    let mut node = ns_node;
    for (i, c) in path.components().enumerate() {
        match c {
            Component::Normal(name) => {
                node = add_push(
                    graph,
                    file,
                    node,
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
    node
}
