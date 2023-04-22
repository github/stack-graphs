// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;
use stack_graphs::c::sg_partial_path_arena_find_all_complete_paths;
use stack_graphs::c::sg_partial_path_arena_free;
use stack_graphs::c::sg_partial_path_arena_new;
use stack_graphs::c::sg_partial_path_list_count;
use stack_graphs::c::sg_partial_path_list_free;
use stack_graphs::c::sg_partial_path_list_new;
use stack_graphs::c::sg_partial_path_list_paths;
use stack_graphs::partial::PartialPath;

use crate::c::test_graph::TestGraph;
use crate::test_graphs;

fn check_jump_to_definition(graph: &TestGraph, expected_paths: &[&str]) {
    let rust_graph = unsafe { &(*graph.graph).inner };
    let paths = sg_partial_path_arena_new();
    let path_list = sg_partial_path_list_new();
    let references = rust_graph
        .iter_nodes()
        .filter(|handle| rust_graph[*handle].is_reference())
        .collect::<Vec<_>>();
    sg_partial_path_arena_find_all_complete_paths(
        graph.graph,
        paths,
        references.len(),
        references.as_ptr() as *const _,
        path_list,
        std::ptr::null(),
    );

    let rust_paths = unsafe { &mut (*paths).inner };
    let results = unsafe {
        std::slice::from_raw_parts(
            sg_partial_path_list_paths(path_list) as *const PartialPath,
            sg_partial_path_list_count(path_list),
        )
    };
    let results = results
        .iter()
        .map(|s| s.display(rust_graph, rust_paths).to_string())
        .collect::<BTreeSet<_>>();
    let expected_paths = expected_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_paths, results);

    sg_partial_path_list_free(path_list);
    unsafe { sg_partial_path_arena_free(paths) };
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in import statement
            "<> () [main.py(17) reference a] -> [a.py(0) definition a] <> ()",
            // reference to `b` in import statement
            "<> () [main.py(15) reference b] -> [b.py(0) definition b] <> ()",
            // reference to `foo` in function call resolves to function definition
            "<> () [main.py(13) reference foo] -> [a.py(5) definition foo] <> ()",
            // reference to `A` as function parameter resolves to class definition
            "<> () [main.py(9) reference A] -> [b.py(5) definition A] <> ()",
            // reference to `bar` on result flows through body of `foo` to find `A.bar`
            "<> () [main.py(10) reference bar] -> [b.py(8) definition bar] <> ()",
            // reference to `x` in function body resolves to formal parameter
            "<> () [a.py(8) reference x] -> [a.py(14) definition x] <> ()",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in import statement
            "<> () [main.py(8) reference a] -> [a.py(0) definition a] <> ()",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "<> () [main.py(6) reference foo] -> [b.py(6) definition foo] <> ()",
            // reference to `b` in import statement
            "<> () [a.py(6) reference b] -> [b.py(0) definition b] <> ()",
            // reference to `a` in import statement
            "<> () [b.py(8) reference a] -> [a.py(0) definition a] <> ()",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in `a::FOO` resolves to module definition
            "<> () [test.rs(103) reference a] -> [test.rs(201) definition a] <> ()",
            // reference to `a::FOO` in `main` can resolve either to `a::BAR` or `b::FOO`
            "<> () [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <> ()",
            "<> () [test.rs(101) reference FOO] -> [test.rs(204) definition BAR] <> ()",
            // reference to `b` in use statement resolves to module definition
            "<> () [test.rs(206) reference b] -> [test.rs(301) definition b] <> ()",
            // reference to `a` in use statement resolves to module definition
            "<> () [test.rs(307) reference a] -> [test.rs(201) definition a] <> ()",
            // reference to `BAR` in module `b` can _only_ resolve to `a::BAR`
            "<> () [test.rs(305) reference BAR] -> [test.rs(204) definition BAR] <> ()",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_jump_to_definition(
        &graph,
        &[
            // reference to `a` in import statement
            "<> () [main.py(8) reference a] -> [a.py(0) definition a] <> ()",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "<> () [main.py(6) reference foo] -> [b.py(5) definition foo] <> ()",
            // reference to `b` in import statement
            "<> () [a.py(6) reference b] -> [b.py(0) definition b] <> ()",
        ],
    );
}
