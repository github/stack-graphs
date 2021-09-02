// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

use stack_graphs::arena::Handle;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;

use crate::test_graphs;

fn check_node_partial_paths(
    graph: &mut StackGraph,
    id: (&str, u32),
    expected_partial_paths: &[&str],
) {
    let file = graph.get_file_unchecked(id.0);
    let id = NodeID::new_in_file(file, id.1);
    let node = graph.node_for_id(id).expect("Cannot find node");
    let mut partials = PartialPaths::new();
    let mut database = Database::new();
    partials.find_all_partial_paths_in_file(graph, file, |graph, partials, path| {
        if !path.is_complete_as_possible(graph) {
            return;
        }
        if !path.is_productive(partials) {
            return;
        }
        database.add_partial_path(graph, partials, path);
    });

    let mut results = Vec::<Handle<PartialPath>>::new();
    database.find_candidate_partial_paths_to_end_node(graph, &mut partials, node, &mut results);

    let actual_partial_paths = results
        .into_iter()
        .map(|path| database[path].display(graph, &mut partials).to_string())
        .collect::<HashSet<_>>();
    let expected_partial_paths = expected_partial_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<HashSet<_>>();
    assert_eq!(actual_partial_paths, expected_partial_paths);
}

#[test]
fn class_field_through_function_parameter() {
    let mut graph = test_graphs::class_field_through_function_parameter::new();
    check_node_partial_paths(
        &mut graph,
        ("main.py", 0),
        &["<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("a.py", 14),
        &[
            "<%1> () [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
            "<a.foo()/($2),%1> ($1) [root] -> [a.py(14) definition x] <%1> ()",
        ],
    );
    check_node_partial_paths(
        &mut graph,
        ("b.py", 8),
        &[
            "<b.A.bar,%1> ($1) [root] -> [b.py(8) definition bar] <%1> ($1)",
            "<b.A()/($2).bar,%1> ($1) [root] -> [b.py(8) definition bar] <%1> ($2)",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let mut graph = test_graphs::cyclic_imports_python::new();
    check_node_partial_paths(
        &mut graph,
        ("main.py", 0),
        &["<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("a.py", 0),
        &["<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("b.py", 6),
        &["<b.foo,%1> ($1) [root] -> [b.py(6) definition foo] <%1> ($1)"],
    );
}

#[test]
fn cyclic_imports_rust() {
    let mut graph = test_graphs::cyclic_imports_rust::new();
    check_node_partial_paths(
        &mut graph,
        ("test.rs", 201),
        &[
            "<%1> () [test.rs(103) reference a] -> [test.rs(201) definition a] <%1> ()",
            "<%1> () [test.rs(307) reference a] -> [test.rs(201) definition a] <%1> ()",
        ],
    );
    check_node_partial_paths(
        &mut graph,
        ("test.rs", 304),
        &["<%1> () [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <%1> ()"],
    );
}

#[test]
fn sequenced_import_star() {
    let mut graph = test_graphs::sequenced_import_star::new();
    check_node_partial_paths(
        &mut graph,
        ("main.py", 0),
        &["<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("a.py", 0),
        &["<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)"],
    );
    check_node_partial_paths(
        &mut graph,
        ("b.py", 5),
        &["<b.foo,%1> ($1) [root] -> [b.py(5) definition foo] <%1> ($1)"],
    );
}
