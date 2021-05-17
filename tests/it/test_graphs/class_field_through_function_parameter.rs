// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::arena::Handle;
use stack_graphs::graph::*;

use crate::test_graphs::CreateStackGraph;

/// A stack graph containing:
///
/// ``` python
/// # main.py
/// from a import *
/// from b import *
/// print(foo(A).bar)
/// ```
///
/// ``` python
/// # a.py
/// def foo(x):
///   return x
/// ```
///
/// ``` python
/// # b.py
/// class A:
///   bar = 1
/// ```
#[allow(non_snake_case)]
pub struct ClassFieldThroughFunctionParameter {
    pub graph: StackGraph,
    // Interesting nodes in main.py
    pub main: Handle<Node>,
    pub main_A: Handle<Node>,
    pub main_a: Handle<Node>,
    pub main_b: Handle<Node>,
    pub main_bar: Handle<Node>,
    pub main_foo: Handle<Node>,
    // Interesting nodes in a.py
    pub a: Handle<Node>,
    pub a_x_def: Handle<Node>,
    pub a_x_ref: Handle<Node>,
    pub a_foo: Handle<Node>,
    // Interesting nodes in b.py
    pub b: Handle<Node>,
    pub b_A: Handle<Node>,
    pub b_bar: Handle<Node>,
}

#[allow(non_snake_case)]
pub fn new() -> ClassFieldThroughFunctionParameter {
    let mut graph = StackGraph::new();
    let root = graph.root_node();
    let jump_to = graph.jump_to_node();
    let sym_call = graph.add_symbol("()");
    let sym_dot = graph.add_symbol(".");
    let sym_zero = graph.add_symbol("0");
    let sym_main = graph.add_symbol("__main__");
    let sym_A = graph.add_symbol("A");
    let sym_a = graph.add_symbol("a");
    let sym_b = graph.add_symbol("b");
    let sym_x = graph.add_symbol("x");
    let sym_foo = graph.add_symbol("foo");
    let sym_bar = graph.add_symbol("bar");

    let main_file = graph.get_or_create_file("main.py");
    let main = graph.definition(main_file, 0, sym_main);
    let main_dot_1 = graph.pop_symbol(main_file, 1, sym_dot);
    let main_bottom_2 = graph.internal_scope(main_file, 2);
    let main_3 = graph.internal_scope(main_file, 3);
    let main_4 = graph.internal_scope(main_file, 4);
    let main_5 = graph.internal_scope(main_file, 5);
    let main_top_6 = graph.internal_scope(main_file, 6);
    let main_exported = graph.exported_scope(main_file, 7);
    let main_zero_8 = graph.pop_symbol(main_file, 8, sym_zero);
    let main_A = graph.reference(main_file, 9, sym_A);
    let main_bar = graph.reference(main_file, 10, sym_bar);
    let main_dot_11 = graph.push_symbol(main_file, 11, sym_dot);
    let main_call_12 = graph.push_scoped_symbol(main_file, 12, sym_call, main_exported);
    let main_foo = graph.reference(main_file, 13, sym_foo);
    let main_dot_14 = graph.push_symbol(main_file, 14, sym_dot);
    let main_b = graph.reference(main_file, 15, sym_b);
    let main_dot_16 = graph.push_symbol(main_file, 16, sym_dot);
    let main_a = graph.reference(main_file, 17, sym_a);
    graph.edge(root, main);
    graph.edge(main, main_dot_1);
    graph.edge(main_dot_1, main_bottom_2);
    graph.edge(main_bottom_2, main_3);
    graph.edge(main_exported, main_zero_8);
    graph.edge(main_zero_8, main_A);
    graph.edge(main_A, main_3);
    graph.edge(main_bar, main_dot_11);
    graph.edge(main_dot_11, main_call_12);
    graph.edge(main_call_12, main_foo);
    graph.edge(main_foo, main_3);
    graph.edge(main_3, main_4);
    graph.edge(main_4, main_dot_14);
    graph.edge(main_dot_14, main_b);
    graph.edge(main_b, root);
    graph.edge(main_4, main_5);
    graph.edge(main_5, main_dot_16);
    graph.edge(main_dot_16, main_a);
    graph.edge(main_a, root);
    graph.edge(main_5, main_top_6);

    let a_file = graph.get_or_create_file("a.py");
    let a = graph.definition(a_file, 0, sym_a);
    let a_dot_1 = graph.pop_symbol(a_file, 1, sym_dot);
    let a_bottom_2 = graph.internal_scope(a_file, 2);
    let a_3 = graph.internal_scope(a_file, 3);
    let a_top_4 = graph.internal_scope(a_file, 4);
    let a_foo = graph.definition(a_file, 5, sym_foo);
    let a_call_6 = graph.pop_scoped_symbol(a_file, 6, sym_call);
    let a_return_7 = graph.internal_scope(a_file, 7);
    let a_x_ref = graph.reference(a_file, 8, sym_x);
    let a_params_9 = graph.internal_scope(a_file, 9);
    let a_drop_10 = graph.drop_scopes(a_file, 10);
    let a_lexical_11 = graph.internal_scope(a_file, 11);
    let a_formals_12 = graph.internal_scope(a_file, 12);
    let a_drop_13 = graph.drop_scopes(a_file, 13);
    let a_x_def = graph.definition(a_file, 14, sym_x);
    let a_x_15 = graph.pop_symbol(a_file, 15, sym_x);
    let a_zero_16 = graph.push_symbol(a_file, 16, sym_zero);
    let a_x_17 = graph.push_symbol(a_file, 17, sym_x);
    graph.edge(root, a);
    graph.edge(a, a_dot_1);
    graph.edge(a_dot_1, a_bottom_2);
    graph.edge(a_bottom_2, a_3);
    graph.edge(a_3, a_foo);
    graph.edge(a_foo, a_call_6);
    graph.edge(a_call_6, a_return_7);
    graph.edge(a_return_7, a_x_ref);
    graph.edge(a_x_ref, a_params_9);
    graph.edge(a_params_9, a_drop_10);
    graph.edge(a_drop_10, a_lexical_11);
    graph.edge(a_lexical_11, a_bottom_2);
    graph.edge(a_params_9, a_formals_12);
    graph.edge(a_formals_12, a_drop_13);
    graph.edge(a_drop_13, a_x_def);
    graph.edge(a_formals_12, a_x_15);
    graph.edge(a_x_15, a_zero_16);
    graph.edge(a_zero_16, jump_to);
    graph.edge(a_x_15, a_x_17);
    graph.edge(a_x_17, jump_to);
    graph.edge(a_3, a_top_4);

    let b_file = graph.get_or_create_file("b.py");
    let b = graph.definition(b_file, 0, sym_b);
    let b_dot_1 = graph.pop_symbol(b_file, 1, sym_dot);
    let b_bottom_2 = graph.internal_scope(b_file, 2);
    let b_3 = graph.internal_scope(b_file, 3);
    let b_top_4 = graph.internal_scope(b_file, 4);
    let b_A = graph.definition(b_file, 5, sym_A);
    let b_dot_6 = graph.pop_symbol(b_file, 6, sym_dot);
    let b_class_members_7 = graph.internal_scope(b_file, 7);
    let b_bar = graph.definition(b_file, 8, sym_bar);
    let b_call_9 = graph.pop_scoped_symbol(b_file, 9, sym_call);
    let b_self_10 = graph.internal_scope(b_file, 10);
    let b_dot_11 = graph.pop_symbol(b_file, 11, sym_dot);
    let b_instance_members_12 = graph.internal_scope(b_file, 12);
    graph.edge(root, b);
    graph.edge(b, b_dot_1);
    graph.edge(b_dot_1, b_bottom_2);
    graph.edge(b_bottom_2, b_3);
    graph.edge(b_3, b_A);
    graph.edge(b_A, b_dot_6);
    graph.edge(b_dot_6, b_class_members_7);
    graph.edge(b_class_members_7, b_bar);
    graph.edge(b_A, b_call_9);
    graph.edge(b_call_9, b_self_10);
    graph.edge(b_self_10, b_dot_11);
    graph.edge(b_dot_11, b_instance_members_12);
    graph.edge(b_instance_members_12, b_class_members_7);
    graph.edge(b_3, b_top_4);

    ClassFieldThroughFunctionParameter {
        graph,
        main,
        main_A,
        main_a,
        main_b,
        main_bar,
        main_foo,
        a,
        a_x_def,
        a_x_ref,
        a_foo,
        b,
        b_A,
        b_bar,
    }
}
