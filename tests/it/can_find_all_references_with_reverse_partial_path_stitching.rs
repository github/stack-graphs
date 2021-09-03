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
use stack_graphs::stitching::ReversePartialPathStitcher;

use crate::test_graphs;

fn check_find_all_references(graph: &StackGraph, expected_partial_paths: &[&str]) {
    let mut partials = PartialPaths::new();
    let mut db = Database::new();

    // Generate partial paths for everything in the database.
    for file in graph.iter_files() {
        partials.find_all_partial_paths_in_file(graph, file, |graph, partials, path| {
            if !path.is_complete_as_possible(graph) {
                return;
            }
            if !path.is_productive(partials) {
                return;
            }
            db.add_partial_path(graph, partials, path);
        });
    }

    let definitions = graph
        .iter_nodes()
        .filter(|handle| graph[*handle].is_definition());
    let complete_partial_paths = ReversePartialPathStitcher::find_all_complete_partial_paths(
        graph,
        &mut partials,
        &mut db,
        definitions,
    );
    let results = complete_partial_paths
        .into_iter()
        .map(|partial_path| partial_path.display(graph, &mut partials).to_string())
        .collect::<BTreeSet<_>>();

    let expected_partial_paths = expected_partial_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_partial_paths, results);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_find_all_references(
        &graph,
        &[
            // reference to `a` in import statement
            "<%2> () [main.py(17) reference a] -> [a.py(0) definition a] <%2> ()",
            // reference to `b` in import statement
            "<%2> () [main.py(15) reference b] -> [b.py(0) definition b] <%2> ()",
            // reference to `foo` in function call resolves to function definition
            "<%2> () [main.py(13) reference foo] -> [a.py(5) definition foo] <%2> ()",
            // reference to `A` as function parameter resolves to class definition
            "<%2> () [main.py(9) reference A] -> [b.py(5) definition A] <%2> ()",
            // reference to `bar` on result flows through body of `foo` to find `A.bar`
            "<%2> () [main.py(10) reference bar] -> [b.py(8) definition bar] <%2> ()",
            // reference to `x` in function body resolves to formal parameter
            "<%1> () [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_find_all_references(
        &graph,
        &[
            // reference to `a` in import statement
            "<%2> () [main.py(8) reference a] -> [a.py(0) definition a] <%2> ()",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "<%2> () [main.py(6) reference foo] -> [b.py(6) definition foo] <%2> ()",
            // reference to `b` in import statement
            "<%2> () [a.py(6) reference b] -> [b.py(0) definition b] <%2> ()",
            // reference to `a` in import statement
            "<%2> () [b.py(8) reference a] -> [a.py(0) definition a] <%2> ()",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_find_all_references(
        &graph,
        &[
            // reference to `a` in `a::FOO` resolves to module definition
            "<%1> () [test.rs(103) reference a] -> [test.rs(201) definition a] <%1> ()",
            // reference to `a::FOO` in `main` can resolve either to `a::BAR` or `b::FOO`
            "<%1> () [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <%1> ()",
            "<%1> () [test.rs(101) reference FOO] -> [test.rs(204) definition BAR] <%1> ()",
            // reference to `b` in use statement resolves to module definition
            "<%1> () [test.rs(206) reference b] -> [test.rs(301) definition b] <%1> ()",
            // reference to `a` in use statement resolves to module definition
            "<%1> () [test.rs(307) reference a] -> [test.rs(201) definition a] <%1> ()",
            // reference to `BAR` in module `b` can _only_ resolve to `a::BAR`
            "<%1> () [test.rs(305) reference BAR] -> [test.rs(204) definition BAR] <%1> ()",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_find_all_references(
        &graph,
        &[
            // reference to `a` in import statement
            "<%2> () [main.py(8) reference a] -> [a.py(0) definition a] <%2> ()",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "<%2> () [main.py(6) reference foo] -> [b.py(5) definition foo] <%2> ()",
            // reference to `b` in import statement
            "<%2> () [a.py(6) reference b] -> [b.py(0) definition b] <%2> ()",
        ],
    );
}
