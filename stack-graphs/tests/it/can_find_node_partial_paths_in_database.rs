// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;
use stack_graphs::arena::Handle;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use stack_graphs::NoCancellation;

use crate::test_graphs;

fn check_node_partial_paths(
    graph: &mut StackGraph,
    id: (&str, u32),
    expected_partial_paths: &[&str],
) {
    let file = graph.get_file(id.0).expect("Missing file");
    let id = NodeID::new_in_file(file, id.1);
    let node = graph.node_for_id(id).expect("Cannot find node");
    let mut partials = PartialPaths::new();
    let mut db = Database::new();
    partials
        .find_minimal_partial_path_set_in_file(
            graph,
            file,
            &NoCancellation,
            |graph, partials, path| {
                db.add_partial_path(graph, partials, path);
            },
        )
        .expect("should never be cancelled");

    let mut results = Vec::<Handle<PartialPath>>::new();
    db.find_candidate_partial_paths_from_node(graph, &mut partials, node, &mut results);

    let actual_partial_paths = results
        .into_iter()
        .map(|path| db[path].display(graph, &mut partials).to_string())
        .collect::<BTreeSet<_>>();
    let expected_partial_paths = expected_partial_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        expected_partial_paths, actual_partial_paths,
        "failed in file {}",
        graph[file]
    );
}

#[test]
fn class_field_through_function_parameter() {
    let mut graph = test_graphs::class_field_through_function_parameter::new();
    check_node_partial_paths(
        &mut graph,
        ("main.py", 10),
        &[
            "<%1> ($1) [main.py(10) reference bar] -> [main.py(13) reference foo] <foo()/([main.py(7)],$1).bar,%1> ($1)",
        ],
    );
    check_node_partial_paths(
        &mut graph,
        ("a.py", 8),
        &[
            "<%1> ($1) [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
            "<%1> ($1) [a.py(8) reference x] -> [jump to scope] <0,%1> ($1)",
            "<%1> ($1) [a.py(8) reference x] -> [jump to scope] <x,%1> ($1)",
        ],
    );
    // no references in b.py
}

#[test]
fn cyclic_imports_python() {
    let mut graph = test_graphs::cyclic_imports_python::new();
    check_node_partial_paths(
        &mut graph,
        ("main.py", 6),
        &["<%1> ($1) [main.py(6) reference foo] -> [main.py(8) reference a] <a.foo,%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("a.py", 6),
        &["<%1> ($1) [a.py(6) reference b] -> [root] <b,%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("b.py", 8),
        &["<%1> ($1) [b.py(8) reference a] -> [root] <a,%1> ($1)"],
    );
}

#[test]
fn cyclic_imports_rust() {
    let mut graph = test_graphs::cyclic_imports_rust::new();
    check_node_partial_paths(
        &mut graph,
        ("test.rs", 101),
        &["<%1> ($1) [test.rs(101) reference FOO] -> [test.rs(103) reference a] <a::FOO,%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("test.rs", 305),
        &["<%1> ($1) [test.rs(305) reference BAR] -> [test.rs(307) reference a] <a::BAR,%1> ($1)"],
    );
}

#[test]
fn sequenced_import_star() {
    let mut graph = test_graphs::sequenced_import_star::new();
    check_node_partial_paths(
        &mut graph,
        ("main.py", 6),
        &["<%1> ($1) [main.py(6) reference foo] -> [main.py(8) reference a] <a.foo,%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("a.py", 6),
        &["<%1> ($1) [a.py(6) reference b] -> [root] <b,%1> ($1)"],
    );
    // no references in b.py
}
