// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;
use stack_graphs::c::sg_partial_path_arena_find_partial_paths_in_file;
use stack_graphs::c::sg_partial_path_arena_free;
use stack_graphs::c::sg_partial_path_arena_new;
use stack_graphs::c::sg_partial_path_database_add_partial_paths;
use stack_graphs::c::sg_partial_path_database_find_local_nodes;
use stack_graphs::c::sg_partial_path_database_free;
use stack_graphs::c::sg_partial_path_database_local_nodes;
use stack_graphs::c::sg_partial_path_database_new;
use stack_graphs::c::sg_partial_path_handle;
use stack_graphs::c::sg_partial_path_list_count;
use stack_graphs::c::sg_partial_path_list_free;
use stack_graphs::c::sg_partial_path_list_new;
use stack_graphs::c::sg_partial_path_list_paths;
use stack_graphs::c::sg_stack_graph_nodes;
use stack_graphs::c::SG_ARENA_CHUNK_SIZE;
use stack_graphs::graph::Node;

use crate::c::test_graph::TestGraph;
use crate::test_graphs;

fn index_chunked<'a, T>(chunks: *const *const T, index: usize) -> &'a T {
    let chunk_index = index / SG_ARENA_CHUNK_SIZE;
    let item_index = index % SG_ARENA_CHUNK_SIZE;
    let chunks = unsafe { std::slice::from_raw_parts(chunks, chunk_index + 1) };
    let chunk = chunks[chunk_index];
    let items = unsafe { std::slice::from_raw_parts(chunk, item_index + 1) };
    &items[item_index]
}

fn check_local_nodes(graph: &TestGraph, file: &str, expected_local_nodes: &[&str]) {
    let rust_graph = unsafe { &(*graph.graph).inner };
    let file = rust_graph.get_file_unchecked(file);

    let partials = sg_partial_path_arena_new();
    let path_list = sg_partial_path_list_new();
    sg_partial_path_arena_find_partial_paths_in_file(
        graph.graph,
        partials,
        file.as_u32(),
        path_list,
    );

    let db = sg_partial_path_database_new();
    let path_ptr = sg_partial_path_list_paths(path_list);
    let path_count = sg_partial_path_list_count(path_list);
    let mut out: Vec<sg_partial_path_handle> = vec![0; path_count];
    sg_partial_path_database_add_partial_paths(
        graph.graph,
        partials,
        db,
        path_count,
        path_ptr,
        out.as_mut_ptr(),
    );

    sg_partial_path_database_find_local_nodes(db);
    let local_nodes = sg_partial_path_database_local_nodes(db);
    let local_nodes =
        unsafe { std::slice::from_raw_parts(local_nodes.elements, local_nodes.length) };
    fn get_is_local(local_nodes: &[u32], index: usize) -> bool {
        let element_index = index / 32;
        if element_index >= local_nodes.len() {
            return false;
        }
        let bit_index = index % 32;
        let bit_mask = 1 << bit_index;
        (local_nodes[element_index] & bit_mask) != 0
    }

    let nodes = sg_stack_graph_nodes(graph.graph);
    let (nodes, node_count) = (nodes.nodes as *const *const Node, nodes.count);
    let results = (1..node_count)
        .into_iter()
        .filter(|idx| get_is_local(&local_nodes, *idx))
        .map(|idx| index_chunked(nodes, idx).display(rust_graph).to_string())
        .collect::<BTreeSet<_>>();

    let expected_local_nodes = expected_local_nodes
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_local_nodes, results);

    sg_partial_path_database_free(db);
    sg_partial_path_list_free(path_list);
    sg_partial_path_arena_free(partials);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_local_nodes(&graph, "main.py", &[]);
    check_local_nodes(
        &graph,
        "a.py",
        &[
            "[a.py(8) reference x]", //
        ],
    );
    check_local_nodes(&graph, "b.py", &[]);
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_local_nodes(&graph, "main.py", &[]);
    check_local_nodes(&graph, "a.py", &[]);
    check_local_nodes(&graph, "b.py", &[]);
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_local_nodes(
        &graph,
        "test.rs",
        // NOTE: Because everything in this example is local to one file, there aren't any partial
        // paths involving the root node.
        &[
            "[test.rs(101) reference FOO]",
            "[test.rs(103) reference a]",
            "[test.rs(201) definition a]",
            "[test.rs(204) definition BAR]",
            "[test.rs(206) reference b]",
            "[test.rs(301) definition b]",
            "[test.rs(304) definition FOO]",
            "[test.rs(305) reference BAR]",
            "[test.rs(307) reference a]",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_local_nodes(&graph, "main.py", &[]);
    check_local_nodes(&graph, "a.py", &[]);
    check_local_nodes(&graph, "b.py", &[]);
}
