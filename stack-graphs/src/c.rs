// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines a C API for working with stack graphs in other languages.

#![allow(non_camel_case_types)]

use std::convert::TryInto;
use std::sync::atomic::AtomicUsize;

use libc::c_char;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::InternedString;
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
use crate::stitching::Database;
use crate::stitching::DatabaseCandidates;
use crate::stitching::ForwardPartialPathStitcher;
use crate::stitching::GraphEdgeCandidates;
use crate::stitching::StitcherConfig;
use crate::CancellationError;
use crate::CancellationFlag;

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

/// The null value for all of our handles.
pub const SG_NULL_HANDLE: u32 = 0;

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

/// Ensures all partial paths in the database are availabe in both forwards and backwards orientation.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_ensure_both_directions(
    db: *mut sg_partial_path_database,
    partials: *mut sg_partial_path_arena,
) {
    let db = unsafe { &mut (*db).inner };
    let partials = unsafe { &mut (*partials).inner };
    db.ensure_both_directions(partials);
}

/// Ensures all partial paths in the database are in forwards orientation.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_ensure_forwards(
    db: *mut sg_partial_path_database,
    partials: *mut sg_partial_path_arena,
) {
    let db = unsafe { &mut (*db).inner };
    let partials = unsafe { &mut (*partials).inner };
    db.ensure_forwards(partials);
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

/// Adds new symbols to the stack graph.  You provide all of the symbol content concatenated
/// together into a single string, and an array of the lengths of each symbol.  You also provide an
/// output array, which must have the same size as `lengths`.  We will place each symbol's handle
/// in the output array.
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
    symbols: *const c_char,
    lengths: *const usize,
    handles_out: *mut sg_symbol_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let mut symbols = symbols as *const u8;
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let handles_out = unsafe {
        std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<Symbol>>, count)
    };
    for i in 0..count {
        let symbol = unsafe { std::slice::from_raw_parts(symbols, lengths[i]) };
        handles_out[i] = match std::str::from_utf8(symbol) {
            Ok(symbol) => Some(graph.add_symbol(symbol)),
            Err(_) => None,
        };
        unsafe { symbols = symbols.add(lengths[i]) };
    }
}

//-------------------------------------------------------------------------------------------------
// Interned strings

/// Arbitrary string content associated with some part of a stack graph.
#[repr(C)]
pub struct sg_string {
    pub content: *const c_char,
    pub length: usize,
}

/// A handle to an interned string in a stack graph.  A zero handle represents a missing string.
///
/// We deduplicate strings in a stack graph — that is, we ensure that there are never multiple
/// `struct sg_string` instances with the same content.  That means that you can compare string
/// handles using simple equality, without having to dereference them.
pub type sg_string_handle = u32;

/// An array of all of the interned strings in a stack graph.  String handles are indices into this
/// array. There will never be a valid string at index 0; a handle with the value 0 represents a
/// missing string.
#[repr(C)]
pub struct sg_strings {
    pub strings: *const sg_string,
    pub count: usize,
}

/// Returns a reference to the array of string data in this stack graph.  The resulting array
/// pointer is only valid until the next call to any function that mutates the stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_strings(graph: *const sg_stack_graph) -> sg_strings {
    let graph = unsafe { &(*graph).inner };
    sg_strings {
        strings: graph.strings.as_ptr() as *const sg_string,
        count: graph.strings.len(),
    }
}

/// Adds new strings to the stack graph.  You provide all of the string content concatenated
/// together into a single string, and an array of the lengths of each string.  You also provide an
/// output array, which must have the same size as `lengths`.  We will place each string's handle
/// in the output array.
///
/// We ensure that there is only ever one copy of a particular string stored in the graph — we
/// guarantee that identical strings will have the same handles, meaning that you can compare the
/// handles using simple integer equality.
///
/// We copy the string data into the stack graph.  The string content you pass in does not need to
/// outlive the call to this function.
///
/// Each string must be a valid UTF-8 string.  If any string isn't valid UTF-8, it won't be added
/// to the stack graph, and the corresponding entry in the output array will be the null handle.
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_strings(
    graph: *mut sg_stack_graph,
    count: usize,
    strings: *const c_char,
    lengths: *const usize,
    handles_out: *mut sg_string_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let mut strings = strings as *const u8;
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let handles_out = unsafe {
        std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<InternedString>>, count)
    };
    for i in 0..count {
        let string = unsafe { std::slice::from_raw_parts(strings, lengths[i]) };
        handles_out[i] = match std::str::from_utf8(string) {
            Ok(string) => Some(graph.add_string(string)),
            Err(_) => None,
        };
        unsafe { strings = strings.add(lengths[i]) };
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

/// Adds new files to the stack graph.  You provide all of the file content concatenated together
/// into a single string, and an array of the lengths of each file.  You also provide an output
/// array, which must have the same size as `lengths`.  We will place each file's handle in the
/// output array.
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
    files: *const c_char,
    lengths: *const usize,
    handles_out: *mut sg_file_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let mut files = files as *const u8;
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let handles_out =
        unsafe { std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<File>>, count) };
    for i in 0..count {
        let file = unsafe { std::slice::from_raw_parts(files, lengths[i]) };
        handles_out[i] = match std::str::from_utf8(file) {
            Ok(file) => Some(graph.get_or_create_file(file)),
            Err(_) => None,
        };
        unsafe { files = files.add(lengths[i]) };
    }
}

//-------------------------------------------------------------------------------------------------
// Nodes

/// Uniquely identifies a node in a stack graph.
///
/// Each node (except for the _root node_ and _jump to scope_ node) lives in a file, and has a
/// _local ID_ that must be unique within its file.
#[repr(C)]
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct sg_node_id {
    pub file: sg_file_handle,
    pub local_id: u32,
}

impl sg_node_id {
    fn is_empty(self) -> bool {
        self.file == 0 && self.local_id == 0
    }
}

impl Into<NodeID> for sg_node_id {
    fn into(self) -> NodeID {
        unsafe { std::mem::transmute(self) }
    }
}

/// The local_id of the singleton root node.
pub const SG_ROOT_NODE_ID: u32 = 1;

/// The local_id of the singleton "jump to scope" node.
pub const SG_JUMP_TO_NODE_ID: u32 = 2;

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
    pub scope: sg_node_id,
    /// Whether this node is an endpoint.  For push nodes, this indicates that the node represents
    /// a reference in the source.  For pop nodes, this indicates that the node represents a
    /// definition in the source.  For scopes, this indicates that the scope is exported. For all
    /// other node types, this field will be unused.
    pub is_endpoint: bool,
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
    /// A node that adds structure to the graph. If the node is exported, it can be
    /// referred to on the scope stack, which allows "jump to" nodes in any other
    /// part of the graph can jump back here.
    SG_NODE_KIND_SCOPE,
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
/// If you try to add a new node that has the same ID as an existing node in the stack graph, the
/// new node will be ignored, and the corresponding entry in the `handles_out` array will contain
/// the handle of the _existing_ node with that ID.
///
/// If any node that you pass in is invalid, it will not be added to the graph, and the
/// corresponding entry in the `handles_out` array will be null.
#[no_mangle]
pub extern "C" fn sg_stack_graph_get_or_create_nodes(
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
        handles_out[i] = validate_node(graph, &nodes[i])
            .map(|node| graph.get_or_create_node(node_id.into(), node));
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
    if (node.scope.is_empty())
        == matches!(&node.kind, sg_node_kind::SG_NODE_KIND_PUSH_SCOPED_SYMBOL)
    {
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
// Source info

/// Contains information about a range of code in a source code file.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct sg_source_info {
    /// The location in its containing file of the source code that this node represents.
    pub span: sg_span,
    /// The kind of syntax entity this node represents (e.g. `function`, `class`, `method`, etc.).
    pub syntax_type: sg_string_handle,
    /// The full content of the line containing this node in its source file.
    pub containing_line: sg_string_handle,
    /// The location in its containing file of the source code that this node's definiens represents.
    /// This is used for things like the bodies of functions, rather than the RHSes of equations.
    /// If you need one of these to make the type checker happy, but you don't have one, just use
    /// sg_span::default(), as this will correspond to the all-0s spans which mean "no definiens".
    pub definiens_span: sg_span,
    /// The fully qualified name is a representation of the symbol that captures its name and its
    /// embedded context (e.g. `foo.bar` for the symbol `bar` defined in the module `foo`).
    pub fully_qualified_name: sg_string_handle,
}

/// All of the position information that we have about a range of content in a source file
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct sg_span {
    pub start: sg_position,
    pub end: sg_position,
}

/// All of the position information that we have about a character in a source file
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct sg_position {
    /// The 0-indexed line number containing the character
    pub line: usize,
    /// The offset of the character within its containing line, expressed as both a UTF-8 byte
    /// index and a UTF-16 code point index
    pub column: sg_offset,
    /// The UTF-8 byte indexes (within the file) of the start and end of the line containing the
    /// character
    pub containing_line: sg_utf8_bounds,
    /// The UTF-8 byte indexes (within the file) of the start and end of the line containing the
    /// character, with any leading and trailing whitespace removed
    pub trimmed_line: sg_utf8_bounds,
}

/// The offset of a character within a string (typically a line of source code), using several
/// different units
///
/// All offsets are 0-indexed.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct sg_offset {
    /// The number of UTF-8-encoded bytes appearing before this character in the string
    pub utf8_offset: usize,
    /// The number of UTF-16 code units appearing before this character in the string
    pub utf16_offset: usize,
    /// The number of graphemes appearing before this character in the string
    pub grapheme_offset: usize,
}

/// A half-open range identifying a range of characters in a string.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct sg_utf8_bounds {
    /// The UTF-8 byte index of the first character in the range.
    pub start: usize,
    /// The UTF-8 byte index of the first character _after_ the range.
    pub end: usize,
}

/// An array of all of the source information in a stack graph.  Source information is associated
/// with nodes, so node handles are indices into this array.  It is _not_ guaranteed that there
/// will an entry in this array for every node handle; if you have a node handle whose value is
/// larger than `count`, then use a 0-valued `sg_source_info` if you need source information for
/// that node.
///
/// There will never be a valid entry at index 0; a handle with the value 0 represents a missing
/// node.
#[repr(C)]
pub struct sg_source_infos {
    pub infos: *const sg_source_info,
    pub count: usize,
}

/// Returns a reference to the array of source information in this stack graph.  The resulting
/// array pointer is only valid until the next call to any function that mutates the stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_source_infos(graph: *const sg_stack_graph) -> sg_source_infos {
    let graph = unsafe { &(*graph).inner };
    sg_source_infos {
        infos: graph.source_info.as_ptr() as *const sg_source_info,
        count: graph.source_info.len(),
    }
}

/// A tuple of a node handle and source information for that node.  Used with the
/// `sg_add_source_info` function to add source information to a stack graph.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sg_node_source_info {
    pub node: sg_node_handle,
    pub source_info: sg_source_info,
}

/// Adds new source information to the stack graph.  You provide an array of `sg_node_source_info`
/// instances.  Any existing source information for any node mentioned in the array is overwritten.
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_source_infos(
    graph: *mut sg_stack_graph,
    count: usize,
    infos: *const sg_node_source_info,
) {
    let graph = unsafe { &mut (*graph).inner };
    let infos = unsafe { std::slice::from_raw_parts(infos, count) };
    for i in 0..count {
        let node = unsafe { std::mem::transmute(infos[i].node) };
        let info = graph.source_info_mut(node);
        *info = unsafe { std::mem::transmute(infos[i].source_info) };
    }
}

//-------------------------------------------------------------------------------------------------
// Partial symbol stacks

/// Represents an unknown list of scoped symbols.
pub type sg_symbol_stack_variable = u32;

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
    pub length: u32,
    /// The symbol stack variable representing the unknown content of a partial symbol stack, or 0
    /// if the variable is missing.  (If so, this partial symbol stack can only match a symbol
    /// stack with exactly the list of symbols in `cells`, instead of any symbol stack with those
    /// symbols as a prefix.)
    pub variable: sg_symbol_stack_variable,
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
/// `lengths` array.  The `variables` array must have `count` elements, and provides the optional
/// symbol stack variable for each partial symbol stack.
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
    variables: *const sg_symbol_stack_variable,
    out: *mut sg_partial_symbol_stack,
) {
    let partials = unsafe { &mut (*partials).inner };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let variables = unsafe { std::slice::from_raw_parts(variables, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, count) };
    for i in 0..count {
        let length = lengths[i];
        let symbols_slice = unsafe { std::slice::from_raw_parts(symbols, length) };
        let mut stack = if variables[i] == 0 {
            PartialSymbolStack::empty()
        } else {
            PartialSymbolStack::from_variable(variables[i].try_into().unwrap())
        };
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
    pub length: u32,
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
    pub tail: sg_partial_scope_stack_cell_handle,
    /// The handle of the reversal of this partial scope stack.
    pub reversed: sg_partial_scope_stack_cell_handle,
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
    pub length: u32,
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
    stitcher_config: *const sg_stitcher_config,
    cancellation_flag: *const usize,
) -> sg_result {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let file = file.into();
    let partial_path_list = unsafe { &mut *partial_path_list };
    let stitcher_config = unsafe { *stitcher_config };
    let cancellation_flag: Option<&AtomicUsize> =
        unsafe { std::mem::transmute(cancellation_flag.as_ref()) };
    ForwardPartialPathStitcher::find_minimal_partial_path_set_in_file(
        graph,
        partials,
        file,
        stitcher_config.into(),
        &AtomicUsizeCancellationFlag(cancellation_flag),
        |_graph, partials, path| {
            let mut path = path.clone();
            path.ensure_both_directions(partials);
            partial_path_list.partial_paths.push(path);
        },
    )
    .into()
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
pub extern "C" fn sg_partial_path_arena_find_all_complete_paths(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    starting_node_count: usize,
    starting_nodes: *const sg_node_handle,
    path_list: *mut sg_partial_path_list,
    stitcher_config: *const sg_stitcher_config,
    cancellation_flag: *const usize,
) -> sg_result {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let starting_nodes = unsafe { std::slice::from_raw_parts(starting_nodes, starting_node_count) };
    let stitcher_config = unsafe { *stitcher_config };
    let path_list = unsafe { &mut *path_list };
    let cancellation_flag: Option<&AtomicUsize> =
        unsafe { std::mem::transmute(cancellation_flag.as_ref()) };
    ForwardPartialPathStitcher::find_all_complete_partial_paths(
        &mut GraphEdgeCandidates::new(graph, partials, None),
        starting_nodes.iter().copied().map(sg_node_handle::into),
        stitcher_config.into(),
        &AtomicUsizeCancellationFlag(cancellation_flag),
        |graph, _partials, path| {
            if path.is_complete(graph) {
                path_list.partial_paths.push(path.clone());
            }
        },
    )
    .into()
}

/// A handle to a partial path in a partial path database.  A zero handle represents a missing
/// partial path.
pub type sg_partial_path_handle = u32;

/// An array of all of the partial paths in a partial path database.  Partial path handles are
/// indices into this array.  There will never be a valid partial path at index 0; a handle with
/// the value 0 represents a missing partial path.
#[repr(C)]
pub struct sg_partial_paths {
    pub paths: *const sg_partial_path,
    pub count: usize,
}

/// Returns a reference to the array of partial path data in this partial path database.  The
/// resulting array pointer is only valid until the next call to any function that mutates the
/// partial path database.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_partial_paths(
    db: *const sg_partial_path_database,
) -> sg_partial_paths {
    let db = unsafe { &(*db).inner };
    sg_partial_paths {
        paths: db.partial_paths.as_ptr() as *const sg_partial_path,
        count: db.partial_paths.len(),
    }
}

/// Adds new partial paths to the partial path database.  `paths` is the array of partial paths
/// that you want to add; `count` is the number of them.
///
/// We copy the partial path content into the partial path database.  The array you pass in does
/// not need to outlive the call to this function.
///
/// You should take care not to add a partial path to the database multiple times.  This won't
/// cause an _error_, in that nothing will break, but it will probably cause you to get duplicate
/// paths from the path-stitching algorithm.
///
/// You must also provide an `out` array, which must also have room for `count` elements.  We will
/// fill this array in with the `sg_partial_path_edge_list` instances for each partial path edge
/// list that is created.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_add_partial_paths(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    db: *mut sg_partial_path_database,
    count: usize,
    paths: *const sg_partial_path,
    out: *mut sg_partial_path_handle,
) {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let db = unsafe { &mut (*db).inner };
    let paths = unsafe { std::slice::from_raw_parts(paths, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(out as *mut Handle<PartialPath>, count) };
    for i in 0..count {
        out[i] = db.add_partial_path(graph, partials, paths[i].into());
    }
}

//-------------------------------------------------------------------------------------------------
// Local nodes

/// Encodes a set of node handles.
///
/// The elements are encoded in a bit set.  Use the traditional mask and shift pattern to determine
/// if a particular handle is in the set:
///
/// ``` c
/// size_t element_index = handle / 32;
/// size_t bit_index = handle % 32;
/// size_t bit_mask = 1 << bit_index;
/// bool bit_is_set =
///     element_index < set.length &&
///     (set.elements[element_index] & bit_mask) != 0;
/// ```
#[repr(C)]
pub struct sg_node_handle_set {
    pub elements: *const u32,
    /// Note that this is the number of uint32_t's in `elements`, NOT the number of bits in the set.
    pub length: usize,
}

/// Determines which nodes in the stack graph are “local”, taking into account the partial paths in
/// this database.  The result is valid until the next call to this function, or until the database
/// is freed.
///
/// A local node has no partial path that connects it to the root node in either direction. That
/// means that it cannot participate in any paths that leave the file.
///
/// This method is meant to be used at index time, to calculate the set of nodes that are local
/// after having just calculated the set of partial paths for the file.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_find_local_nodes(db: *mut sg_partial_path_database) {
    let db = unsafe { &mut (*db).inner };
    db.find_local_nodes();
}

/// Marks that a list of stack graph nodes are local.
///
/// This method is meant to be used at query time.  You will have precalculated the set of local
/// nodes for a file at index time; at query time, you will load this information from your storage
/// layer and use this method to update our internal view of which nodes are local.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_mark_local_nodes(
    db: *mut sg_partial_path_database,
    count: usize,
    nodes: *const sg_node_handle,
) {
    let db = unsafe { &mut (*db).inner };
    let nodes = unsafe { std::slice::from_raw_parts(nodes, count) };
    for node in nodes {
        db.mark_local_node(node.clone().into());
    }
}

/// Returns a reference to the set of stack graph nodes that are local, according to this database
/// of partial paths.  The resulting set is only valid until the next call to any function that
/// mutates the partial path database.
#[no_mangle]
pub extern "C" fn sg_partial_path_database_local_nodes(
    db: *const sg_partial_path_database,
) -> sg_node_handle_set {
    let db = unsafe { &(*db).inner };
    sg_node_handle_set {
        elements: db.local_nodes.as_ptr(),
        length: db.local_nodes.len(),
    }
}

//-------------------------------------------------------------------------------------------------
// Forward partial path stitching

/// Implements a phased forward partial path stitching algorithm.
///
/// Our overall goal is to start with a set of _seed_ partial paths, and to repeatedly extend each
/// partial path by concatenating another, compatible partial path onto the end of it.  (If there
/// are multiple compatible partial paths, we concatenate each of them separately, resulting in
/// more than one extension for the current path.)
///
/// We perform this processing in _phases_.  At the start of each phase, we have a _current set_ of
/// partial paths that need to be processed.  As we extend those partial paths, we add the
/// extensions to the set of partial paths to process in the _next_ phase.  Phases are processed
/// one at a time, each time you invoke `sg_forward_partial_path_stitcher_process_next_phase`.
///
/// After each phase has completed, the `previous_phase_paths` and `previous_phase_paths_length`
/// fields give you all of the partial paths that were discovered during that phase.  That gives
/// you a chance to add to the `sg_partial_path_database` all of the other partial paths that we
/// might need to extend those partial paths with before invoking the next phase.
#[repr(C)]
pub struct sg_forward_partial_path_stitcher {
    /// The new candidate partial paths that were discovered in the most recent phase.
    pub previous_phase_partial_paths: *const sg_partial_path,
    /// The number of new candidate partial paths that were discovered in the most recent phase.
    /// If this is 0, then the partial path stitching algorithm is complete.
    pub previous_phase_partial_paths_length: usize,
    /// Whether the stitching algorithm is complete.  You should keep calling
    /// `sg_forward_partial_path_stitcher_process_next_phase` until this field is true.
    pub is_complete: bool,
}

// Configuration for partial path stitchers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sg_stitcher_config {
    /// Enables similar path detection during stiching.
    pub detect_similar_paths: bool,
}

impl Into<StitcherConfig> for sg_stitcher_config {
    fn into(self) -> StitcherConfig {
        StitcherConfig::default().with_detect_similar_paths(self.detect_similar_paths)
    }
}

// This is the Rust equivalent of a common C trick, where you have two versions of a struct — a
// publicly visible one and a private one containing internal implementation details.  In our case,
// `sg_forward_partial_path_stitcher` is the public struct, and
// `InternalForwardPartialPathStitcher` is the internal one.  The main requirement is that the
// private struct must start with a copy of all of the fields in the public struct — ensuring that
// those fields occur at the same offset in both.  The private struct can contain additional
// (private) fields, but they must appear _after_ all of the publicly visible fields.
//
// In our case, we do this because we don't want to expose the existence or details of the
// ForwardPartialPathStitcher type via the C API.
#[repr(C)]
struct InternalForwardPartialPathStitcher {
    previous_phase_partial_paths: *const PartialPath,
    previous_phase_partial_paths_length: usize,
    is_complete: bool,
    stitcher: ForwardPartialPathStitcher<Handle<PartialPath>>,
}

impl InternalForwardPartialPathStitcher {
    fn new(
        stitcher: ForwardPartialPathStitcher<Handle<PartialPath>>,
        partials: &mut PartialPaths,
    ) -> InternalForwardPartialPathStitcher {
        let mut this = InternalForwardPartialPathStitcher {
            previous_phase_partial_paths: std::ptr::null(),
            previous_phase_partial_paths_length: 0,
            is_complete: false,
            stitcher,
        };
        this.update_previous_phase_partial_paths(partials);
        this
    }

    fn update_previous_phase_partial_paths(&mut self, partials: &mut PartialPaths) {
        for path in self.stitcher.previous_phase_partial_paths_slice_mut() {
            path.ensure_both_directions(partials);
        }
        let slice = self.stitcher.previous_phase_partial_paths_slice();
        self.previous_phase_partial_paths = slice.as_ptr();
        self.previous_phase_partial_paths_length = slice.len();
        self.is_complete = self.stitcher.is_complete();
    }
}

/// Creates a new forward partial path stitcher that is "seeded" with a set of starting stack graph
/// nodes. The path stitcher will be set up to find complete paths only.
#[no_mangle]
pub extern "C" fn sg_forward_partial_path_stitcher_from_nodes(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    count: usize,
    starting_nodes: *const sg_node_handle,
) -> *mut sg_forward_partial_path_stitcher {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let starting_nodes = unsafe { std::slice::from_raw_parts(starting_nodes, count) };
    let initial_paths = starting_nodes
        .iter()
        .copied()
        .map(sg_node_handle::into)
        .map(|n| {
            let mut p = PartialPath::from_node(graph, partials, n);
            p.eliminate_precondition_stack_variables(partials);
            p
        })
        .collect::<Vec<_>>();
    let stitcher = ForwardPartialPathStitcher::from_partial_paths(graph, partials, initial_paths);
    Box::into_raw(Box::new(InternalForwardPartialPathStitcher::new(
        stitcher, partials,
    ))) as *mut _
}

/// Creates a new forward partial path stitcher that is "seeded" with a set of initial partial
/// paths.
#[no_mangle]
pub extern "C" fn sg_forward_partial_path_stitcher_from_partial_paths(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    count: usize,
    initial_partial_paths: *const sg_partial_path,
) -> *mut sg_forward_partial_path_stitcher {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let initial_partial_paths =
        unsafe { std::slice::from_raw_parts(initial_partial_paths as *const PartialPath, count) };
    let stitcher = ForwardPartialPathStitcher::from_partial_paths(
        graph,
        partials,
        initial_partial_paths.to_vec(),
    );
    Box::into_raw(Box::new(InternalForwardPartialPathStitcher::new(
        stitcher, partials,
    ))) as *mut _
}

/// Sets whether similar path detection should be enabled during path stitching. Paths are similar
/// if start and end node, and pre- and postconditions are the same. The presence of similar paths
/// can lead to exponential blow up during path stitching. Similar path detection is disabled by
/// default because of the accociated preformance cost.
#[no_mangle]
pub extern "C" fn sg_forward_partial_path_stitcher_set_similar_path_detection(
    stitcher: *mut sg_forward_partial_path_stitcher,
    detect_similar_paths: bool,
) {
    let stitcher = unsafe { &mut *(stitcher as *mut InternalForwardPartialPathStitcher) };
    stitcher
        .stitcher
        .set_similar_path_detection(detect_similar_paths);
}

/// Sets the maximum amount of work that can be performed during each phase of the algorithm. By
/// bounding our work this way, you can ensure that it's not possible for our CPU-bound algorithm
/// to starve any worker threads or processes that you might be using.  If you don't call this
/// method, then we allow ourselves to process all of the extensions of all of the paths found in
/// the previous phase, with no additional bound.
#[no_mangle]
pub extern "C" fn sg_forward_partial_path_stitcher_set_max_work_per_phase(
    stitcher: *mut sg_forward_partial_path_stitcher,
    max_work: usize,
) {
    let stitcher = unsafe { &mut *(stitcher as *mut InternalForwardPartialPathStitcher) };
    stitcher.stitcher.set_max_work_per_phase(max_work);
}

/// Runs the next phase of the algorithm.  We will have built up a set of incomplete partial paths
/// during the _previous_ phase.  Before calling this function, you must ensure that `db` contains
/// all of the possible partial paths that we might want to extend any of those candidate partial
/// paths with.
///
/// After this method returns, you can retrieve a list of the (possibly incomplete) partial paths
/// that were encountered during this phase via the `previous_phase_partial_paths` and
/// `previous_phase_partial_paths_length` fields.
#[no_mangle]
pub extern "C" fn sg_forward_partial_path_stitcher_process_next_phase(
    graph: *const sg_stack_graph,
    partials: *mut sg_partial_path_arena,
    db: *mut sg_partial_path_database,
    stitcher: *mut sg_forward_partial_path_stitcher,
) {
    let graph = unsafe { &(*graph).inner };
    let partials = unsafe { &mut (*partials).inner };
    let db = unsafe { &mut (*db).inner };
    let stitcher = unsafe { &mut *(stitcher as *mut InternalForwardPartialPathStitcher) };
    stitcher.stitcher.process_next_phase(
        &mut DatabaseCandidates::new(graph, partials, db),
        |_, _, _| true,
    );
    stitcher.update_previous_phase_partial_paths(partials);
}

/// Frees a forward path stitcher.
#[no_mangle]
pub extern "C" fn sg_forward_partial_path_stitcher_free(
    stitcher: *mut sg_forward_partial_path_stitcher,
) {
    drop(unsafe { Box::from_raw(stitcher as *mut InternalForwardPartialPathStitcher) });
}

//-------------------------------------------------------------------------------------------------
// Cancellation

struct AtomicUsizeCancellationFlag<'a>(Option<&'a AtomicUsize>);
impl CancellationFlag for AtomicUsizeCancellationFlag<'_> {
    fn check(&self, at: &'static str) -> Result<(), crate::CancellationError> {
        self.0
            .map(|flag| {
                if flag.fetch_and(0b0, std::sync::atomic::Ordering::Relaxed) != 0 {
                    Err(CancellationError(at))
                } else {
                    Ok(())
                }
            })
            .unwrap_or(Ok(()))
    }
}

/// Describes the result of a computation
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum sg_result {
    SG_RESULT_SUCCESS,
    SG_RESULT_CANCELLED,
}

impl<T> From<Result<T, CancellationError>> for sg_result {
    fn from(result: Result<T, CancellationError>) -> Self {
        match result {
            Ok(_) => Self::SG_RESULT_SUCCESS,
            Err(_) => Self::SG_RESULT_CANCELLED,
        }
    }
}
