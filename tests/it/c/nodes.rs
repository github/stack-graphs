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
use stack_graphs::c::sg_nodes;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::c::sg_stack_graph_add_files;
use stack_graphs::c::sg_stack_graph_add_nodes;
use stack_graphs::c::sg_stack_graph_add_symbols;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::sg_stack_graph_nodes;
use stack_graphs::c::sg_symbol_handle;
use stack_graphs::c::SG_JUMP_TO_NODE_HANDLE;
use stack_graphs::c::SG_JUMP_TO_NODE_ID;
use stack_graphs::c::SG_ROOT_NODE_HANDLE;
use stack_graphs::c::SG_ROOT_NODE_ID;
use stack_graphs::graph::Node;
use stack_graphs::graph::NodeID;

fn node_id(file: sg_file_handle, local_id: u32) -> NodeID {
    NodeID::new_in_file(unsafe { std::mem::transmute(file) }, local_id)
}

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

fn add_symbol(graph: *mut sg_stack_graph, symbol: &str) -> sg_symbol_handle {
    let strings = [symbol.as_bytes().as_ptr() as *const c_char];
    let lengths = [symbol.len()];
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

//-------------------------------------------------------------------------------------------------
// Singleton nodes

fn jump_to_node() -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_JUMP_TO,
        id: sg_node_id {
            file: 0,
            local_id: SG_JUMP_TO_NODE_ID,
        },
        symbol: 0,
        is_clickable: false,
        scope: 0,
    }
}

fn root_node() -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_ROOT,
        id: sg_node_id {
            file: 0,
            local_id: SG_ROOT_NODE_ID,
        },
        symbol: 0,
        is_clickable: false,
        scope: 0,
    }
}

fn get_node(arena: &sg_nodes, handle: sg_node_handle) -> &Node {
    assert!(handle != 0);
    let slice = unsafe { std::slice::from_raw_parts(arena.nodes as *const Node, arena.count) };
    &slice[handle as usize]
}

#[test]
fn cannot_add_singleton_nodes() {
    let graph = sg_stack_graph_new();
    let nodes = [root_node(), jump_to_node()];
    let mut handles: [sg_node_handle; 2] = [0; 2];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles.iter().all(|h| *h == 0));
    sg_stack_graph_free(graph);
}

#[test]
fn can_dereference_singleton_nodes() {
    let graph = sg_stack_graph_new();
    let node_arena = sg_stack_graph_nodes(graph);
    assert!(get_node(&node_arena, SG_ROOT_NODE_HANDLE).is_root());
    assert!(get_node(&node_arena, SG_JUMP_TO_NODE_HANDLE).is_jump_to());
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Drop scopes node

fn drop_scopes(file: sg_file_handle, local_id: u32) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_DROP_SCOPES,
        id: sg_node_id { file, local_id },
        symbol: 0,
        is_clickable: false,
        scope: 0,
    }
}

#[test]
fn can_add_drop_scopes_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [drop_scopes(file, 42)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::DropScopes(_)));
    assert!(node.id() == node_id(file, 42));
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn drop_scopes_cannot_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let mut nodes = [drop_scopes(file, 42)];
    nodes[0].symbol = symbol;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn drop_scopes_cannot_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let mut nodes = [drop_scopes(file, 42)];
    nodes[0].scope = SG_JUMP_TO_NODE_HANDLE;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Exported scope node

fn exported_scope(file: sg_file_handle, local_id: u32) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_EXPORTED_SCOPE,
        id: sg_node_id { file, local_id },
        symbol: 0,
        is_clickable: false,
        scope: 0,
    }
}

#[test]
fn can_add_exported_scope_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [exported_scope(file, 42)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::ExportedScope(_)));
    assert!(node.id() == node_id(file, 42));
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn exported_scope_cannot_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let mut nodes = [exported_scope(file, 42)];
    nodes[0].symbol = symbol;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn exported_scope_cannot_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let mut nodes = [exported_scope(file, 42)];
    nodes[0].scope = SG_JUMP_TO_NODE_HANDLE;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Internal scope node

fn internal_scope(file: sg_file_handle, local_id: u32) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_INTERNAL_SCOPE,
        id: sg_node_id { file, local_id },
        symbol: 0,
        is_clickable: false,
        scope: 0,
    }
}

#[test]
fn can_add_internal_scope_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [internal_scope(file, 42)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::InternalScope(_)));
    assert!(node.id() == node_id(file, 42));
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn internal_scope_cannot_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let mut nodes = [internal_scope(file, 42)];
    nodes[0].symbol = symbol;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn internal_scope_cannot_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let mut nodes = [internal_scope(file, 42)];
    nodes[0].scope = SG_JUMP_TO_NODE_HANDLE;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Pop scoped symbol node

fn pop_scoped_symbol(file: sg_file_handle, local_id: u32, symbol: sg_symbol_handle) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_POP_SCOPED_SYMBOL,
        id: sg_node_id { file, local_id },
        symbol,
        is_clickable: true,
        scope: 0,
    }
}

#[test]
fn can_add_pop_scoped_symbol_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let nodes = [pop_scoped_symbol(file, 42, symbol)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::PopScopedSymbol(_)));
    assert!(node.id() == node_id(file, 42));
    assert!(node.symbol().unwrap().as_usize() == symbol as usize);
    assert!(node.is_definition());
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn pop_scoped_symbol_must_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [pop_scoped_symbol(file, 42, 0)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn pop_scoped_symbol_cannot_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let mut nodes = [pop_scoped_symbol(file, 42, symbol)];
    nodes[0].scope = SG_JUMP_TO_NODE_HANDLE;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Pop symbol node

fn pop_symbol(file: sg_file_handle, local_id: u32, symbol: sg_symbol_handle) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_POP_SYMBOL,
        id: sg_node_id { file, local_id },
        symbol,
        is_clickable: true,
        scope: 0,
    }
}

#[test]
fn can_add_pop_symbol_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let nodes = [pop_symbol(file, 42, symbol)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::PopSymbol(_)));
    assert!(node.id() == node_id(file, 42));
    assert!(node.symbol().unwrap().as_usize() == symbol as usize);
    assert!(node.is_definition());
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn pop_symbol_must_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [pop_symbol(file, 42, 0)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn pop_symbol_cannot_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let mut nodes = [pop_symbol(file, 42, symbol)];
    nodes[0].scope = SG_JUMP_TO_NODE_HANDLE;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Push scoped symbol node

fn push_scoped_symbol(
    file: sg_file_handle,
    local_id: u32,
    symbol: sg_symbol_handle,
    scope: sg_node_handle,
) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_PUSH_SCOPED_SYMBOL,
        id: sg_node_id { file, local_id },
        symbol,
        is_clickable: true,
        scope,
    }
}

#[test]
fn can_add_push_scoped_symbol_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let nodes = [push_scoped_symbol(file, 42, symbol, SG_JUMP_TO_NODE_HANDLE)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::PushScopedSymbol(_)));
    assert!(node.id() == node_id(file, 42));
    assert!(node.symbol().unwrap().as_usize() == symbol as usize);
    assert!(node.scope().unwrap().as_usize() == SG_JUMP_TO_NODE_HANDLE as usize);
    assert!(node.is_reference());
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn push_scoped_symbol_must_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [push_scoped_symbol(file, 42, 0, SG_JUMP_TO_NODE_HANDLE)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn push_scoped_symbol_must_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let nodes = [push_scoped_symbol(file, 42, symbol, 0)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

//-------------------------------------------------------------------------------------------------
// Push symbol node

fn push_symbol(file: sg_file_handle, local_id: u32, symbol: sg_symbol_handle) -> sg_node {
    sg_node {
        kind: sg_node_kind::SG_NODE_KIND_PUSH_SYMBOL,
        id: sg_node_id { file, local_id },
        symbol,
        is_clickable: true,
        scope: 0,
    }
}

#[test]
fn can_add_push_symbol_node() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let nodes = [push_symbol(file, 42, symbol)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    // Add the node and verify its contents after dereferencing it.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    let node_arena = sg_stack_graph_nodes(graph);
    let node = get_node(&node_arena, handles[0]);
    assert!(matches!(node, Node::PushSymbol(_)));
    assert!(node.id() == node_id(file, 42));
    assert!(node.symbol().unwrap().as_usize() == symbol as usize);
    assert!(node.is_reference());
    // Make sure we can't add the same node again.
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn push_symbol_must_have_symbol() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let nodes = [push_symbol(file, 42, 0)];
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}

#[test]
fn push_symbol_cannot_have_scope() {
    let graph = sg_stack_graph_new();
    let file = add_file(graph, "test.py");
    let symbol = add_symbol(graph, "a");
    let mut nodes = [push_symbol(file, 42, symbol)];
    nodes[0].scope = SG_JUMP_TO_NODE_HANDLE;
    let mut handles: [sg_node_handle; 1] = [0; 1];
    sg_stack_graph_add_nodes(graph, nodes.len(), nodes.as_ptr(), handles.as_mut_ptr());
    assert!(handles[0] == 0);
    sg_stack_graph_free(graph);
}
