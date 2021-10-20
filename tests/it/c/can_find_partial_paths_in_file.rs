// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;
use stack_graphs::c::sg_partial_path_arena;
use stack_graphs::c::sg_partial_path_arena_find_partial_paths_in_file;
use stack_graphs::c::sg_partial_path_arena_free;
use stack_graphs::c::sg_partial_path_arena_new;
use stack_graphs::c::sg_partial_path_arena_partial_path_edge_list_cells;
use stack_graphs::c::sg_partial_path_arena_partial_scope_stack_cells;
use stack_graphs::c::sg_partial_path_arena_partial_symbol_stack_cells;
use stack_graphs::c::sg_partial_path_edge_list;
use stack_graphs::c::sg_partial_path_list_count;
use stack_graphs::c::sg_partial_path_list_free;
use stack_graphs::c::sg_partial_path_list_new;
use stack_graphs::c::sg_partial_path_list_paths;
use stack_graphs::c::sg_partial_scope_stack;
use stack_graphs::c::sg_partial_symbol_stack;
use stack_graphs::c::SG_LIST_EMPTY_HANDLE;
use stack_graphs::partial::PartialPath;

use crate::c::test_graph::TestGraph;
use crate::test_graphs;

fn partial_symbol_stack_available_in_both_directions(
    partials: *mut sg_partial_path_arena,
    list: &sg_partial_symbol_stack,
) -> bool {
    let cells = sg_partial_path_arena_partial_symbol_stack_cells(partials);
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let head = list.cells;
    if head == SG_LIST_EMPTY_HANDLE {
        return true;
    }
    let cell = &cells[head as usize];
    cell.reversed != 0
}

fn partial_scope_stack_available_in_both_directions(
    partials: *mut sg_partial_path_arena,
    list: &sg_partial_scope_stack,
) -> bool {
    let cells = sg_partial_path_arena_partial_scope_stack_cells(partials);
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let head = list.cells;
    if head == SG_LIST_EMPTY_HANDLE {
        return true;
    }
    let cell = &cells[head as usize];
    cell.reversed != 0
}

fn partial_path_edge_list_available_in_both_directions(
    partials: *mut sg_partial_path_arena,
    list: &sg_partial_path_edge_list,
) -> bool {
    let cells = sg_partial_path_arena_partial_path_edge_list_cells(partials);
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let head = list.cells;
    if head == SG_LIST_EMPTY_HANDLE {
        return true;
    }
    let cell = &cells[head as usize];
    cell.reversed != 0
}

fn check_partial_paths_in_file(graph: &TestGraph, file: &str, expected_paths: &[&str]) {
    let rust_graph = unsafe { &(*graph.graph).inner };
    let file = rust_graph.get_file_unchecked(file);

    let partials = sg_partial_path_arena_new();
    let path_list = sg_partial_path_list_new();
    sg_partial_path_arena_find_partial_paths_in_file(
        graph.graph,
        partials,
        file.as_u32(),
        path_list,
    );

    // Ensure that every path has its content available in both directions.
    let results = unsafe {
        std::slice::from_raw_parts(
            sg_partial_path_list_paths(path_list),
            sg_partial_path_list_count(path_list),
        )
    };
    for path in results {
        assert!(partial_symbol_stack_available_in_both_directions(
            partials,
            &path.symbol_stack_precondition,
        ));
        assert!(partial_symbol_stack_available_in_both_directions(
            partials,
            &path.symbol_stack_postcondition,
        ));
        assert!(partial_scope_stack_available_in_both_directions(
            partials,
            &path.scope_stack_precondition,
        ));
        assert!(partial_scope_stack_available_in_both_directions(
            partials,
            &path.scope_stack_postcondition,
        ));
        assert!(partial_path_edge_list_available_in_both_directions(
            partials,
            &path.edges,
        ));
    }

    let rust_partials = unsafe { &mut (*partials).inner };
    let results = unsafe {
        std::slice::from_raw_parts(
            sg_partial_path_list_paths(path_list) as *const PartialPath,
            sg_partial_path_list_count(path_list),
        )
    };
    let results = results
        .iter()
        .map(|s| s.display(rust_graph, rust_partials).to_string())
        .collect::<BTreeSet<_>>();
    let expected_paths = expected_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_paths, results);

    sg_partial_path_list_free(path_list);
    sg_partial_path_arena_free(partials);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        &[
            // definition of `__main__` module
            "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> () [main.py(17) reference a] -> [root] <a,%1> ()",
            // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // reference to `b` in import statement
            "<%1> () [main.py(15) reference b] -> [root] <b,%1> ()",
            // `from b import *` means we can rewrite any lookup of `__main__.*` → `b.*`
            "<__main__.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
            // we can look for every reference in either `a` or `b`
            "<%1> () [main.py(9) reference A] -> [root] <a.A,%1> ()",
            "<%1> () [main.py(9) reference A] -> [root] <b.A,%1> ()",
            "<%1> () [main.py(10) reference bar] -> [root] <a.foo()/([main.py(7)]).bar,%1> ()",
            "<%1> () [main.py(10) reference bar] -> [root] <b.foo()/([main.py(7)]).bar,%1> ()",
            "<%1> () [main.py(13) reference foo] -> [root] <a.foo,%1> ()",
            "<%1> () [main.py(13) reference foo] -> [root] <b.foo,%1> ()",
            // parameter 0 of function call is `A`, which we can look up in either `a` or `b`
            "<0,%1> ($1) [main.py(7) exported scope] -> [root] <a.A,%1> ($1)",
            "<0,%1> ($1) [main.py(7) exported scope] -> [root] <b.A,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        &[
            // definition of `a` module
            "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
            // definition of `foo` function
            "<a.foo,%1> ($1) [root] -> [a.py(5) definition foo] <%1> ($1)",
            // reference to `x` in function body can resolve to formal parameter
            "<%1> () [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
            // result of function is `x`, which is passed in as a formal parameter...
            "<a.foo()/($2),%1> ($1) [root] -> [a.py(14) definition x] <%1> ()",
            // ...which we can look up either the 0th actual positional parameter...
            "<a.foo()/($2),%1> ($1) [root] -> [jump to scope] <0,%1> ($2)",
            // ...or the actual named parameter `x`
            "<a.foo()/($2),%1> ($1) [root] -> [jump to scope] <x,%1> ($2)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        &[
            // definition of `b` module
            "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
            // definition of class `A`
            "<b.A,%1> ($1) [root] -> [b.py(5) definition A] <%1> ($1)",
            // definition of class member `A.bar`
            "<b.A.bar,%1> ($1) [root] -> [b.py(8) definition bar] <%1> ($1)",
            // `bar` can also be accessed as an instance member
            "<b.A()/($2).bar,%1> ($1) [root] -> [b.py(8) definition bar] <%1> ($2)",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        &[
            // definition of `__main__` module
            "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> () [main.py(8) reference a] -> [root] <a,%1> ()",
            // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // reference to `foo` becomes `a.foo` because of import statement
            "<%1> () [main.py(6) reference foo] -> [root] <a.foo,%1> ()",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        &[
            // definition of `a` module
            "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
            // reference to `b` in import statement
            "<%1> () [a.py(6) reference b] -> [root] <b,%1> ()",
            // `from b import *` means we can rewrite any lookup of `a.*` → `b.*`
            "<a.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        &[
            // definition of `b` module
            "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> () [b.py(8) reference a] -> [root] <a,%1> ()",
            // `from a import *` means we can rewrite any lookup of `b.*` → `a.*`
            "<b.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // definition of `foo`
            "<b.foo,%1> ($1) [root] -> [b.py(6) definition foo] <%1> ($1)",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_partial_paths_in_file(
        &graph,
        "test.rs",
        // NOTE: Because everything in this example is local to one file, there aren't any partial
        // paths involving the root node.
        &[
            // reference to `a` in `main` function
            "<%1> () [test.rs(103) reference a] -> [test.rs(201) definition a] <%1> ()",
            // reference to `a` in `b` function
            "<%1> () [test.rs(307) reference a] -> [test.rs(201) definition a] <%1> ()",
            // reference to `b` in `a` function
            "<%1> () [test.rs(206) reference b] -> [test.rs(301) definition b] <%1> ()",
            // reference to `FOO` in `main` can resolve either to `a::BAR` or `b::FOO`
            "<%1> () [test.rs(101) reference FOO] -> [test.rs(204) definition BAR] <%1> ()",
            "<%1> () [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <%1> ()",
            // reference to `BAR` in `b` resolves _only_ to `a::BAR`
            "<%1> () [test.rs(305) reference BAR] -> [test.rs(204) definition BAR] <%1> ()",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        &[
            // definition of `__main__` module
            "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> () [main.py(8) reference a] -> [root] <a,%1> ()",
            // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // reference to `foo` becomes `a.foo` because of import statement
            "<%1> () [main.py(6) reference foo] -> [root] <a.foo,%1> ()",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        &[
            // definition of `a` module
            "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
            // reference to `b` in import statement
            "<%1> () [a.py(6) reference b] -> [root] <b,%1> ()",
            // `from b import *` means we can rewrite any lookup of `a.*` → `b.*`
            "<a.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        &[
            // definition of `b` module
            "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
            // definition of `foo` inside of `b` module
            "<b.foo,%1> ($1) [root] -> [b.py(5) definition foo] <%1> ($1)",
        ],
    );
}
