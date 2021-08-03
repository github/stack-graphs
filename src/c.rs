// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines a C API for working with stack graphs in other languages.

#![allow(non_camel_case_types)]

use std::convert::TryInto;

use libc::c_char;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::NodeID;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::partial::PartialPath;
use crate::partial::PartialPathEdge;
use crate::partial::PartialPathEdgeList;
use crate::partial::PartialPaths;
use crate::partial::PartialScopeStack;
use crate::partial::PartialScopedSymbol;
use crate::partial::PartialSymbolStack;
use crate::paths::Path;
use crate::paths::PathEdge;
use crate::paths::PathEdgeList;
use crate::paths::Paths;
use crate::paths::ScopeStack;
use crate::paths::ScopedSymbol;
use crate::paths::SymbolStack;
use crate::stitching::Database;
use crate::stitching::PathStitcher;

/// Contains all of the nodes and edges that make up a stack graph.
pub struct sg_stack_graph {
    pub inner: StackGraph,
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

/// Manages the state of a collection of paths built up as part of the path-finding algorithm.
pub struct sg_path_arena {
    pub inner: Paths,
}

/// Creates a new, initially empty path arena.
#[no_mangle]
pub extern "C" fn sg_path_arena_new() -> *mut sg_path_arena {
    Box::into_raw(Box::new(sg_path_arena {
        inner: Paths::new(),
    }))
}

/// Frees a path arena, and all of its contents.
#[no_mangle]
pub extern "C" fn sg_path_arena_free(paths: *mut sg_path_arena) {
    drop(unsafe { Box::from_raw(paths) })
}

/// Manages the state of a collection of partial paths to be used in the path-stitching algorithm.
pub struct sg_partial_path_arena {
    pub inner: PartialPaths,
}

/// Creates a new, initially empty partial path arena.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_new() -> *mut sg_partial_path_arena {
    Box::into_raw(Box::new(sg_partial_path_arena {
        inner: PartialPaths::new(),
    }))
}

/// Frees a path arena, and all of its contents.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_free(partials: *mut sg_partial_path_arena) {
    drop(unsafe { Box::from_raw(partials) })
}

/// Contains a "database" of partial paths.
///
/// This type is meant to be a lazily loaded "view" into a proper storage layer.  During the
/// path-stitching algorithm, we repeatedly try to extend a currently incomplete path with any
/// partial paths that are compatible with it.  For large codebases, or projects with a large
/// number of dependencies, it can be prohibitive to load in _all_ of the partial paths up-front.
/// We've written the path-stitching algorithm so that you have a chance to only load in the
/// partial paths that are actually needed, placing them into a sg_partial_path_database instance
/// as they're needed.
pub struct sg_partial_path_database {
    pub inner: Database,
}

/// Creates a new, initially empty partial path database.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_new() -> *mut sg_partial_path_database {
    Box::into_raw(Box::new(sg_partial_path_database {
        inner: Database::new(),
    }))
}

/// Frees a partial path database, and all of its contents.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_free(db: *mut sg_partial_path_database) {
    drop(unsafe { Box::from_raw(db) })
}

/// The handle of an empty list.
pub const SG_LIST_EMPTY_HANDLE: u32 = 0xffffffff;

/// Describes in which direction the content of a deque is stored in memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum sg_deque_direction {
    SG_DEQUE_FORWARDS,
    SG_DEQUE_BACKWARDS,
}

impl Default for sg_deque_direction {
    fn default() -> sg_deque_direction {
        sg_deque_direction::SG_DEQUE_FORWARDS
    }
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

impl Into<Handle<File>> for sg_file_handle {
    fn into(self) -> Handle<File> {
        unsafe { std::mem::transmute(self) }
    }
}

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
#[derive(Clone, Copy, Eq, PartialEq)]
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

impl Into<Handle<Node>> for sg_node_handle {
    fn into(self) -> Handle<Node> {
        unsafe { std::mem::transmute(self) }
    }
}

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
    pub precedence: i32,
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
        graph.add_edge(source, sink, edges[i].precedence);
    }
}

//-------------------------------------------------------------------------------------------------
// Symbol stacks

/// A symbol with a possibly empty list of exported scopes attached to it.
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct sg_scoped_symbol {
    pub symbol: sg_symbol_handle,
    pub scopes: sg_scope_stack,
}

impl Into<ScopedSymbol> for sg_scoped_symbol {
    fn into(self) -> ScopedSymbol {
        unsafe { std::mem::transmute(self) }
    }
}

/// A sequence of symbols that describe what we are currently looking for while in the middle of
/// the path-finding algorithm.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_symbol_stack {
    /// The handle of the first element in the symbol stack, or SG_LIST_EMPTY_HANDLE if the list is
    /// empty, or 0 if the list is null.
    pub cells: sg_symbol_stack_cell_handle,
    pub length: usize,
}

impl From<SymbolStack> for sg_symbol_stack {
    fn from(stack: SymbolStack) -> sg_symbol_stack {
        unsafe { std::mem::transmute(stack) }
    }
}

/// A handle to an element of a symbol stack.  A zero handle represents a missing symbol stack.  A
/// UINT32_MAX handle represents an empty symbol stack.
pub type sg_symbol_stack_cell_handle = u32;

/// An element of a symbol stack.
#[repr(C)]
pub struct sg_symbol_stack_cell {
    /// The scoped symbol at this position in the symbol stack.
    pub head: sg_scoped_symbol,
    /// The handle of the next element in the symbol stack, or SG_LIST_EMPTY_HANDLE if this is the
    /// last element.
    pub tail: sg_symbol_stack_cell_handle,
}

/// The array of all of the symbol stack content in a path arena.
#[repr(C)]
pub struct sg_symbol_stack_cells {
    pub cells: *const sg_symbol_stack_cell,
    pub count: usize,
}

/// Returns a reference to the array of symbol stack content in a path arena.  The resulting array
/// pointer is only valid until the next call to any function that mutates the path arena.
#[no_mangle]
pub extern "C" fn sg_path_arena_symbol_stack_cells(
    paths: *const sg_path_arena,
) -> sg_symbol_stack_cells {
    let paths = unsafe { &(*paths).inner };
    sg_symbol_stack_cells {
        cells: paths.symbol_stacks.as_ptr() as *const sg_symbol_stack_cell,
        count: paths.symbol_stacks.len(),
    }
}

/// Adds new symbol stacks to the path arena.  `count` is the number of symbol stacks you want to
/// create.  The content of each symbol stack comes from two arrays.  The `lengths` array must have
/// `count` elements, and provides the number of symbols in each symbol stack.  The `symbols` array
/// contains the contents of each of these symbol stacks in one contiguous array.  Its length must
/// be the sum of all of the counts in the `lengths` array.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_symbol_stack` instances for each symbol stack that is created.
#[no_mangle]
pub extern "C" fn sg_path_arena_add_symbol_stacks(
    paths: *mut sg_path_arena,
    count: usize,
    mut symbols: *const sg_scoped_symbol,
    lengths: *const usize,
    out: *mut sg_symbol_stack,
) {
    let paths = unsafe { &mut (*paths).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let symbols_slice = unsafe { std::slice::from_raw_parts(symbols, length) };
        let mut stack = SymbolStack::empty();
        for j in (0..length).rev() {
            let symbol = symbols_slice[j].into();
            stack.push_front(paths, symbol);
        }
        out[i] = stack.into();
        unsafe { symbols = symbols.add(length) };
    }
}

//-------------------------------------------------------------------------------------------------
// Scope stacks

/// A sequence of exported scopes, used to pass name-binding context around a stack graph.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_scope_stack {
    /// The handle of the first element in the scope stack, or SG_LIST_EMPTY_HANDLE if the list is
    /// empty, or 0 if the list is null.
    pub cells: sg_scope_stack_cell_handle,
}

impl From<ScopeStack> for sg_scope_stack {
    fn from(stack: ScopeStack) -> sg_scope_stack {
        unsafe { std::mem::transmute(stack) }
    }
}

/// A handle to an element of a scope stack.  A zero handle represents a missing scope stack.  A
/// UINT32_MAX handle represents an empty scope stack.
pub type sg_scope_stack_cell_handle = u32;

/// An element of a scope stack.
#[repr(C)]
pub struct sg_scope_stack_cell {
    /// The exported scope at this position in the scope stack.
    pub head: sg_node_handle,
    /// The handle of the next element in the scope stack, or SG_LIST_EMPTY_HANDLE if this is the
    /// last element.
    pub tail: sg_scope_stack_cell_handle,
}

/// The array of all of the scope stack content in a path arena.
#[repr(C)]
pub struct sg_scope_stack_cells {
    pub cells: *const sg_scope_stack_cell,
    pub count: usize,
}

/// Returns a reference to the array of scope stack content in a path arena.  The resulting array
/// pointer is only valid until the next call to any function that mutates the path arena.
#[no_mangle]
pub extern "C" fn sg_path_arena_scope_stack_cells(
    paths: *const sg_path_arena,
) -> sg_scope_stack_cells {
    let paths = unsafe { &(*paths).inner };
    sg_scope_stack_cells {
        cells: paths.scope_stacks.as_ptr() as *const sg_scope_stack_cell,
        count: paths.scope_stacks.len(),
    }
}

/// Adds new scope stacks to the path arena.  `count` is the number of scope stacks you want to
/// create.  The content of each scope stack comes from two arrays.  The `lengths` array must have
/// `count` elements, and provides the number of scopes in each scope stack.  The `scopes` array
/// contains the contents of each of these scope stacks in one contiguous array.  Its length must
/// be the sum of all of the counts in the `lengths` array.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_scope_stack` instances for each scope stack that is created.
#[no_mangle]
pub extern "C" fn sg_path_arena_add_scope_stacks(
    paths: *mut sg_path_arena,
    count: usize,
    mut scopes: *const sg_node_handle,
    lengths: *const usize,
    out: *mut sg_scope_stack,
) {
    let paths = unsafe { &mut (*paths).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let scopes_slice = unsafe { std::slice::from_raw_parts(scopes, length) };
        let mut stack = ScopeStack::empty();
        for j in (0..length).rev() {
            let node = scopes_slice[j].into();
            stack.push_front(paths, node);
        }
        out[i] = stack.into();
        unsafe { scopes = scopes.add(length) };
    }
}

//-------------------------------------------------------------------------------------------------
// Edge lists

/// Details about one of the edges in a name-binding path
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct sg_path_edge {
    pub source_node_id: sg_node_id,
    pub precedence: i32,
}

impl Into<PathEdge> for sg_path_edge {
    fn into(self) -> PathEdge {
        unsafe { std::mem::transmute(self) }
    }
}

/// The edges in a path keep track of precedence information so that we can correctly handle
/// shadowed definitions.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_path_edge_list {
    /// The handle of the first element in the edge list, or SG_LIST_EMPTY_HANDLE if the list is
    /// empty, or 0 if the list is null.
    pub cells: sg_path_edge_list_cell_handle,
    pub direction: sg_deque_direction,
    pub length: usize,
}

impl From<PathEdgeList> for sg_path_edge_list {
    fn from(edges: PathEdgeList) -> sg_path_edge_list {
        unsafe { std::mem::transmute(edges) }
    }
}

/// A handle to an element of a path edge list.  A zero handle represents a missing path edge list.
/// A UINT32_MAX handle represents an empty path edge list.
pub type sg_path_edge_list_cell_handle = u32;

/// An element of a path edge list.
#[repr(C)]
pub struct sg_path_edge_list_cell {
    /// The path edge at this position in the path edge list.
    pub head: sg_path_edge,
    /// The handle of the next element in the path edge list, or SG_LIST_EMPTY_HANDLE if this is
    /// the last element.
    pub tail: sg_path_edge_list_cell_handle,
    /// The handle of the reversal of this list.
    pub reversed: sg_path_edge_list_cell_handle,
}

/// The array of all of the path edge list content in a path arena.
#[repr(C)]
pub struct sg_path_edge_list_cells {
    pub cells: *const sg_path_edge_list_cell,
    pub count: usize,
}

/// Returns a reference to the array of path edge list content in a path arena.  The resulting
/// array pointer is only valid until the next call to any function that mutates the path arena.
#[no_mangle]
pub extern "C" fn sg_path_arena_path_edge_list_cells(
    paths: *const sg_path_arena,
) -> sg_path_edge_list_cells {
    let paths = unsafe { &(*paths).inner };
    sg_path_edge_list_cells {
        cells: paths.path_edges.as_ptr() as *const sg_path_edge_list_cell,
        count: paths.path_edges.len(),
    }
}

/// Adds new path edge lists to the path arena.  `count` is the number of path edge lists you want
/// to create.  The content of each path edge list comes from two arrays.  The `lengths` array must
/// have `count` elements, and provides the number of edges in each path edge list.  The `edges`
/// array contains the contents of each of these path edge lists in one contiguous array.  Its
/// length must be the sum of all of the counts in the `lengths` array.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_path_edge_list` instances for each path edge list that is
/// created.
#[no_mangle]
pub extern "C" fn sg_path_arena_add_path_edge_lists(
    paths: *mut sg_path_arena,
    count: usize,
    mut edges: *const sg_path_edge,
    lengths: *const usize,
    out: *mut sg_path_edge_list,
) {
    let paths = unsafe { &mut (*paths).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let edges_slice = unsafe { std::slice::from_raw_parts(edges, length) };
        let mut list = PathEdgeList::empty();
        for j in 0..length {
            let edge: PathEdge = edges_slice[j].into();
            list.push_back(paths, edge);
        }
        // We pushed the edges onto the list in reverse order.  Requesting a forwards iterator
        // before we return ensures that it will also be available in forwards order.
        let _ = list.iter(paths);
        out[i] = list.into();
        unsafe { edges = edges.add(length) };
    }
}

//-------------------------------------------------------------------------------------------------
// Paths

/// A sequence of edges from a stack graph.  A _complete_ path represents a full name binding in a
/// source language.
#[repr(C)]
pub struct sg_path {
    pub start_node: sg_node_handle,
    pub end_node: sg_node_handle,
    pub symbol_stack: sg_symbol_stack,
    pub scope_stack: sg_scope_stack,
    pub edges: sg_path_edge_list,
}

/// A list of paths found by the path-finding algorithm.
#[derive(Default)]
pub struct sg_path_list {
    paths: Vec<Path>,
}

/// Creates a new, empty sg_path_list.
#[no_mangle]
pub extern "C" fn sg_path_list_new() -> *mut sg_path_list {
    Box::into_raw(Box::new(sg_path_list::default()))
}

#[no_mangle]
pub extern "C" fn sg_path_list_free(path_list: *mut sg_path_list) {
    drop(unsafe { Box::from_raw(path_list) });
}

#[no_mangle]
pub extern "C" fn sg_path_list_count(path_list: *const sg_path_list) -> usize {
    let path_list = unsafe { &*path_list };
    path_list.paths.len()
}

#[no_mangle]
pub extern "C" fn sg_path_list_paths(path_list: *const sg_path_list) -> *const sg_path {
    let path_list = unsafe { &*path_list };
    path_list.paths.as_ptr() as *const _
}

/// Finds all complete paths reachable from a set of starting nodes, placing the result into the
/// `path_list` output parameter.  You must free the path list when you are done with it by calling
/// `sg_path_list_done`.
///
/// This function will not return until all reachable paths have been processed, so `graph` must
/// already contain a complete stack graph.  If you have a very large stack graph stored in some
/// other storage system, and want more control over lazily loading only the necessary pieces, then
/// you should use sg_forward_path_stitcher.
#[no_mangle]
pub extern "C" fn sg_path_arena_find_all_complete_paths(
    graph: *const sg_stack_graph,
    paths: *mut sg_path_arena,
    starting_node_count: usize,
    starting_nodes: *const sg_node_handle,
    path_list: *mut sg_path_list,
) {
    let graph = unsafe { &(*graph).inner };
    let paths = unsafe { &mut (*paths).inner };
    let starting_nodes = unsafe { std::slice::from_raw_parts(starting_nodes, starting_node_count) };
    let path_list = unsafe { &mut *path_list };
    paths.find_all_paths(
        graph,
        starting_nodes.iter().copied().map(sg_node_handle::into),
        |graph, _paths, path| {
            if path.is_complete(graph) {
                path_list.paths.push(path);
            }
        },
    );
}

//-------------------------------------------------------------------------------------------------
// Partial symbol stacks

/// A symbol with an unknown, but possibly empty, list of exported scopes attached to it.
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct sg_partial_scoped_symbol {
    pub symbol: sg_symbol_handle,
    pub scopes: sg_partial_scope_stack,
}

impl Into<PartialScopedSymbol> for sg_partial_scoped_symbol {
    fn into(self) -> PartialScopedSymbol {
        unsafe { std::mem::transmute(self) }
    }
}

/// A pattern that might match against a symbol stack.  Consists of a (possibly empty) list of
/// partial scoped symbols.
///
/// (Note that unlike partial scope stacks, we don't store any "symbol stack variable" here.  We
/// could!  But with our current path-finding rules, every partial path will always have exactly
/// one symbol stack variable, which will appear at the end of its precondition and postcondition.
/// So for simplicity we just leave it out.  At some point in the future we might add it in so that
/// the symbol and scope stack formalisms and implementations are more obviously symmetric.)
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_partial_symbol_stack {
    /// The handle of the first element in the partial symbol stack, or SG_LIST_EMPTY_HANDLE if the
    /// list is empty, or 0 if the list is null.
    pub cells: sg_partial_symbol_stack_cell_handle,
    pub direction: sg_deque_direction,
}

impl From<PartialSymbolStack> for sg_partial_symbol_stack {
    fn from(stack: PartialSymbolStack) -> sg_partial_symbol_stack {
        unsafe { std::mem::transmute(stack) }
    }
}

/// A handle to an element of a partial symbol stack.  A zero handle represents a missing partial
/// symbol stack.  A UINT32_MAX handle represents an empty partial symbol stack.
pub type sg_partial_symbol_stack_cell_handle = u32;

/// An element of a partial symbol stack.
#[repr(C)]
pub struct sg_partial_symbol_stack_cell {
    /// The partial scoped symbol at this position in the partial symbol stack.
    pub head: sg_partial_scoped_symbol,
    /// The handle of the next element in the partial symbol stack, or SG_LIST_EMPTY_HANDLE if this
    /// is the last element.
    pub tail: sg_partial_symbol_stack_cell_handle,
    /// The handle of the reversal of this partial scope stack.
    pub reversed: sg_partial_symbol_stack_cell_handle,
}

/// The array of all of the partial symbol stack content in a partial path arena.
#[repr(C)]
pub struct sg_partial_symbol_stack_cells {
    pub cells: *const sg_partial_symbol_stack_cell,
    pub count: usize,
}

/// Returns a reference to the array of partial symbol stack content in a partial path arena.  The
/// resulting array pointer is only valid until the next call to any function that mutates the path
/// arena.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_partial_symbol_stack_cells(
    partials: *const sg_partial_path_arena,
) -> sg_partial_symbol_stack_cells {
    let partials = unsafe { &(*partials).inner };
    sg_partial_symbol_stack_cells {
        cells: partials.partial_symbol_stacks.as_ptr() as *const sg_partial_symbol_stack_cell,
        count: partials.partial_symbol_stacks.len(),
    }
}

/// Adds new partial symbol stacks to the partial path arena.  `count` is the number of partial
/// symbol stacks you want to create.  The content of each partial symbol stack comes from two
/// arrays.  The `lengths` array must have `count` elements, and provides the number of symbols in
/// each partial symbol stack.  The `symbols` array contains the contents of each of these partial
/// symbol stacks in one contiguous array.  Its length must be the sum of all of the counts in the
/// `lengths` array.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_partial_symbol_stack` instances for each partial symbol stack
/// that is created.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_add_partial_symbol_stacks(
    partials: *mut sg_partial_path_arena,
    count: usize,
    mut symbols: *const sg_partial_scoped_symbol,
    lengths: *const usize,
    out: *mut sg_partial_symbol_stack,
) {
    let partials = unsafe { &mut (*partials).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let symbols_slice = unsafe { std::slice::from_raw_parts(symbols, length) };
        let mut stack = PartialSymbolStack::empty();
        for j in 0..length {
            let symbol = symbols_slice[j].into();
            stack.push_back(partials, symbol);
        }
        // We pushed the edges onto the list in reverse order.  Requesting a forwards iterator
        // before we return ensures that it will also be available in forwards order.
        let _ = stack.iter(partials);
        out[i] = stack.into();
        unsafe { symbols = symbols.add(length) };
    }
}

//-------------------------------------------------------------------------------------------------
// Partial scope stacks

/// Represents an unknown list of exported scopes.
pub type sg_scope_stack_variable = u32;

/// A pattern that might match against a scope stack.  Consists of a (possibly empty) list of
/// exported scopes, along with an optional scope stack variable.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_partial_scope_stack {
    /// The handle of the first element in the partial scope stack, or SG_LIST_EMPTY_HANDLE if the
    /// list is empty, or 0 if the list is null.
    pub cells: sg_partial_scope_stack_cell_handle,
    pub direction: sg_deque_direction,
    /// The scope stack variable representing the unknown content of a partial scope stack, or 0 if
    /// the variable is missing.  (If so, this partial scope stack can only match a scope stack
    /// with exactly the list of scopes in `cells`, instead of any scope stack with those scopes as
    /// a prefix.)
    pub variable: sg_scope_stack_variable,
}

impl From<PartialScopeStack> for sg_partial_scope_stack {
    fn from(stack: PartialScopeStack) -> sg_partial_scope_stack {
        unsafe { std::mem::transmute(stack) }
    }
}

/// A handle to an element of a partial scope stack.  A zero handle represents a missing partial
/// scope stack.  A UINT32_MAX handle represents an empty partial scope stack.
pub type sg_partial_scope_stack_cell_handle = u32;

/// An element of a partial scope stack.
#[repr(C)]
pub struct sg_partial_scope_stack_cell {
    /// The exported scope at this position in the partial scope stack.
    pub head: sg_node_handle,
    /// The handle of the next element in the partial scope stack, or SG_LIST_EMPTY_HANDLE if this
    /// is the last element.
    pub tail: sg_path_edge_list_cell_handle,
    /// The handle of the reversal of this partial scope stack.
    pub reversed: sg_path_edge_list_cell_handle,
}

/// The array of all of the partial scope stack content in a partial path arena.
#[repr(C)]
pub struct sg_partial_scope_stack_cells {
    pub cells: *const sg_partial_scope_stack_cell,
    pub count: usize,
}

/// Returns a reference to the array of partial scope stack content in a partial path arena.  The
/// resulting array pointer is only valid until the next call to any function that mutates the
/// partial path arena.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_partial_scope_stack_cells(
    partials: *const sg_partial_path_arena,
) -> sg_partial_scope_stack_cells {
    let partials = unsafe { &(*partials).inner };
    sg_partial_scope_stack_cells {
        cells: partials.partial_scope_stacks.as_ptr() as *const sg_partial_scope_stack_cell,
        count: partials.partial_scope_stacks.len(),
    }
}

/// Adds new partial scope stacks to the partial path arena.  `count` is the number of partial
/// scope stacks you want to create.  The content of each partial scope stack comes from three
/// arrays.  The `lengths` array must have `count` elements, and provides the number of scopes in
/// each scope stack.  The `scopes` array contains the contents of each of these scope stacks in
/// one contiguous array.  Its length must be the sum of all of the counts in the `lengths` array.
/// The `variables` array must have `count` elements, and provides the optional scope stack
/// variable for each partial scope stack.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_partial_scope_stack` instances for each partial scope stack
/// that is created.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_add_partial_scope_stacks(
    partials: *mut sg_partial_path_arena,
    count: usize,
    mut scopes: *const sg_node_handle,
    lengths: *const usize,
    variables: *const sg_scope_stack_variable,
    out: *mut sg_partial_scope_stack,
) {
    let partials = unsafe { &mut (*partials).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let variables = unsafe { std::slice::from_raw_parts(variables, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let scopes_slice = unsafe { std::slice::from_raw_parts(scopes, length) };
        let mut stack = if variables[i] == 0 {
            PartialScopeStack::empty()
        } else {
            PartialScopeStack::from_variable(variables[i].try_into().unwrap())
        };
        for j in 0..length {
            let node = scopes_slice[j].into();
            stack.push_back(partials, node);
        }
        // We pushed the edges onto the list in reverse order.  Requesting a forwards iterator
        // before we return ensures that it will also be available in forwards order.
        let _ = stack.iter_scopes(partials);
        out[i] = stack.into();
        unsafe { scopes = scopes.add(length) };
    }
}

//-------------------------------------------------------------------------------------------------
// Partial edge lists

/// Details about one of the edges in a partial path
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct sg_partial_path_edge {
    pub source_node_id: sg_node_id,
    pub precedence: i32,
}

impl Into<PartialPathEdge> for sg_partial_path_edge {
    fn into(self) -> PartialPathEdge {
        unsafe { std::mem::transmute(self) }
    }
}

/// The edges in a path keep track of precedence information so that we can correctly handle
/// shadowed definitions.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_partial_path_edge_list {
    /// The handle of the first element in the edge list, or SG_LIST_EMPTY_HANDLE if the list is
    /// empty, or 0 if the list is null.
    pub cells: sg_partial_path_edge_list_cell_handle,
    pub direction: sg_deque_direction,
    pub length: usize,
}

impl From<PartialPathEdgeList> for sg_partial_path_edge_list {
    fn from(edges: PartialPathEdgeList) -> sg_partial_path_edge_list {
        unsafe { std::mem::transmute(edges) }
    }
}

/// A handle to an element of a partial path edge list.  A zero handle represents a missing partial
/// path edge list.  A UINT32_MAX handle represents an empty partial path edge list.
pub type sg_partial_path_edge_list_cell_handle = u32;

/// An element of a partial path edge list.
#[repr(C)]
pub struct sg_partial_path_edge_list_cell {
    /// The partial path edge at this position in the partial path edge list.
    pub head: sg_partial_path_edge,
    /// The handle of the next element in the partial path edge list, or SG_LIST_EMPTY_HANDLE if
    /// this is the last element.
    pub tail: sg_partial_path_edge_list_cell_handle,
    /// The handle of the reversal of this list.
    pub reversed: sg_partial_path_edge_list_cell_handle,
}

/// The array of all of the partial path edge list content in a partial path arena.
#[repr(C)]
pub struct sg_partial_path_edge_list_cells {
    pub cells: *const sg_partial_path_edge_list_cell,
    pub count: usize,
}

/// Returns a reference to the array of partial path edge list content in a partial path arena.
/// The resulting array pointer is only valid until the next call to any function that mutates the
/// partial path arena.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_partial_path_edge_list_cells(
    partials: *const sg_partial_path_arena,
) -> sg_partial_path_edge_list_cells {
    let partials = unsafe { &(*partials).inner };
    sg_partial_path_edge_list_cells {
        cells: partials.partial_path_edges.as_ptr() as *const sg_partial_path_edge_list_cell,
        count: partials.partial_path_edges.len(),
    }
}

/// Adds new partial path edge lists to the partial path arena.  `count` is the number of partial
/// path edge lists you want to create.  The content of each partial path edge list comes from two
/// arrays.  The `lengths` array must have `count` elements, and provides the number of edges in
/// each partial path edge list.  The `edges` array contains the contents of each of these partial
/// path edge lists in one contiguous array.  Its length must be the sum of all of the counts in
/// the `lengths` array.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_partial_path_edge_list` instances for each partial path edge
/// list that is created.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_add_partial_path_edge_lists(
    partials: *mut sg_partial_path_arena,
    count: usize,
    mut edges: *const sg_partial_path_edge,
    lengths: *const usize,
    out: *mut sg_partial_path_edge_list,
) {
    let partials = unsafe { &mut (*partials).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let edges_slice = unsafe { std::slice::from_raw_parts(edges, length) };
        let mut list = PartialPathEdgeList::empty();
        for j in 0..length {
            let edge: PartialPathEdge = edges_slice[j].into();
            list.push_back(partials, edge);
        }
        // We pushed the edges onto the list in reverse order.  Requesting a forwards iterator
        // before we return ensures that it will also be available in forwards order.
        let _ = list.iter(partials);
        out[i] = list.into();
        unsafe { edges = edges.add(length) };
    }
}

//-------------------------------------------------------------------------------------------------
// Partial paths

/// A portion of a name-binding path.
///
/// Partial paths can be computed _incrementally_, in which case all of the edges in the partial
/// path belong to a single file.  At query time, we can efficiently concatenate partial paths to
/// yield a name-binding path.
///
/// Paths describe the contents of the symbol stack and scope stack at the end of the path.
/// Partial paths, on the other hand, have _preconditions_ and _postconditions_ for each stack.
/// The precondition describes what the stack must look like for us to be able to concatenate this
/// partial path onto the end of a path.  The postcondition describes what the resulting stack
/// looks like after doing so.
///
/// The preconditions can contain _scope stack variables_, which describe parts of the scope stack
/// (or parts of a scope symbol's attached scope list) whose contents we don't care about.  The
/// postconditions can _also_ refer to those variables, and describe how those variable parts of
/// the input scope stacks are carried through unmodified into the resulting scope stack.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sg_partial_path {
    pub start_node: sg_node_handle,
    pub end_node: sg_node_handle,
    pub symbol_stack_precondition: sg_partial_symbol_stack,
    pub symbol_stack_postcondition: sg_partial_symbol_stack,
    pub scope_stack_precondition: sg_partial_scope_stack,
    pub scope_stack_postcondition: sg_partial_scope_stack,
    pub edges: sg_partial_path_edge_list,
}

impl Into<PartialPath> for sg_partial_path {
    fn into(self) -> PartialPath {
        unsafe { std::mem::transmute(self) }
    }
}

/// A list of paths found by the path-finding algorithm.
#[derive(Default)]
pub struct sg_partial_path_list {
    partial_paths: Vec<PartialPath>,
}

/// Creates a new, empty sg_partial_path_list.
#[no_mangle]
pub extern "C" fn sg_partial_path_list_new() -> *mut sg_partial_path_list {
    Box::into_raw(Box::new(sg_partial_path_list::default()))
}

#[no_mangle]
pub extern "C" fn sg_partial_path_list_free(partial_path_list: *mut sg_partial_path_list) {
    drop(unsafe { Box::from_raw(partial_path_list) });
}

#[no_mangle]
pub extern "C" fn sg_partial_path_list_count(
    partial_path_list: *const sg_partial_path_list,
) -> usize {
    let partial_path_list = unsafe { &*partial_path_list };
    partial_path_list.partial_paths.len()
}

#[no_mangle]
pub extern "C" fn sg_partial_path_list_paths(
    partial_path_list: *const sg_partial_path_list,
) -> *const sg_partial_path {
    let partial_path_list = unsafe { &*partial_path_list };
    partial_path_list.partial_paths.as_ptr() as *const _
}

/// Finds all partial paths in a file that are _productive_ and _as complete as possible_, placing
/// the result into the `partial_path_list` output parameter.  You must free the path list when you
/// are done with it by calling `sg_partial_path_list_done`.
///
/// This function will not return until all reachable paths have been processed, so `graph` must
/// already contain a complete stack graph.  If you have a very large stack graph stored in some
/// other storage system, and want more control over lazily loading only the necessary pieces, then
/// you should use sg_forward_path_stitcher.
#[no_mangle]
pub extern "C" fn sg_partial_path_arena_find_partial_paths_in_file(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    file: sg_file_handle,
    partial_path_list: *mut sg_partial_path_list,
) {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let file = file.into();
    let partial_path_list = unsafe { &mut *partial_path_list };
    partials.find_all_partial_paths_in_file(graph, file, |graph, partials, mut path| {
        if !path.is_complete_as_possible(graph) {
            return;
        }
        if !path.is_productive(partials) {
            return;
        }
        path.ensure_both_directions(partials);
        partial_path_list.partial_paths.push(path);
    });
}

/// A handle to a partial path in a partial path database.  A zero handle represents a missing
/// partial path.
pub type sg_partial_path_handle = u32;

/// Adds new partial paths to the partial path database.  `paths` is the array of partial paths
/// that you want to add; `count` is the number of them.
///
/// We copy the partial path content into the partial path database.  The array you pass in does
/// not need to outlive the call to this function.
///
/// You should take care not to add a partial path to the database multiple times.  This won't
/// cause an _error_, in that nothing will break, but it will probably cause you to get duplicate
/// paths from the path-stitching algorithm.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_add_partial_paths(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    db: *mut sg_partial_path_database,
    count: usize,
    paths: *const sg_partial_path,
) {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let db = unsafe { &mut (*db).inner };
    let paths = unsafe { std::slice::from_raw_parts(paths, count) };
    for i in 0..count {
        db.add_partial_path(graph, partials, paths[i].into());
    }
}

//-------------------------------------------------------------------------------------------------
// Path stitching

/// Implements a phased forward path-stitching algorithm.
///
/// Our overall goal is to start with a set of _seed_ paths, and to repeatedly extend each path by
/// appending a compatible partial path onto the end of it.  (If there are multiple compatible
/// partial paths, we append each of them separately, resulting in more than one extension for the
/// current path.)
///
/// We perform this processing in _phases_.  At the start of each phase, we have a _current set_ of
/// paths that need to be processed.  As we extend those paths, we add the extensions to the set of
/// paths to process in the _next_ phase.  Phases are processed one at a time, each time you invoke
/// `sg_forward_path_stitcher_process_next_phase`.
///
/// After each phase has completed, the `previous_phase_paths` and `previous_phase_paths_length`
/// fields give you all of the paths that were discovered during that phase.  That gives you a
/// chance to add to the `sg_partial_path_database` all of the partial paths that we might need to
/// extend those paths with before invoking the next phase.
#[repr(C)]
pub struct sg_forward_path_stitcher {
    /// The new candidate paths that were discovered in the most recent phase.
    pub previous_phase_paths: *const sg_path,
    /// The number of new candidate paths that were discovered in the most recent phase.  If this
    /// is 0, then the path stitching algorithm is complete.
    pub previous_phase_paths_length: usize,
}

// This is the Rust equivalent of a common C trick, where you have two versions of a struct — a
// publicly visible one and a private one containing internal implementation details.  In our case,
// `sg_forward_path_stitcher` is the public struct, and `ForwardPathStitcher` is the internal one.
// The main requirement is that the private struct must start with a copy of all of the fields in
// the public struct — ensuring that those fields occur at the same offset in both.  The private
// struct can contain additional (private) fields, but they must appear _after_ all of the publicly
// visible fields.
//
// In our case, we do this because we don't want to expose the existence or details of the
// PathStitcher type via the C API.
#[repr(C)]
struct ForwardPathStitcher {
    previous_phase_paths: *const Path,
    previous_phase_paths_length: usize,
    stitcher: PathStitcher,
}

impl ForwardPathStitcher {
    fn new(stitcher: PathStitcher) -> ForwardPathStitcher {
        let mut this = ForwardPathStitcher {
            previous_phase_paths: std::ptr::null(),
            previous_phase_paths_length: 0,
            stitcher,
        };
        this.update_previous_phase_paths();
        this
    }

    fn update_previous_phase_paths(&mut self) {
        let slice = self.stitcher.previous_phase_paths_slice();
        self.previous_phase_paths = slice.as_ptr();
        self.previous_phase_paths_length = slice.len();
    }
}

/// Creates a new forward path stitcher that is "seeded" with a set of starting stack graph nodes.
///
/// Before calling this method, you must ensure that `db` contains all of the possible partial
/// paths that start with any of your requested starting nodes.
///
/// Before calling `sg_forward_path_stitcher_process_next_phase` for the first time, you must
/// ensure that `db` contains all possible extensions of any of those initial paths.  You can
/// retrieve a list of those extensions via the `previous_phase_paths` and
/// `previous_phase_paths_length` fields.
#[no_mangle]
pub extern "C" fn sg_forward_path_stitcher_new(
    graph: *const sg_stack_graph,
    paths: *mut sg_path_arena,
    partials: *mut sg_partial_path_arena,
    db: *mut sg_partial_path_database,
    count: usize,
    starting_nodes: *const sg_node_handle,
) -> *mut sg_forward_path_stitcher {
    let graph = unsafe { &(*graph).inner };
    let paths = unsafe { &mut (*paths).inner };
    let partials = unsafe { &mut (*partials).inner };
    let db = unsafe { &mut (*db).inner };
    let starting_nodes = unsafe { std::slice::from_raw_parts(starting_nodes, count) };
    let stitcher = PathStitcher::new(
        graph,
        paths,
        partials,
        db,
        starting_nodes.iter().copied().map(sg_node_handle::into),
    );
    Box::into_raw(Box::new(ForwardPathStitcher::new(stitcher))) as *mut _
}

/// Runs the next phase of the path-stitching algorithm.  We will have built up a set of
/// incomplete paths during the _previous_ phase.  Before calling this function, you must
/// ensure that `db` contains all of the possible partial paths that we might want to extend
/// any of those paths with.
///
/// After this method returns, you can retrieve a list of the (possibly incomplete) paths that were
/// encountered during this phase via the `previous_phase_paths` and `previous_phase_paths_length`
/// fields.
#[no_mangle]
pub extern "C" fn sg_forward_path_stitcher_process_next_phase(
    graph: *const sg_stack_graph,
    paths: *mut sg_path_arena,
    partials: *mut sg_partial_path_arena,
    db: *mut sg_partial_path_database,
    stitcher: *mut sg_forward_path_stitcher,
) {
    let graph = unsafe { &(*graph).inner };
    let paths = unsafe { &mut (*paths).inner };
    let partials = unsafe { &mut (*partials).inner };
    let db = unsafe { &mut (*db).inner };
    let stitcher = unsafe { &mut *(stitcher as *mut ForwardPathStitcher) };
    stitcher
        .stitcher
        .process_next_phase(graph, paths, partials, db);
    stitcher.update_previous_phase_paths();
}

/// Frees a forward path stitcher.
#[no_mangle]
pub extern "C" fn sg_forward_path_stitcher_free(stitcher: *mut sg_forward_path_stitcher) {
    drop(unsafe { Box::from_raw(stitcher as *mut ForwardPathStitcher) });
}
