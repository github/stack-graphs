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
/// ``` ignore
/// mod a {
///   pub use crate::b::*;
///   pub const BAR: i32 = 1;
/// }
///
/// mod b {
///   pub use crate::a::*;
///   pub const FOO: i32 = BAR;
/// }
///
/// fn main() {
///   println!("FOO is {}", a::FOO);
/// }
/// ```
#[allow(non_snake_case)]
pub struct CyclicImportsRust {
    pub graph: StackGraph,
    pub file_root: Handle<Node>,
    // Interesting nodes in crate root
    pub main_a: Handle<Node>,
    pub main_FOO: Handle<Node>,
    // Interesting nodes in mod a
    pub a: Handle<Node>,
    pub a_b: Handle<Node>,
    pub a_BAR: Handle<Node>,
    // Interesting nodes in mod b
    pub b: Handle<Node>,
    pub b_a: Handle<Node>,
    pub b_BAR: Handle<Node>,
    pub b_FOO: Handle<Node>,
}

#[allow(non_snake_case)]
pub fn new() -> CyclicImportsRust {
    let mut graph = StackGraph::new();
    let sym_colons = graph.add_symbol("::");
    let sym_a = graph.add_symbol("a");
    let sym_b = graph.add_symbol("b");
    let sym_BAR = graph.add_symbol("BAR");
    let sym_FOO = graph.add_symbol("FOO");

    let file = graph.get_or_create_file("test.rs");
    let file_root = graph.internal_scope(file, 0);

    let main_FOO = graph.reference(file, 101, sym_FOO);
    let main_colons_2 = graph.push_symbol(file, 102, sym_colons);
    let main_a = graph.reference(file, 103, sym_a);
    graph.edge(main_FOO, main_colons_2);
    graph.edge(main_colons_2, main_a);
    graph.edge(main_a, file_root);

    let a = graph.definition(file, 201, sym_a);
    let a_colons_2 = graph.pop_symbol(file, 202, sym_colons);
    let a_mod_3 = graph.internal_scope(file, 203);
    let a_BAR = graph.definition(file, 204, sym_BAR);
    let a_colons_5 = graph.push_symbol(file, 205, sym_colons);
    let a_b = graph.reference(file, 206, sym_b);
    graph.edge(file_root, a);
    graph.edge(a, a_colons_2);
    graph.edge(a_colons_2, a_mod_3);
    graph.edge(a_mod_3, a_BAR);
    graph.edge(a_mod_3, a_colons_5);
    graph.edge(a_colons_5, a_b);
    graph.edge(a_b, file_root);

    let b = graph.definition(file, 301, sym_b);
    let b_colons_2 = graph.pop_symbol(file, 302, sym_colons);
    let b_mod_3 = graph.internal_scope(file, 303);
    let b_FOO = graph.definition(file, 304, sym_FOO);
    let b_BAR = graph.reference(file, 305, sym_BAR);
    let b_colons_6 = graph.push_symbol(file, 306, sym_colons);
    let b_a = graph.reference(file, 307, sym_a);
    graph.edge(file_root, b);
    graph.edge(b, b_colons_2);
    graph.edge(b_colons_2, b_mod_3);
    graph.edge(b_mod_3, b_FOO);
    graph.edge(b_FOO, b_BAR);
    graph.edge(b_BAR, b_mod_3);
    graph.edge(b_mod_3, b_colons_6);
    graph.edge(b_colons_6, b_a);
    graph.edge(b_a, file_root);

    CyclicImportsRust {
        graph,
        file_root,
        main_a,
        main_FOO,
        a,
        a_b,
        a_BAR,
        b,
        b_a,
        b_BAR,
        b_FOO,
    }
}
