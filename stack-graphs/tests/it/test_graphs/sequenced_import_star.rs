// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::test_graphs::CreateStackGraph;

/// A stack graph containing:
///
/// ``` python
/// # main.py
/// from a import *
/// print(foo)
/// ```
///
/// ``` python
/// # a.py
/// from b import *
/// ```
///
/// ``` python
/// # b.py
/// foo = 2
/// ```
pub fn new<T>() -> T
where
    T: CreateStackGraph + Default,
{
    let mut graph = T::default();
    let root = graph.root_node();
    let sym_dot = graph.symbol(".");
    let sym_main = graph.symbol("__main__");
    let sym_a = graph.symbol("a");
    let sym_b = graph.symbol("b");
    let sym_foo = graph.symbol("foo");

    let main_file = graph.file("main.py");
    let main = graph.definition(main_file, 0, sym_main);
    let main_dot_1 = graph.pop_symbol(main_file, 1, sym_dot);
    let main_bottom_2 = graph.internal_scope(main_file, 2);
    let main_3 = graph.internal_scope(main_file, 3);
    let main_4 = graph.internal_scope(main_file, 4);
    let main_top_5 = graph.internal_scope(main_file, 5);
    let main_foo = graph.reference(main_file, 6, sym_foo);
    let main_dot_7 = graph.push_symbol(main_file, 7, sym_dot);
    let main_a = graph.reference(main_file, 8, sym_a);
    graph.edge(root, main);
    graph.edge(main, main_dot_1);
    graph.edge(main_dot_1, main_bottom_2);
    graph.edge(main_bottom_2, main_3);
    graph.edge(main_foo, main_3);
    graph.edge(main_3, main_4);
    graph.edge(main_4, main_dot_7);
    graph.edge(main_dot_7, main_a);
    graph.edge(main_a, root);
    graph.edge(main_4, main_top_5);

    let a_file = graph.file("a.py");
    let a = graph.definition(a_file, 0, sym_a);
    let a_dot_1 = graph.pop_symbol(a_file, 1, sym_dot);
    let a_bottom_2 = graph.internal_scope(a_file, 2);
    let a_3 = graph.internal_scope(a_file, 3);
    let a_top_4 = graph.internal_scope(a_file, 4);
    let a_dot_5 = graph.push_symbol(a_file, 5, sym_dot);
    let a_b = graph.reference(a_file, 6, sym_b);
    graph.edge(root, a);
    graph.edge(a, a_dot_1);
    graph.edge(a_dot_1, a_bottom_2);
    graph.edge(a_bottom_2, a_3);
    graph.edge(a_3, a_dot_5);
    graph.edge(a_dot_5, a_b);
    graph.edge(a_b, root);
    graph.edge(a_3, a_top_4);

    let b_file = graph.file("b.py");
    let b = graph.definition(b_file, 0, sym_b);
    let b_dot_1 = graph.pop_symbol(b_file, 1, sym_dot);
    let b_bottom_2 = graph.internal_scope(b_file, 2);
    let b_3 = graph.internal_scope(b_file, 3);
    let b_top_4 = graph.internal_scope(b_file, 4);
    let b_foo = graph.definition(b_file, 5, sym_foo);
    graph.edge(root, b);
    graph.edge(b, b_dot_1);
    graph.edge(b_dot_1, b_bottom_2);
    graph.edge(b_bottom_2, b_3);
    graph.edge(b_3, b_foo);
    graph.edge(b_3, b_top_4);

    graph
}
