// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use either::Either;
use libc::c_char;
use stack_graphs::c::sg_deque_direction;
use stack_graphs::c::sg_file_handle;
use stack_graphs::c::sg_node;
use stack_graphs::c::sg_node_handle;
use stack_graphs::c::sg_node_id;
use stack_graphs::c::sg_node_kind;
use stack_graphs::c::sg_partial_path_arena_add_partial_scope_stacks;
use stack_graphs::c::sg_partial_path_arena_add_partial_symbol_stacks;
use stack_graphs::c::sg_partial_path_arena_free;
use stack_graphs::c::sg_partial_path_arena_new;
use stack_graphs::c::sg_partial_path_arena_partial_scope_stack_cells;
use stack_graphs::c::sg_partial_path_arena_partial_symbol_stack_cells;
use stack_graphs::c::sg_partial_scope_stack;
use stack_graphs::c::sg_partial_scope_stack_cells;
use stack_graphs::c::sg_partial_scoped_symbol;
use stack_graphs::c::sg_partial_symbol_stack;
use stack_graphs::c::sg_partial_symbol_stack_cells;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::c::sg_stack_graph_add_files;
use stack_graphs::c::sg_stack_graph_add_nodes;
use stack_graphs::c::sg_stack_graph_add_symbols;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::sg_symbol_handle;
use stack_graphs::c::SG_LIST_EMPTY_HANDLE;

fn add_file(graph: *mut sg_stack_graph, filename: &str) -> sg_file_handle {
    let strings = [filename.as_bytes().as_ptr() as *const c_char];
    let lengths = [filename.len()];
    let mut handles: [sg_file_handle; 1] = [0; 1];
    sg_stack_graph_add_files(
        graph,
        1,
        strings.as_ptr(),
        lengths.as_ptr(),
        handles.as_mut_ptr(),
    );
    assert!(handles[0] != 0);
    handles[0]
}

fn add_symbol(graph: *mut sg_stack_graph, value: &str) -> sg_symbol_handle {
    let strings = [value.as_bytes().as_ptr() as *const c_char];
    let lengths = [value.len()];
    let mut handles: [sg_symbol_handle; 1] = [0; 1];
    sg_stack_graph_add_symbols(
        graph,
        1,
        strings.as_ptr(),
        lengths.as_ptr(),
        handles.as_mut_ptr(),
    );
    assert!(handles[0] != 0);
    handles[0]
}

fn add_exported_scope(
    graph: *mut sg_stack_graph,
    file: sg_file_handle,
    local_id: u32,
) -> sg_node_handle {
    let node = sg_node {
        kind: sg_node_kind::SG_NODE_KIND_EXPORTED_SCOPE,
        id: sg_node_id { file, local_id },
        symbol: 0,
        is_clickable: false,
        scope: 0,
    };
    let nodes = [node];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    handles[0]
}

//-------------------------------------------------------------------------------------------------
// Partial symbol stacks

fn empty_partial_scope_stack() -> sg_partial_scope_stack {
    sg_partial_scope_stack {
        cells: 0,
        direction: sg_deque_direction::SG_DEQUE_FORWARDS,
        variable: 0,
    }
}

fn partial_scoped_symbol(
    symbol: sg_symbol_handle,
    scopes: sg_partial_scope_stack,
) -> sg_partial_scoped_symbol {
    sg_partial_scoped_symbol { symbol, scopes }
}

fn partial_symbol_stack_contains(
    cells: &sg_partial_symbol_stack_cells,
    stack: &sg_partial_symbol_stack,
    expected: &[sg_partial_scoped_symbol],
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let mut current = stack.cells;
    let expected = if stack.direction == sg_deque_direction::SG_DEQUE_FORWARDS {
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

#[test]
fn can_create_partial_symbol_stacks() {
    let graph = sg_stack_graph_new();
    let partials = sg_partial_path_arena_new();

    // We need a lot of other crap to be able to create any symbol stacks...
    let file = add_file(graph, "test.py");
    let a = add_symbol(graph, "a");
    let b = add_symbol(graph, "b");
    let c = add_symbol(graph, "c");
    let node1 = add_exported_scope(graph, file, 1);
    let node2 = add_exported_scope(graph, file, 2);
    let scopes = [node1, node2];
    let lengths = [scopes.len()];
    let variables = [0];
    let mut partial_scope_stacks = [sg_partial_scope_stack::default(); 1];
    sg_partial_path_arena_add_partial_scope_stacks(
        partials,
        lengths.len(),
        scopes.as_ptr(),
        lengths.as_ptr(),
        variables.as_ptr(),
        partial_scope_stacks.as_mut_ptr(),
    );
    let partial_scope_stack = partial_scope_stacks[0];

    // Build up the arrays of stack content and add the stacks to the path arena.
    let symbols0 = [];
    let symbols1 = [partial_scoped_symbol(a, empty_partial_scope_stack())];
    let symbols2 = [
        partial_scoped_symbol(b, partial_scope_stack),
        partial_scoped_symbol(c, empty_partial_scope_stack()),
        partial_scoped_symbol(b, empty_partial_scope_stack()),
    ];
    let lengths = [symbols0.len(), symbols1.len(), symbols2.len()];
    let mut symbolses = Vec::new();
    symbolses.extend_from_slice(&symbols0);
    symbolses.extend_from_slice(&symbols1);
    symbolses.extend_from_slice(&symbols2);
    let mut stacks = [sg_partial_symbol_stack::default(); 3];
    sg_partial_path_arena_add_partial_symbol_stacks(
        partials,
        lengths.len(),
        symbolses.as_slice().as_ptr(),
        lengths.as_ptr(),
        stacks.as_mut_ptr(),
    );

    // Then verify that we can dereference all of the new stacks.
    let cells = sg_partial_path_arena_partial_symbol_stack_cells(partials);
    assert!(partial_symbol_stack_contains(&cells, &stacks[0], &symbols0));
    assert!(partial_symbol_stack_contains(&cells, &stacks[1], &symbols1));
    assert!(partial_symbol_stack_contains(&cells, &stacks[2], &symbols2));

    sg_partial_path_arena_free(partials);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Partial scope stacks

fn partial_scope_stack_contains(
    cells: &sg_partial_scope_stack_cells,
    stack: &sg_partial_scope_stack,
    expected: &[sg_node_handle],
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let mut current = stack.cells;
    let expected = if stack.direction == sg_deque_direction::SG_DEQUE_FORWARDS {
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

fn partial_scope_stack_available_in_both_directions(
    cells: &sg_partial_scope_stack_cells,
    list: &sg_partial_scope_stack,
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let head = list.cells;
    if head == SG_LIST_EMPTY_HANDLE {
        return true;
    }
    let cell = &cells[head as usize];
    cell.reversed != 0
}

#[test]
fn can_create_partial_scope_stacks() {
    let graph = sg_stack_graph_new();
    let partials = sg_partial_path_arena_new();
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
    let variables = [0, 0, 1];
    let mut scopeses = Vec::new();
    scopeses.extend_from_slice(&scopes0);
    scopeses.extend_from_slice(&scopes1);
    scopeses.extend_from_slice(&scopes2);
    let mut stacks = [sg_partial_scope_stack::default(); 3];
    sg_partial_path_arena_add_partial_scope_stacks(
        partials,
        lengths.len(),
        scopeses.as_slice().as_ptr(),
        lengths.as_ptr(),
        variables.as_ptr(),
        stacks.as_mut_ptr(),
    );

    // Then verify that we can dereference all of the new stacks.
    let cells = sg_partial_path_arena_partial_scope_stack_cells(partials);
    assert!(partial_scope_stack_contains(&cells, &stacks[0], &scopes0));
    assert!(partial_scope_stack_contains(&cells, &stacks[1], &scopes1));
    assert!(partial_scope_stack_contains(&cells, &stacks[2], &scopes2));

    assert_eq!(stacks[0].variable, variables[0]);
    assert_eq!(stacks[1].variable, variables[1]);
    assert_eq!(stacks[2].variable, variables[2]);

    // Verify that each stack is available in both directions.
    assert!(partial_scope_stack_available_in_both_directions(
        &cells, &stacks[0]
    ));
    assert!(partial_scope_stack_available_in_both_directions(
        &cells, &stacks[1]
    ));
    assert!(partial_scope_stack_available_in_both_directions(
        &cells, &stacks[2]
    ));

    sg_partial_path_arena_free(partials);
    sg_stack_graph_free(graph);
}
