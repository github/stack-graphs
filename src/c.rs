// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines a C API for working with stack graphs in other languages.

#![allow(non_camel_case_types)]

use libc::c_char;

use crate::arena::Handle;
use crate::graph::Edge;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::NodeID;
use crate::graph::StackGraph;
use crate::graph::Symbol;

/// Contains all of the nodes and edges that make up a stack graph.
pub struct sg_stack_graph {
    inner: StackGraph,
}

/// Creates a new, initially empty stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_new() -> *mut sg_stack_graph {
    Box::into_raw(Box::new(sg_stack_graph {
        inner: StackGraph::new(),
    }))
}

/// Frees a stack graph, and all of its contents.
#[no_mangle]
pub extern "C" fn sg_stack_graph_free(graph: *mut sg_stack_graph) {
    drop(unsafe { Box::from_raw(graph) })
}

//-------------------------------------------------------------------------------------------------
// Symbols

/// A name that we are trying to resolve using stack graphs.
///
/// This typically represents a portion of an identifier as it appears in the source language.  It
/// can also represent some other "operation" that can occur in source code, and which needs to be
/// modeled in a stack graph — for instance, many languages will use a "fake" symbol named `.` to
/// represent member access.
#[repr(C)]
pub struct sg_symbol {
    pub symbol: *const c_char,
    pub symbol_len: usize,
}

/// A handle to a symbol in a stack graph.  A zero handle represents a missing symbol.
///
/// We deduplicate symbols in a stack graph — that is, we ensure that there are never multiple
/// `struct sg_symbol` instances with the same content.  That means that you can compare symbol
/// handles using simple equality, without having to dereference them.
pub type sg_symbol_handle = u32;

/// An array of all of the symbols in a stack graph.  Symbol handles are indices into this array.
/// There will never be a valid symbol at index 0; a handle with the value 0 represents a missing
/// symbol.
#[repr(C)]
pub struct sg_symbols {
    pub symbols: *const sg_symbol,
    pub count: usize,
}

/// Returns a reference to the array of symbol data in this stack graph.  The resulting array
/// pointer is only valid until the next call to any function that mutates the stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_symbols(graph: *const sg_stack_graph) -> sg_symbols {
    let graph = unsafe { &(*graph).inner };
    sg_symbols {
        symbols: graph.symbols.as_ptr() as *const sg_symbol,
        count: graph.symbols.len(),
    }
}

/// Adds new symbols to the stack graph.  You provide an array of symbol content, and an output
/// array, which must have the same length.  We will place each symbol's handle in the output
/// array.
///
/// We ensure that there is only ever one copy of a particular symbol stored in the graph — we
/// guarantee that identical symbols will have the same handles, meaning that you can compare the
/// handles using simple integer equality.
///
/// We copy the symbol data into the stack graph.  The symbol content you pass in does not need to
/// outlive the call to this function.
///
/// Each symbol must be a valid UTF-8 string.  If any symbol isn't valid UTF-8, it won't be added
/// to the stack graph, and the corresponding entry in the output array will be the null handle.
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_symbols(
    graph: *mut sg_stack_graph,
    count: usize,
    symbols: *const *const c_char,
    lengths: *const usize,
    handles_out: *mut sg_symbol_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let symbols = unsafe { std::slice::from_raw_parts(symbols as *const *const u8, count) };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let handles_out = unsafe {
        std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<Symbol>>, count)
    };
    for i in 0..count {
        let symbol = unsafe { std::slice::from_raw_parts(symbols[i], lengths[i]) };
        handles_out[i] = match std::str::from_utf8(symbol) {
            Ok(symbol) => Some(graph.add_symbol(symbol)),
            Err(_) => None,
        };
    }
}

//-------------------------------------------------------------------------------------------------
// Files

/// A source file that we have extracted stack graph data from.
///
/// It's up to you to choose what names to use for your files, but they must be unique within a
/// stack graph.  If you are analyzing files from the local filesystem, the file's path is a good
/// choice.  If your files belong to packages or repositories, they should include the package or
/// repository IDs to make sure that files in different packages or repositories don't clash with
/// each other.
#[repr(C)]
pub struct sg_file {
    pub name: *const c_char,
    pub name_len: usize,
}

/// A handle to a file in a stack graph.  A zero handle represents a missing file.
///
/// We deduplicate files in a stack graph — that is, we ensure that there are never multiple
/// `struct sg_file` instances with the same filename.  That means that you can compare file
/// handles using simple equality, without having to dereference them.
pub type sg_file_handle = u32;

/// An array of all of the files in a stack graph.  File handles are indices into this array.
/// There will never be a valid file at index 0; a handle with the value 0 represents a missing
/// file.
#[repr(C)]
pub struct sg_files {
    pub files: *const sg_file,
    pub count: usize,
}

/// Returns a reference to the array of file data in this stack graph.  The resulting array pointer
/// is only valid until the next call to any function that mutates the stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_files(graph: *const sg_stack_graph) -> sg_files {
    let graph = unsafe { &(*graph).inner };
    sg_files {
        files: graph.files.as_ptr() as *const sg_file,
        count: graph.files.len(),
    }
}

/// Adds new files to the stack graph.  You provide an array of file content, and an output array,
/// which must have the same length.  We will place each file's handle in the output array.
///
/// There can only ever be one file with a particular name in the graph.  If you try to add a file
/// with a name that already exists, you'll get the same handle as a result.
///
/// We copy the filenames into the stack graph.  The filenames you pass in do not need to outlive
/// the call to this function.
///
/// Each filename must be a valid UTF-8 string.  If any filename isn't valid UTF-8, it won't be
/// added to the stack graph, and the corresponding entry in the output array will be the null
/// handle.
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_files(
    graph: *mut sg_stack_graph,
    count: usize,
    files: *const *const c_char,
    lengths: *const usize,
    handles_out: *mut sg_file_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let files = unsafe { std::slice::from_raw_parts(files as *const *const u8, count) };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let handles_out =
        unsafe { std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<File>>, count) };
    for i in 0..count {
        let file = unsafe { std::slice::from_raw_parts(files[i], lengths[i]) };
        handles_out[i] = match std::str::from_utf8(file) {
            Ok(file) => Some(graph.get_or_create_file(file)),
            Err(_) => None,
        };
    }
}

//-------------------------------------------------------------------------------------------------
// Nodes

/// Uniquely identifies a node in a stack graph.
///
/// Each node (except for the _root node_ and _jump to scope_ node) lives in a file, and has a
/// _local ID_ that must be unique within its file.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sg_node_id {
    pub file: sg_file_handle,
    pub local_id: u32,
}

impl Into<NodeID> for sg_node_id {
    fn into(self) -> NodeID {
        unsafe { std::mem::transmute(self) }
    }
}

/// The local_id of the singleton root node.
pub const SG_ROOT_NODE_ID: u32 = 0;

/// The local_id of the singleton "jump to scope" node.
pub const SG_JUMP_TO_NODE_ID: u32 = 1;

/// A node in a stack graph.
#[repr(C)]
#[derive(Clone)]
pub struct sg_node {
    pub kind: sg_node_kind,
    pub id: sg_node_id,
    /// The symbol associated with this node.  For push nodes, this is the symbol that will be
    /// pushed onto the symbol stack.  For pop nodes, this is the symbol that we expect to pop off
    /// the symbol stack.  For all other node types, this will be null.
    pub symbol: sg_symbol_handle,
    /// The scope associated with this node.  For push scope nodes, this is the scope that will be
    /// attached to the symbol before it's pushed onto the symbol stack.  For all other node types,
    /// this will be null.
    pub scope: sg_node_handle,
    /// Whether this node is "clickable".  For push nodes, this indicates that the node represents
    /// a reference in the source.  For pop nodes, this indicates that the node represents a
    /// definition in the source.  For all other node types, this field will be unused.
    pub is_clickable: bool,
}

impl Into<Node> for sg_node {
    fn into(self) -> Node {
        unsafe { std::mem::transmute(self) }
    }
}

/// The different kinds of node that can appear in a stack graph.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum sg_node_kind {
    /// Removes everything from the current scope stack.
    SG_NODE_KIND_DROP_SCOPES,
    /// A node that can be referred to on the scope stack, which allows "jump to" nodes in any
    /// other part of the graph can jump back here.
    SG_NODE_KIND_EXPORTED_SCOPE,
    /// A node internal to a single file.  This node has no effect on the symbol or scope stacks;
    /// it's just used to add structure to the graph.
    SG_NODE_KIND_INTERNAL_SCOPE,
    /// The singleton "jump to" node, which allows a name binding path to jump back to another part
    /// of the graph.
    SG_NODE_KIND_JUMP_TO,
    /// Pops a scoped symbol from the symbol stack.  If the top of the symbol stack doesn't match
    /// the requested symbol, or if the top of the symbol stack doesn't have an attached scope
    /// list, then the path is not allowed to enter this node.
    SG_NODE_KIND_POP_SCOPED_SYMBOL,
    /// Pops a symbol from the symbol stack.  If the top of the symbol stack doesn't match the
    /// requested symbol, then the path is not allowed to enter this node.
    SG_NODE_KIND_POP_SYMBOL,
    /// Pushes a scoped symbol onto the symbol stack.
    SG_NODE_KIND_PUSH_SCOPED_SYMBOL,
    /// Pushes a symbol onto the symbol stack.
    SG_NODE_KIND_PUSH_SYMBOL,
    /// The singleton root node, which allows a name binding path to cross between files.
    SG_NODE_KIND_ROOT,
}

/// A handle to a node in a stack graph.  A zero handle represents a missing node.
pub type sg_node_handle = u32;

/// The handle of the singleton root node.
pub const SG_ROOT_NODE_HANDLE: sg_node_handle = 1;

/// The handle of the singleton "jump to scope" node.
pub const SG_JUMP_TO_NODE_HANDLE: sg_node_handle = 2;

/// An array of all of the nodes in a stack graph.  Node handles are indices into this array.
/// There will never be a valid node at index 0; a handle with the value 0 represents a missing
/// node.
#[repr(C)]
pub struct sg_nodes {
    pub nodes: *const sg_node,
    pub count: usize,
}

/// Returns a reference to the array of nodes in this stack graph.  The resulting array pointer is
/// only valid until the next call to any function that mutates the stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_nodes(graph: *const sg_stack_graph) -> sg_nodes {
    let graph = unsafe { &(*graph).inner };
    sg_nodes {
        nodes: graph.nodes.as_ptr() as *const sg_node,
        count: graph.nodes.len(),
    }
}

/// Adds new nodes to the stack graph.  You provide an array of `struct sg_node` instances.  You
/// also provide an output array, which must have the same length as `nodes`, in which we will
/// place each node's handle in the stack graph.
///
/// We copy the node content into the stack graph.  The array you pass in does not need to outlive
/// the call to this function.
///
/// You cannot add new instances of the root node or "jump to scope" node, since those are
/// singletons and already exist in the stack graph.
///
/// If any node that you pass in is invalid, it will not be added to the graph, and the
/// corresponding entry in the `handles_out` array will be null.  (Note that includes trying to add
/// a node with the same ID as an existing node, since all nodes must have unique IDs.)
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_nodes(
    graph: *mut sg_stack_graph,
    count: usize,
    nodes: *const sg_node,
    handles_out: *mut sg_node_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let nodes = unsafe { std::slice::from_raw_parts(nodes, count) };
    let handles_out =
        unsafe { std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<Node>>, count) };
    for i in 0..count {
        let node_id = nodes[i].id;
        handles_out[i] =
            validate_node(graph, &nodes[i]).and_then(|node| graph.add_node(node_id.into(), node));
    }
}

fn validate_node_id(graph: &StackGraph, node_id: sg_node_id) -> Option<()> {
    if node_id.file == 0 || node_id.file >= (graph.files.len() as u32) {
        return None;
    }
    Some(())
}

fn validate_node(graph: &StackGraph, node: &sg_node) -> Option<Node> {
    if matches!(
        &node.kind,
        sg_node_kind::SG_NODE_KIND_JUMP_TO | sg_node_kind::SG_NODE_KIND_ROOT
    ) {
        // You can never add a singleton node, since there already is one!
        return None;
    }

    // Every node must have a valid ID, which refers to an existing file.
    validate_node_id(graph, node.id)?;

    // Push and pop nodes must have a non-null symbol, and all other nodes must have a null symbol.
    if (node.symbol != 0)
        != matches!(
            &node.kind,
            sg_node_kind::SG_NODE_KIND_POP_SCOPED_SYMBOL
                | sg_node_kind::SG_NODE_KIND_POP_SYMBOL
                | sg_node_kind::SG_NODE_KIND_PUSH_SCOPED_SYMBOL
                | sg_node_kind::SG_NODE_KIND_PUSH_SYMBOL
        )
    {
        return None;
    }

    // Push scoped symbol nodes must have a non-null scope, and all other nodes must have a null
    // scope.
    if (node.scope != 0) != matches!(&node.kind, sg_node_kind::SG_NODE_KIND_PUSH_SCOPED_SYMBOL) {
        return None;
    }

    Some(node.clone().into())
}

//-------------------------------------------------------------------------------------------------
// Edges

/// Connects two nodes in a stack graph.
///
/// These edges provide the basic graph connectivity that allow us to search for name binding paths
/// in a stack graph.  (Though not all sequence of edges is a well-formed name binding: the nodes
/// that you encounter along the path must also satisfy all of the rules for maintaining correct
/// symbol and scope stacks.)
#[repr(C)]
pub struct sg_edge {
    pub source: sg_node_handle,
    pub sink: sg_node_handle,
}

/// Adds new edges to the stack graph.  You provide an array of `struct sg_edges` instances.  A
/// stack graph can contain at most one edge between any two nodes.  It is not an error if you try
/// to add an edge that already exists, but it won't have any effect on the graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_edges(
    graph: *mut sg_stack_graph,
    count: usize,
    edges: *const sg_edge,
) {
    let graph = unsafe { &mut (*graph).inner };
    let edges = unsafe { std::slice::from_raw_parts(edges, count) };
    for i in 0..count {
        let source = unsafe { std::mem::transmute(edges[i].source) };
        let sink = unsafe { std::mem::transmute(edges[i].sink) };
        graph.add_edge(Edge { source, sink });
    }
}
