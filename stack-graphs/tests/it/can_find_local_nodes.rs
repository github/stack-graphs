// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use stack_graphs::NoCancellation;

use crate::test_graphs;

fn check_local_nodes(graph: &StackGraph, file: &str, expected_local_nodes: &[&str]) {
    let file = graph.get_file_unchecked(file);
    let mut partials = PartialPaths::new();
    let mut database = Database::new();
    partials
        .find_all_partial_paths_in_file(graph, file, &NoCancellation, |graph, partials, path| {
            database.add_partial_path(graph, partials, path);
        })
        .expect("should never be cancelled");

    let mut results = BTreeSet::new();
    database.find_local_nodes();
    for handle in graph.iter_nodes() {
        if database.node_is_local(handle) {
            results.insert(graph[handle].display(graph).to_string());
        }
    }

    let expected_local_nodes = expected_local_nodes
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_local_nodes, results);
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
