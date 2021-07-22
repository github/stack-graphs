// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use itertools::Itertools;
use stack_graphs::arena::Handle;
use stack_graphs::graph::Edge;
use stack_graphs::graph::File;
use stack_graphs::graph::Node;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Path;
use stack_graphs::paths::Paths;
use stack_graphs::paths::ScopeStack;
use stack_graphs::paths::ScopedSymbol;
use stack_graphs::paths::SymbolStack;

use crate::test_graphs::CreateStackGraph;

#[test]
fn can_iterate_symbol_stacks() {
    let mut graph = StackGraph::new();
    let mut paths = Paths::new();
    let sym_a = graph.add_symbol("a");
    let sym_b = graph.add_symbol("b");
    let sym_dot = graph.add_symbol(".");
    let file = graph.get_or_create_file("test.py");
    let exported = graph.exported_scope(file, 0);
    let mut scope_stack = ScopeStack::empty();
    scope_stack.push_front(&mut paths, exported);
    let mut symbol_stack = SymbolStack::empty();
    symbol_stack.push_front(
        &mut paths,
        ScopedSymbol {
            symbol: sym_b,
            scopes: Some(scope_stack),
        },
    );
    symbol_stack.push_front(
        &mut paths,
        ScopedSymbol {
            symbol: sym_dot,
            scopes: None,
        },
    );
    symbol_stack.push_front(
        &mut paths,
        ScopedSymbol {
            symbol: sym_a,
            scopes: None,
        },
    );

    let symbols = symbol_stack.iter(&paths).collect::<Vec<_>>();
    let rendered: String = symbols
        .into_iter()
        .map(|symbol| symbol.display(&graph, &mut paths).to_string())
        .collect();
    assert_eq!(rendered, "a.b/[test.py(0)]");
}

#[test]
fn can_iterate_scope_stacks() {
    let mut graph = StackGraph::new();
    let mut paths = Paths::new();
    let file = graph.get_or_create_file("test.py");
    let exported0 = graph.exported_scope(file, 0);
    let exported1 = graph.exported_scope(file, 1);
    let exported2 = graph.exported_scope(file, 2);
    let mut scope_stack = ScopeStack::empty();
    scope_stack.push_front(&mut paths, exported2);
    scope_stack.push_front(&mut paths, exported1);
    scope_stack.push_front(&mut paths, exported0);

    let rendered: String = scope_stack
        .iter(&paths)
        .map(|symbol| symbol.display(&graph).to_string())
        .collect();
    assert_eq!(
        rendered,
        "[test.py(0) exported scope][test.py(1) exported scope][test.py(2) exported scope]"
    );
}

struct ShadowingTest {
    graph: StackGraph,
    paths: Paths,
    file: Handle<File>,
}

impl ShadowingTest {
    fn new() -> ShadowingTest {
        let mut graph = StackGraph::new();
        let paths = Paths::new();
        let file = graph.get_or_create_file("test.py");
        ShadowingTest { graph, paths, file }
    }

    fn new_node(&mut self, local_id: u32) -> Handle<Node> {
        let id = NodeID::new_in_file(self.file, local_id);
        match self.graph.node_for_id(id) {
            Some(node) => node,
            None => self.graph.internal_scope(self.file, local_id),
        }
    }

    fn new_path(&mut self, start_node: u32, edges: &[(i32, u32)]) -> Path {
        let start_node = self.new_node(start_node);
        let mut path = Path::from_node(&mut self.graph, &mut self.paths, start_node).unwrap();
        let mut previous = start_node;
        for (precedence, node) in edges.iter().copied() {
            let node = self.new_node(node);
            path.append(
                &mut self.graph,
                &mut self.paths,
                Edge {
                    source: previous,
                    sink: node,
                    precedence,
                },
            )
            .unwrap();
            previous = node;
        }
        path
    }
}

#[test]
fn paths_can_shadow_other_paths() {
    let mut test = ShadowingTest::new();
    let path1 = test.new_path(1, &vec![(0, 2), (0, 3), (1, 4), (0, 5)]);
    let path2 = test.new_path(1, &vec![(0, 2), (1, 5), (0, 6)]);
    assert!(path2.shadows(&mut test.paths, &path1));
    assert!(!path1.shadows(&mut test.paths, &path2));
}

#[test]
fn path_does_not_shadow_if_source_ids_differ() {
    let mut test = ShadowingTest::new();
    let path1 = test.new_path(1, &vec![(0, 2), (1, 3), (0, 4)]);
    let path2 = test.new_path(2, &vec![(0, 3), (1, 4), (0, 5)]);
    assert!(!path1.shadows(&mut test.paths, &path2));
    assert!(!path2.shadows(&mut test.paths, &path1));
}

#[test]
fn can_remove_shadowed_paths() {
    let mut test = ShadowingTest::new();
    // path1 is shadowed by path2 because of their second edges.
    let path1 = test.new_path(1, &vec![(0, 2), (0, 3), (1, 4), (0, 5)]);
    let path2 = test.new_path(1, &vec![(0, 2), (1, 5), (0, 3)]);
    let mut paths = vec![path1.clone(), path2.clone()];
    test.paths.remove_shadowed_paths(&mut paths);
    let expected = vec![path2];
    assert!(paths
        .into_iter()
        .zip_eq(expected)
        .all(|(a, b)| a.equals(&mut test.paths, &b)));
}

#[test]
fn nonshadowed_paths_are_not_removed() {
    let mut test = ShadowingTest::new();
    let path1 = test.new_path(1, &vec![(0, 2), (1, 3), (0, 4)]);
    let path2 = test.new_path(1, &vec![(0, 5), (1, 6), (0, 7)]);
    let mut paths = vec![path1.clone(), path2.clone()];
    test.paths.remove_shadowed_paths(&mut paths);
    let expected = vec![path1, path2];
    assert!(paths
        .into_iter()
        .zip_eq(expected)
        .all(|(a, b)| a.equals(&mut test.paths, &b)));
}

#[test]
fn can_remove_shadowed_paths_for_empty_paths() {
    let mut test = ShadowingTest::new();
    let mut paths = vec![];
    test.paths.remove_shadowed_paths(&mut paths);
    let expected = vec![];
    assert!(paths
        .into_iter()
        .zip_eq(expected)
        .all(|(a, b)| a.equals(&mut test.paths, &b)));
}
