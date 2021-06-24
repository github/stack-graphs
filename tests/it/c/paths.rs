// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use libc::c_char;
use stack_graphs::c::sg_file_handle;
use stack_graphs::c::sg_node;
use stack_graphs::c::sg_node_handle;
use stack_graphs::c::sg_node_id;
use stack_graphs::c::sg_node_kind;
use stack_graphs::c::sg_path_arena_add_scope_stacks;
use stack_graphs::c::sg_path_arena_free;
use stack_graphs::c::sg_path_arena_new;
use stack_graphs::c::sg_path_arena_scope_stack_cells;
use stack_graphs::c::sg_scope_stack;
use stack_graphs::c::sg_scope_stack_cells;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::c::sg_stack_graph_add_files;
use stack_graphs::c::sg_stack_graph_add_nodes;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::SG_SCOPE_STACK_EMPTY_HANDLE;

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
// Scope stacks

fn scope_stack_contains(
    cells: &sg_scope_stack_cells,
    stack: &sg_scope_stack,
    expected: &[sg_node_handle],
) -> bool {
    let cells = unsafe { std::slice::from_raw_parts(cells.cells, cells.count) };
    let mut current = stack.cells;
    for node in expected {
        if current == SG_SCOPE_STACK_EMPTY_HANDLE {
            return false;
        }
        let cell = &cells[current as usize];
        if cell.head != *node {
            return false;
        }
        current = cell.tail;
    }
    current == SG_SCOPE_STACK_EMPTY_HANDLE
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
