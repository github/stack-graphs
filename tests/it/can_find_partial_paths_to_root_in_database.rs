// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

use controlled_option::ControlledOption;
use stack_graphs::arena::Handle;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialPaths;
use stack_graphs::paths::Paths;
use stack_graphs::paths::ScopedSymbol;
use stack_graphs::paths::SymbolStack;
use stack_graphs::stitching::Database;
use stack_graphs::stitching::SymbolStackKey;

use crate::test_graphs;

fn check_root_partial_paths(
    graph: &mut StackGraph,
    file: &str,
    precondition: &[&str],
    expected_partial_paths: &[&str],
) {
    let file = graph.get_file_unchecked(file);
    let mut paths = Paths::new();
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

    let mut symbol_stack = SymbolStack::empty();
    for symbol in precondition.iter().rev() {
        let symbol = graph.add_symbol(symbol);
        let scoped_symbol = ScopedSymbol {
            symbol,
            scopes: ControlledOption::none(),
        };
        symbol_stack.push_front(&mut paths, scoped_symbol);
    }

    let mut results = Vec::<Handle<PartialPath>>::new();
    let key = SymbolStackKey::from_symbol_stack(&mut paths, &mut database, symbol_stack);
    database.find_candidate_partial_paths_to_root(graph, &mut partials, key, &mut results);

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
    check_root_partial_paths(
        &mut graph,
        "main.py",
        &["a", ".", "foo"],
        &[
            "<%1> () [main.py(17) reference a] -> [root] <a,%1> ()",
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            "<%1> () [main.py(13) reference foo] -> [root] <a.foo,%1> ()",
        ],
    );
    check_root_partial_paths(
        &mut graph, //
        "a.py",
        &["b", ".", "foo"],
        // There are no partial paths in this file that end at the root node, regardless of their
        // symbol stacks.
        &[],
    );
    check_root_partial_paths(
        &mut graph, //
        "b.py",
        &["a", ".", "foo"],
        // There are no partial paths in this file that end at the root node, regardless of their
        // symbol stacks.
        &[],
    );
}

#[test]
fn cyclic_imports_python() {
    let mut graph = test_graphs::cyclic_imports_python::new();
    check_root_partial_paths(
        &mut graph,
        "main.py",
        &["a", ".", "foo"],
        &[
            "<%1> () [main.py(8) reference a] -> [root] <a,%1> ()",
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            "<%1> () [main.py(6) reference foo] -> [root] <a.foo,%1> ()",
        ],
    );
    check_root_partial_paths(
        &mut graph,
        "a.py",
        &["b", ".", "foo"],
        &[
            "<%1> () [a.py(6) reference b] -> [root] <b,%1> ()",
            "<a.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
        ],
    );
    check_root_partial_paths(
        &mut graph,
        "b.py",
        &["a", ".", "foo"],
        &[
            "<%1> () [b.py(8) reference a] -> [root] <a,%1> ()",
            "<b.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let mut graph = test_graphs::cyclic_imports_rust::new();
    check_root_partial_paths(
        &mut graph,
        "test.rs",
        &[],
        // NOTE: Because everything in this example is local to one file, there aren't any partial
        // paths involving the root node.
        &[],
    );
}

#[test]
fn sequenced_import_star() {
    let mut graph = test_graphs::sequenced_import_star::new();
    check_root_partial_paths(
        &mut graph,
        "main.py",
        &["a", ".", "foo"],
        &[
            "<%1> () [main.py(8) reference a] -> [root] <a,%1> ()",
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            "<%1> () [main.py(6) reference foo] -> [root] <a.foo,%1> ()",
        ],
    );
    check_root_partial_paths(
        &mut graph,
        "a.py",
        &["b", ".", "foo"],
        &[
            "<%1> () [a.py(6) reference b] -> [root] <b,%1> ()",
            "<a.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
        ],
    );
    check_root_partial_paths(
        &mut graph, //
        "b.py",
        &["a", ".", "foo"],
        &[],
    );
}
