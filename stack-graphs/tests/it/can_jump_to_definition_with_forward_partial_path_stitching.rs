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
use stack_graphs::stitching::DatabaseCandidates;
use stack_graphs::stitching::ForwardPartialPathStitcher;
use stack_graphs::stitching::StitcherConfig;
use stack_graphs::NoCancellation;

use crate::test_graphs;

fn check_jump_to_definition(graph: &StackGraph, expected_partial_paths: &[&str]) {
    let mut partials = PartialPaths::new();
    let mut db = Database::new();

    // Generate partial paths for everything in the database.
    for file in graph.iter_files() {
        ForwardPartialPathStitcher::find_minimal_partial_path_set_in_file(
            graph,
            &mut partials,
            file,
            &StitcherConfig::default(),
            &NoCancellation,
            |graph, partials, path| {
                db.add_partial_path(graph, partials, path.clone());
            },
        )
        .expect("should never be cancelled");
    }

    let references = graph
        .iter_nodes()
        .filter(|handle| graph[*handle].is_reference());
    let mut complete_partial_paths = Vec::new();
    ForwardPartialPathStitcher::find_all_complete_partial_paths(
        &mut DatabaseCandidates::new(graph, &mut partials, &mut db),
        references,
        &StitcherConfig::default(),
        &NoCancellation,
        |_, _, p| {
            complete_partial_paths.push(p.clone());
        },
    )
    .expect("should never be cancelled");
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
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in import statement
            "<> () [main.py(17) reference a] -> [a.py(0) definition a] <> ()",
            // reference to `b` in import statement
            "<> () [main.py(15) reference b] -> [b.py(0) definition b] <> ()",
            // reference to `foo` in function call resolves to function definition
            "<> () [main.py(13) reference foo] -> [a.py(5) definition foo] <> ()",
            // reference to `A` as function parameter resolves to class definition
            "<> () [main.py(9) reference A] -> [b.py(5) definition A] <> ()",
            // reference to `bar` on result flows through body of `foo` to find `A.bar`
            "<> () [main.py(10) reference bar] -> [b.py(8) definition bar] <> ()",
            // reference to `x` in function body resolves to formal parameter
            "<> () [a.py(8) reference x] -> [a.py(14) definition x] <> ()",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in import statement
            "<> () [main.py(8) reference a] -> [a.py(0) definition a] <> ()",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "<> () [main.py(6) reference foo] -> [b.py(6) definition foo] <> ()",
            // reference to `b` in import statement
            "<> () [a.py(6) reference b] -> [b.py(0) definition b] <> ()",
            // reference to `a` in import statement
            "<> () [b.py(8) reference a] -> [a.py(0) definition a] <> ()",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in `a::FOO` resolves to module definition
            "<> () [test.rs(103) reference a] -> [test.rs(201) definition a] <> ()",
            // reference to `a::FOO` in `main` can resolve either to `a::BAR` or `b::FOO`
            "<> () [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <> ()",
            "<> () [test.rs(101) reference FOO] -> [test.rs(204) definition BAR] <> ()",
            // reference to `b` in use statement resolves to module definition
            "<> () [test.rs(206) reference b] -> [test.rs(301) definition b] <> ()",
            // reference to `a` in use statement resolves to module definition
            "<> () [test.rs(307) reference a] -> [test.rs(201) definition a] <> ()",
            // reference to `BAR` in module `b` can _only_ resolve to `a::BAR`
            "<> () [test.rs(305) reference BAR] -> [test.rs(204) definition BAR] <> ()",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in import statement
            "<> () [main.py(8) reference a] -> [a.py(0) definition a] <> ()",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "<> () [main.py(6) reference foo] -> [b.py(5) definition foo] <> ()",
            // reference to `b` in import statement
            "<> () [a.py(6) reference b] -> [b.py(0) definition b] <> ()",
        ],
    );
}
