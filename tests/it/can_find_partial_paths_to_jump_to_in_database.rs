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
    database.find_candidate_partial_paths_to_jump_to(graph, &mut partials, key, &mut results);

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
        &["a"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
    check_root_partial_paths(
        &mut graph,
        "a.py",
        &["0"],
        &["<a.foo()/($2),%1> ($1) [root] -> [jump to scope] <0,%1> ($2)"],
    );
    check_root_partial_paths(
        &mut graph,
        "b.py",
        &["a"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
}

#[test]
fn cyclic_imports_python() {
    let mut graph = test_graphs::cyclic_imports_python::new();
    check_root_partial_paths(
        &mut graph,
        "main.py",
        &["a"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
    check_root_partial_paths(
        &mut graph,
        "a.py",
        &["b"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
    check_root_partial_paths(
        &mut graph,
        "b.py",
        &["a"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
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
        // paths involving the jump to scope node.
        &[],
    );
}

#[test]
fn sequenced_import_star() {
    let mut graph = test_graphs::sequenced_import_star::new();
    check_root_partial_paths(
        &mut graph,
        "main.py",
        &["a"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
    check_root_partial_paths(
        &mut graph,
        "a.py",
        &["b"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
    check_root_partial_paths(
        &mut graph,
        "b.py",
        &["a"],
        // There are no partial paths in this file that end at the jump to scope node, regardless
        // of their symbol stacks.
        &[],
    );
}
