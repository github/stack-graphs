// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

use maplit::hashset;
use stack_graphs::graph::StackGraph;

use crate::test_graphs;
use crate::test_graphs::CreateStackGraph;

#[test]
fn can_create_symbols() {
    let mut graph = StackGraph::new();
    let a1 = graph.add_symbol("a");
    let a2 = graph.add_symbol("a");
    let b = graph.add_symbol("b");
    let c = graph.add_symbol("c");
    let empty1 = graph.add_symbol("");
    // The content of each symbol be comparable
    assert_eq!(graph[a1], graph[a2]);
    assert_ne!(graph[a1], graph[b]);
    assert_ne!(graph[a1], graph[c]);
    assert_ne!(graph[a2], graph[b]);
    assert_ne!(graph[a2], graph[c]);
    assert_ne!(graph[b], graph[c]);
    assert_ne!(graph[empty1], graph[a1]);
    // and because we deduplicate symbols, the handles should be comparable too.
    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert_ne!(a1, c);
    assert_ne!(a2, b);
    assert_ne!(a2, c);
    assert_ne!(b, c);
    assert_ne!(empty1, a1);
}

#[test]
fn can_iterate_symbols() {
    let mut graph = StackGraph::new();
    graph.add_symbol("a");
    graph.add_symbol("b");
    graph.add_symbol("c");
    // We should get all of the symbols that we've created — though there's no guarantee in which
    // order they'll come out of the iterator.
    let symbols = graph
        .iter_symbols()
        .map(|symbol| &graph[symbol])
        .collect::<HashSet<_>>();
    assert_eq!(symbols, hashset! {"a", "b", "c"});
}

#[test]
fn can_display_symbols() {
    let mut graph = StackGraph::new();
    graph.add_symbol("a");
    graph.add_symbol("b");
    graph.add_symbol("c");
    let mut symbols = graph
        .iter_symbols()
        .map(|symbol| symbol.display(&graph).to_string())
        .collect::<Vec<_>>();
    symbols.sort();
    assert_eq!(symbols, vec!["a", "b", "c"]);
}

#[test]
fn can_create_strings() {
    let mut graph = StackGraph::new();
    let a1 = graph.add_string("a");
    let a2 = graph.add_string("a");
    let b = graph.add_string("b");
    let c = graph.add_string("c");
    let empty1 = graph.add_string("");
    // The content of each string be comparable
    assert_eq!(graph[a1], graph[a2]);
    assert_ne!(graph[a1], graph[b]);
    assert_ne!(graph[a1], graph[c]);
    assert_ne!(graph[a2], graph[b]);
    assert_ne!(graph[a2], graph[c]);
    assert_ne!(graph[b], graph[c]);
    assert_ne!(graph[empty1], graph[a1]);
    // and because we deduplicate strings, the handles should be comparable too.
    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert_ne!(a1, c);
    assert_ne!(a2, b);
    assert_ne!(a2, c);
    assert_ne!(b, c);
    assert_ne!(empty1, a1);
}

#[test]
fn can_iterate_strings() {
    let mut graph = StackGraph::new();
    graph.add_string("a");
    graph.add_string("b");
    graph.add_string("c");
    // We should get all of the strings that we've created — though there's no guarantee in which
    // order they'll come out of the iterator.
    let strings = graph
        .iter_strings()
        .map(|string| &graph[string])
        .collect::<HashSet<_>>();
    assert_eq!(strings, hashset! {"a", "b", "c"});
}

#[test]
fn can_display_strings() {
    let mut graph = StackGraph::new();
    graph.add_string("a");
    graph.add_string("b");
    graph.add_string("c");
    let mut strings = graph
        .iter_strings()
        .map(|string| string.display(&graph).to_string())
        .collect::<Vec<_>>();
    strings.sort();
    assert_eq!(strings, vec!["a", "b", "c"]);
}

#[test]
fn can_iterate_nodes() {
    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file("test.py");
    let h1 = graph.internal_scope(file, 0);
    let h2 = graph.internal_scope(file, 1);
    let h3 = graph.internal_scope(file, 2);
    let handles = graph.iter_nodes().collect::<HashSet<_>>();
    assert_eq!(
        handles,
        hashset! {graph.root_node(), graph.jump_to_node(), h1, h2, h3}
    );
}

#[test]
fn can_add_and_remove_edges() {
    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file("test.py");
    let h1 = graph.internal_scope(file, 0);
    let h2 = graph.internal_scope(file, 1);
    let h3 = graph.internal_scope(file, 2);
    let h4 = graph.internal_scope(file, 3);
    graph.add_edge(h1, h2, 0);
    graph.add_edge(h1, h3, 0);
    graph.add_edge(h1, h4, 0);
    // If you try to overwrite an edge, the original edge takes precedence.
    graph.add_edge(h1, h3, 1);
    assert_eq!(
        graph
            .outgoing_edges(h1)
            .map(|edge| (edge.sink, edge.precedence))
            .collect::<HashSet<_>>(),
        hashset! { (h2, 0), (h3, 0), (h4, 0) }
    );
}

#[test]
fn singleton_nodes_have_correct_ids() {
    let graph = StackGraph::new();
    let root_handle = StackGraph::root_node();
    let root = &graph[root_handle];
    assert!(root.is_root());
    assert!(root.id().is_root());
    assert_eq!(root.display(&graph).to_string(), "[root]");
    assert_eq!(root.id().display(&graph).to_string(), "[root]");
}

#[test]
fn can_add_graph_to_empty_graph() {
    let mut graph = StackGraph::new();
    let other = test_graphs::simple::new();
    graph.add_from_graph(&other).expect("Adding graph failed");

    for other_file in other.iter_files() {
        let file = graph
            .get_file(other[other_file].name())
            .expect("Missing file");
        assert_eq!(
            graph.nodes_for_file(file).count(),
            other.nodes_for_file(other_file).count()
        );
        assert_eq!(
            graph
                .nodes_for_file(file)
                .map(|n| graph.outgoing_edges(n).count())
                .sum::<usize>(),
            other
                .nodes_for_file(other_file)
                .map(|n| graph.outgoing_edges(n).count())
                .sum::<usize>()
        );
    }
}
