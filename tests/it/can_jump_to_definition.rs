// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;

use crate::test_graphs;

fn check_jump_to_definition(graph: &StackGraph, expected_paths: &[&str]) {
    let mut paths = Paths::new();
    let mut results = HashSet::new();
    let references = graph
        .iter_nodes()
        .filter(|handle| graph[*handle].is_reference());
    paths.find_all_paths(graph, references, |graph, paths, path| {
        if path.is_complete(graph) {
            results.insert(path.display(graph, paths).to_string());
        }
    });
    let expected_paths = expected_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<HashSet<_>>();
    assert_eq!(results, expected_paths);
}

#[test]
fn class_field_through_function_parameter() {
    let fixture = test_graphs::class_field_through_function_parameter::new();
    check_jump_to_definition(
        &fixture.graph,
        &[
            // reference to `a` in import statement
            "[main.py(17) reference a] -> [a.py(0) definition a]",
            // reference to `b` in import statement
            "[main.py(15) reference b] -> [b.py(0) definition b]",
            // reference to `foo` in function call resolves to function definition
            "[main.py(13) reference foo] -> [a.py(5) definition foo]",
            // reference to `A` as function parameter resolves to class definition
            "[main.py(9) reference A] -> [b.py(5) definition A]",
            // reference to `bar` on result flows through body of `foo` to find `A.bar`
            "[main.py(10) reference bar] -> [b.py(8) definition bar]",
            // reference to `x` in function body resolves to formal parameter
            "[a.py(8) reference x] -> [a.py(14) definition x]",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let fixture = test_graphs::sequenced_import_star::new();
    check_jump_to_definition(
        &fixture.graph,
        &[
            // reference to `a` in import statement
            "[main.py(8) reference a] -> [a.py(0) definition a]",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "[main.py(6) reference foo] -> [b.py(5) definition foo]",
            // reference to `b` in import statement
            "[a.py(6) reference b] -> [b.py(0) definition b]",
        ],
    );
}
