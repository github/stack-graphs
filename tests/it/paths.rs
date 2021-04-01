// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::StackGraph;
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
