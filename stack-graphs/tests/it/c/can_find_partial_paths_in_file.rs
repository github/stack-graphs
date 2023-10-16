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
use stack_graphs::c::sg_stitcher_config;
use stack_graphs::c::SG_LIST_EMPTY_HANDLE;
use stack_graphs::c::SG_NULL_HANDLE;
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
    cell.reversed != SG_NULL_HANDLE
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
    cell.reversed != SG_NULL_HANDLE
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
    cell.reversed != SG_NULL_HANDLE
}

fn check_partial_paths_in_file(graph: &TestGraph, file: &str, expected_paths: &[&str]) {
    let rust_graph = unsafe { &(*graph.graph).inner };
    let file = rust_graph.get_file(file).expect("Missing file");

    let partials = sg_partial_path_arena_new();
    let path_list = sg_partial_path_list_new();
    let config = sg_stitcher_config {
        detect_similar_paths: false,
    };
    sg_partial_path_arena_find_partial_paths_in_file(
        graph.graph,
        partials,
        file.as_u32(),
        path_list,
        config,
        std::ptr::null(),
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
        crate::can_find_partial_paths_in_file::CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_MAIN_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        crate::can_find_partial_paths_in_file::CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_A_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        crate::can_find_partial_paths_in_file::CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_B_PATHS,
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        crate::can_find_partial_paths_in_file::CYCLIC_IMPORTS_PYTHON_MAIN_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        crate::can_find_partial_paths_in_file::CYCLIC_IMPORTS_PYTHON_A_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        crate::can_find_partial_paths_in_file::CYCLIC_IMPORTS_PYTHON_B_PATHS,
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_partial_paths_in_file(
        &graph,
        "test.rs",
        crate::can_find_partial_paths_in_file::CYCLIC_IMPORTS_RUST_PATHS,
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        crate::can_find_partial_paths_in_file::SEQUENCED_IMPORT_STAR_MAIN_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        crate::can_find_partial_paths_in_file::SEQUENCED_IMPORT_STAR_A_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        crate::can_find_partial_paths_in_file::SEQUENCED_IMPORT_STAR_B_PATHS,
    );
}
