// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use itertools::Itertools;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::storage::SQLiteWriter;
use stack_graphs::NoCancellation;

use crate::util::create_partial_path_and_edges;
use crate::util::create_pop_symbol_node;
use crate::util::create_push_symbol_node;

fn test_foo_bar_root_candidate_paths(symbols: &[&str], variable: bool) -> usize {
    let mut reader = {
        let mut writer = SQLiteWriter::open_in_memory().unwrap();

        let mut graph = StackGraph::new();
        let file = graph.add_file("test1").unwrap();
        let mut partials = PartialPaths::new();

        let r = StackGraph::root_node();
        let foo = create_pop_symbol_node(&mut graph, file, "foo", true);
        let bar = create_pop_symbol_node(&mut graph, file, "bar", true);

        let path_with_variable =
            create_partial_path_and_edges(&mut graph, &mut partials, &[r, foo, bar]).unwrap();

        let mut path_without_variable = path_with_variable.clone();
        path_without_variable.eliminate_precondition_stack_variables(&mut partials);

        writer
            .store_result_for_file(
                &graph,
                file,
                "",
                &mut partials,
                vec![&path_with_variable, &path_without_variable],
            )
            .unwrap();

        writer.into_reader()
    };

    {
        let (graph, partials, _) = reader.get();
        let file = graph.add_file("test2").unwrap();

        let r = StackGraph::root_node();
        let refs = symbols
            .into_iter()
            .map(|r| create_push_symbol_node(graph, file, *r, true))
            .chain(std::iter::once(r))
            .collect_vec();
        let mut path = create_partial_path_and_edges(graph, partials, &refs).unwrap();
        if !variable {
            path.eliminate_precondition_stack_variables(partials);
        }

        reader
            .load_partial_path_extensions(&path, &NoCancellation)
            .unwrap();

        let (graph, partials, db) = reader.get();
        let mut results = Vec::new();
        db.find_candidate_partial_paths_from_root(
            graph,
            partials,
            Some(path.symbol_stack_postcondition),
            &mut results,
        );

        results.len()
    }
}

#[test]
fn find_candidates_for_exact_symbol_stack_with_variable() {
    // <"foo","bar",%2> ~ <"foo","bar",%1> | yes, %2 = %1
    // <"foo","bar",%2> ~ <"foo","bar">    | yes, %2 = <>
    let results = test_foo_bar_root_candidate_paths(&["bar", "foo"], true);
    assert_eq!(2, results);
}

#[test]
fn find_candidates_for_exact_symbol_stack_without_variable() {
    // <"foo","bar"> ~ <"foo","bar",%1> | yes, %1 = <>
    // <"foo","bar"> ~ <"foo","bar">    | yes
    let results = test_foo_bar_root_candidate_paths(&["bar", "foo"], false);
    assert_eq!(2, results);
}

#[test]
fn find_candidates_for_longer_symbol_stack_with_variable() {
    // <"foo","bar","quz",%2> ~ <"foo","bar",%1> | yes, %1 = <"quz",%2>
    // <"foo","bar","quz",%2> ~ <"foo","bar">    | no
    let results = test_foo_bar_root_candidate_paths(&["quz", "bar", "foo"], true);
    assert_eq!(1, results);
}

#[test]
fn find_candidates_for_longer_symbol_stack_without_variable() {
    // <"foo","bar","quz"> ~ <"foo","bar",%1> | yes, %1 = <"quz">
    // <"foo","bar","quz"> ~ <"foo","bar">    | no
    let results = test_foo_bar_root_candidate_paths(&["quz", "bar", "foo"], false);
    assert_eq!(1, results);
}

#[test]
fn find_candidates_for_shorter_symbol_stack_with_variable() {
    // <"foo",%2> ~ <"foo","bar",%1> | yes, %2 = <"bar",%1>
    // <"foo",%2> ~ <"foo","bar">    | yes, %2 = <"bar">
    let results = test_foo_bar_root_candidate_paths(&["foo"], true);
    assert_eq!(2, results);
}

#[test]
fn find_candidates_for_shorter_symbol_stack_without_variable() {
    // <"foo"> ~ <"foo","bar",%1> | no
    // <"foo"> ~ <"foo","bar">    | no
    let results = test_foo_bar_root_candidate_paths(&["foo"], false);
    assert_eq!(0, results);
}
