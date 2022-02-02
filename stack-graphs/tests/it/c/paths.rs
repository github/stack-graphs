// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use controlled_option::ControlledOption;
use either::Either;
use libc::c_char;
use stack_graphs::c::sg_deque_direction;
use stack_graphs::c::sg_file_handle;
use stack_graphs::c::sg_node;
use stack_graphs::c::sg_node_handle;
use stack_graphs::c::sg_node_id;
use stack_graphs::c::sg_node_kind;
use stack_graphs::c::sg_path_arena_add_path_edge_lists;
use stack_graphs::c::sg_path_arena_add_scope_stacks;
use stack_graphs::c::sg_path_arena_add_symbol_stacks;
use stack_graphs::c::sg_path_arena_free;
use stack_graphs::c::sg_path_arena_new;
use stack_graphs::c::sg_path_arena_path_edge_list_cells;
use stack_graphs::c::sg_path_arena_scope_stack_cells;
use stack_graphs::c::sg_path_arena_symbol_stack_cells;
use stack_graphs::c::sg_path_edge;
use stack_graphs::c::sg_path_edge_list;
use stack_graphs::c::sg_path_edge_list_cells;
use stack_graphs::c::sg_scope_stack;
use stack_graphs::c::sg_scope_stack_cells;
use stack_graphs::c::sg_scoped_symbol;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::c::sg_stack_graph_add_files;
use stack_graphs::c::sg_stack_graph_add_symbols;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_get_or_create_nodes;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::sg_symbol_handle;
use stack_graphs::c::sg_symbol_stack;
use stack_graphs::c::sg_symbol_stack_cells;
use stack_graphs::c::SG_LIST_EMPTY_HANDLE;
use stack_graphs::c::SG_NULL_HANDLE;
use stack_graphs::paths::PathEdgeList;
use stack_graphs::paths::ScopeStack;
use stack_graphs::paths::SymbolStack;

fn add_file(graph: *mut sg_stack_graph, filename: &str) -> sg_file_handle {
    let lengths = [filename.len()];
    let mut handles: [sg_file_handle; 1] = [SG_NULL_HANDLE; 1];
    sg_stack_graph_add_files(
        graph,
        1,
        filename.as_bytes().as_ptr() as *const c_char,
        lengths.as_ptr(),
        handles.as_mut_ptr(),
    );
    assert!(handles[0] != SG_NULL_HANDLE);
    handles[0]
}

fn add_symbol(graph: *mut sg_stack_graph, value: &str) -> sg_symbol_handle {
    let lengths = [value.len()];
    let mut handles: [sg_symbol_handle; 1] = [SG_NULL_HANDLE; 1];
    sg_stack_graph_add_symbols(
        graph,
        1,
        value.as_bytes().as_ptr() as *const c_char,
        lengths.as_ptr(),
        handles.as_mut_ptr(),
    );
    assert!(handles[0] != SG_NULL_HANDLE);
    handles[0]
}

fn add_exported_scope(
    graph: *mut sg_stack_graph,
    file: sg_file_handle,
    local_id: u32,
) -> sg_node_handle {
    let node = sg_node {
        kind: sg_node_kind::SG_NODE_KIND_SCOPE,
        id: sg_node_id { file, local_id },
        symbol: SG_NULL_HANDLE,
        is_endpoint: true,
        scope: sg_node_id::default(),
    };
    let nodes = [node];
    let mut handles: [sg_node_handle; 1] = [SG_NULL_HANDLE; 1];
    sg_stack_graph_get_or_create_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    handles[0]
}

//-------------------------------------------------------------------------------------------------
// Symbol stacks

fn empty_scope_stack() -> sg_scope_stack {
    sg_scope_stack {
        cells: SG_NULL_HANDLE,
        length: 0,
    }
}

fn scoped_symbol(symbol: sg_symbol_handle, scopes: sg_scope_stack) -> sg_scoped_symbol {
    sg_scoped_symbol { symbol, scopes }
}

fn symbol_stack_contains(
    cells: &sg_symbol_stack_cells,
    stack: &sg_symbol_stack,
    expected: &[sg_scoped_symbol],
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let mut current = stack.cells;
    for node in expected {
        if current == SG_LIST_EMPTY_HANDLE {
            return false;
        }
        let cell = &cells[current as usize];
        if cell.head != *node {
            return false;
        }
        current = cell.tail;
    }
    current == SG_LIST_EMPTY_HANDLE
}

#[test]
fn can_create_symbol_stacks() {
    let graph = sg_stack_graph_new();
    let paths = sg_path_arena_new();

    // We need a lot of other crap to be able to create any symbol stacks...
    let file = add_file(graph, "test.py");
    let a = add_symbol(graph, "a");
    let b = add_symbol(graph, "b");
    let c = add_symbol(graph, "c");
    let node1 = add_exported_scope(graph, file, 1);
    let node2 = add_exported_scope(graph, file, 2);
    let scopes = [node1, node2];
    let lengths = [scopes.len()];
    let mut scope_stacks = [sg_scope_stack::default(); 1];
    sg_path_arena_add_scope_stacks(
        paths,
        lengths.len(),
        scopes.as_ptr(),
        lengths.as_ptr(),
        scope_stacks.as_mut_ptr(),
    );
    let scope_stack = scope_stacks[0];

    // Build up the arrays of stack content and add the stacks to the path arena.
    let symbols0 = [];
    let symbols1 = [scoped_symbol(a, empty_scope_stack())];
    let symbols2 = [
        scoped_symbol(b, scope_stack),
        scoped_symbol(c, empty_scope_stack()),
        scoped_symbol(b, empty_scope_stack()),
    ];
    let lengths = [symbols0.len(), symbols1.len(), symbols2.len()];
    let mut symbolses = Vec::new();
    symbolses.extend_from_slice(&symbols0);
    symbolses.extend_from_slice(&symbols1);
    symbolses.extend_from_slice(&symbols2);
    let mut stacks = [sg_symbol_stack::default(); 3];
    sg_path_arena_add_symbol_stacks(
        paths,
        lengths.len(),
        symbolses.as_slice().as_ptr(),
        lengths.as_ptr(),
        stacks.as_mut_ptr(),
    );

    // Then verify that we can dereference all of the new stacks.
    let cells = sg_path_arena_symbol_stack_cells(paths);
    assert!(symbol_stack_contains(&cells, &stacks[0], &symbols0));
    assert!(symbol_stack_contains(&cells, &stacks[1], &symbols1));
    assert!(symbol_stack_contains(&cells, &stacks[2], &symbols2));

    sg_path_arena_free(paths);
    sg_stack_graph_free(graph);
}

#[test]
#[allow(unused_assignments)]
fn verify_null_symbol_stack_representation() {
    let bytes = [0x55u8; std::mem::size_of::<SymbolStack>()];
    let mut rust: ControlledOption<SymbolStack> = unsafe { std::mem::transmute(bytes) };
    rust = ControlledOption::none();
    let c: sg_symbol_stack = unsafe { std::mem::transmute(rust) };
    assert_eq!(c.cells, SG_NULL_HANDLE);
}

//-------------------------------------------------------------------------------------------------
// Scope stacks

fn scope_stack_contains(
    cells: &sg_scope_stack_cells,
    stack: &sg_scope_stack,
    expected: &[sg_node_handle],
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let mut current = stack.cells;
    for node in expected {
        if current == SG_LIST_EMPTY_HANDLE {
            return false;
        }
        let cell = &cells[current as usize];
        if cell.head != *node {
            return false;
        }
        current = cell.tail;
    }
    current == SG_LIST_EMPTY_HANDLE
}

#[test]
fn can_create_scope_stacks() {
    let graph = sg_stack_graph_new();
    let paths = sg_path_arena_new();
    let file = add_file(graph, "test.py");
    let node1 = add_exported_scope(graph, file, 1);
    let node2 = add_exported_scope(graph, file, 2);
    let node3 = add_exported_scope(graph, file, 3);
    let node4 = add_exported_scope(graph, file, 4);

    // Build up the arrays of stack content and add the stacks to the path arena.
    let scopes0 = [];
    let scopes1 = [node1];
    let scopes2 = [node2, node3, node4];
    let lengths = [scopes0.len(), scopes1.len(), scopes2.len()];
    let mut scopeses = Vec::new();
    scopeses.extend_from_slice(&scopes0);
    scopeses.extend_from_slice(&scopes1);
    scopeses.extend_from_slice(&scopes2);
    let mut stacks = [sg_scope_stack::default(); 3];
    sg_path_arena_add_scope_stacks(
        paths,
        lengths.len(),
        scopeses.as_slice().as_ptr(),
        lengths.as_ptr(),
        stacks.as_mut_ptr(),
    );

    // Then verify that we can dereference all of the new stacks.
    let cells = sg_path_arena_scope_stack_cells(paths);
    assert!(scope_stack_contains(&cells, &stacks[0], &scopes0));
    assert!(scope_stack_contains(&cells, &stacks[1], &scopes1));
    assert!(scope_stack_contains(&cells, &stacks[2], &scopes2));

    sg_path_arena_free(paths);
    sg_stack_graph_free(graph);
}

#[test]
#[allow(unused_assignments)]
fn verify_null_scope_stack_representation() {
    let bytes = [0x55u8; std::mem::size_of::<ScopeStack>()];
    let mut rust: ControlledOption<ScopeStack> = unsafe { std::mem::transmute(bytes) };
    rust = ControlledOption::none();
    let c: sg_scope_stack = unsafe { std::mem::transmute(rust) };
    assert_eq!(c.cells, SG_NULL_HANDLE);
}

//-------------------------------------------------------------------------------------------------
// Path edge lists

fn path_edge(file: sg_file_handle, local_id: u32, precedence: i32) -> sg_path_edge {
    let source_node_id = sg_node_id { file, local_id };
    sg_path_edge {
        source_node_id,
        precedence,
    }
}

fn path_edge_list_contains(
    cells: &sg_path_edge_list_cells,
    list: &sg_path_edge_list,
    expected: &[sg_path_edge],
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let mut current = list.cells;
    let expected = if list.direction == sg_deque_direction::SG_DEQUE_FORWARDS {
        Either::Left(expected.iter())
    } else {
        Either::Right(expected.iter().rev())
    };
    for node in expected {
        if current == SG_LIST_EMPTY_HANDLE {
            return false;
        }
        let cell = &cells[current as usize];
        if cell.head != *node {
            return false;
        }
        current = cell.tail;
    }
    current == SG_LIST_EMPTY_HANDLE
}

fn path_edge_list_available_in_both_directions(
    cells: &sg_path_edge_list_cells,
    list: &sg_path_edge_list,
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let head = list.cells;
    if head == SG_LIST_EMPTY_HANDLE {
        return true;
    }
    let cell = &cells[head as usize];
    cell.reversed != SG_NULL_HANDLE
}

#[test]
fn can_create_path_edge_lists() {
    let graph = sg_stack_graph_new();
    let paths = sg_path_arena_new();
    let file = add_file(graph, "test.py");

    // Build up the arrays of edge list content and add the lists to the path arena.
    let edges0 = [];
    let edges1 = [path_edge(file, 25, 25)];
    let edges2 = [
        path_edge(file, 1, 11),
        path_edge(file, 2, 12),
        path_edge(file, 3, 13),
    ];
    let lengths = [edges0.len(), edges1.len(), edges2.len()];
    let mut edgeses = Vec::new();
    edgeses.extend_from_slice(&edges0);
    edgeses.extend_from_slice(&edges1);
    edgeses.extend_from_slice(&edges2);
    let mut lists = [sg_path_edge_list::default(); 3];
    sg_path_arena_add_path_edge_lists(
        paths,
        lengths.len(),
        edgeses.as_slice().as_ptr(),
        lengths.as_ptr(),
        lists.as_mut_ptr(),
    );

    // Then verify that we can dereference all of the new lists.
    let cells = sg_path_arena_path_edge_list_cells(paths);
    assert!(path_edge_list_contains(&cells, &lists[0], &edges0));
    assert!(path_edge_list_contains(&cells, &lists[1], &edges1));
    assert!(path_edge_list_contains(&cells, &lists[2], &edges2));

    // Verify that each list is available in both directions.
    assert!(path_edge_list_available_in_both_directions(
        &cells, &lists[0]
    ));
    assert!(path_edge_list_available_in_both_directions(
        &cells, &lists[1]
    ));
    assert!(path_edge_list_available_in_both_directions(
        &cells, &lists[2]
    ));

    sg_path_arena_free(paths);
    sg_stack_graph_free(graph);
}

#[test]
#[allow(unused_assignments)]
fn verify_null_path_edge_list_representation() {
    let bytes = [0x55u8; std::mem::size_of::<PathEdgeList>()];
    let mut rust: ControlledOption<PathEdgeList> = unsafe { std::mem::transmute(bytes) };
    rust = ControlledOption::none();
    let c: sg_path_edge_list = unsafe { std::mem::transmute(rust) };
    assert_eq!(c.cells, SG_NULL_HANDLE);
}
