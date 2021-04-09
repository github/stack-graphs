// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::StackGraph;

#[test]
fn can_create_symbols() {
    let mut graph = StackGraph::new();
    let a1 = graph.add_symbol("a");
    let a2 = graph.add_symbol("a");
    let b = graph.add_symbol("b");
    let c = graph.add_symbol("c");
    // The content of each symbol be comparable
    assert_eq!(graph[a1], graph[a2]);
    assert_ne!(graph[a1], graph[b]);
    assert_ne!(graph[a1], graph[c]);
    assert_ne!(graph[a2], graph[b]);
    assert_ne!(graph[a2], graph[c]);
    assert_ne!(graph[b], graph[c]);
    // and because we deduplicate symbols, the handles should be comparable too.
    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert_ne!(a1, c);
    assert_ne!(a2, b);
    assert_ne!(a2, c);
    assert_ne!(b, c);
}
