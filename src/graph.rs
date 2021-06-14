// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines the structure of a stack graph.
//!
//! This module contains all of the types that you need to define the structure of a particular
//! stack graph.
//!
//! The stack graph as a whole lives in an instance of [`StackGraph`][].  This type contains
//! several [`Arena`s][`Arena`], which are used to manage the life cycle of the data instances that
//! comprise the stack graph.  You cannot delete anything from the stack graph; all of its contents
//! are dropped in a single operation when the graph itself is dropped.
//!
//! [`Arena`]: ../arena/struct.Arena.html
//! [`StackGraph`]: struct.StackGraph.html
//!
//! There are several different kinds of node that can appear in a stack graph.  As we search for a
//! path representing a name binding, each kind of node has different rules for how it interacts
//! with the symbol and scope stacks:
//!
//!   - the singleton [_root node_][`RootNode`], which allows name binding paths to cross between
//!     files
//!   - [_exported_][`ExportedScopeNode`] and [_internal_][`InternalScopeNode`] _scopes_, which
//!     define the name binding structure within a single file
//!   - [_push symbol_][`PushSymbolNode`] and [_push scoped symbol_][`PushScopedSymbolNode`] nodes,
//!     which push onto the symbol stack new things for us to look for
//!   - [_pop symbol_][`PopSymbolNode`] and [_pop scoped symbol_][`PopScopedSymbolNode`] nodes,
//!     which pop things off the symbol stack once they've been found
//!   - [_drop scopes_][`DropScopesNode`] and [_jump to scope_][`JumpToNode`] nodes, which
//!     manipulate the scope stack
//!
//! [`DropScopesNode`]: struct.DropScopesNode.html
//! [`ExportedScopeNode`]: struct.ExportedScopeNode.html
//! [`InternalScopeNode`]: struct.InternalScopeNode.html
//! [`JumpToNode`]: struct.JumpToNode.html
//! [`PushScopedSymbolNode`]: struct.PushScopedSymbolNode.html
//! [`PushSymbolNode`]: struct.PushSymbolNode.html
//! [`PopScopedSymbolNode`]: struct.PopScopedSymbolNode.html
//! [`PopSymbolNode`]: struct.PopSymbolNode.html
//! [`RootNode`]: struct.RootNode.html
//!
//! All nodes except for the singleton _root node_ and _jump to scope_ node belong to
//! [files][`File`].
//!
//! Nodes are connected via [edges][`Edge`].
//!
//! [`Edge`]: struct.Edge.html
//! [`File`]: struct.File.html

use std::fmt::Display;
use std::ops::Index;
use std::ops::IndexMut;

use either::Either;
use fxhash::FxHashMap;
use smallvec::SmallVec;

use crate::arena::Arena;
use crate::arena::Handle;
use crate::arena::SupplementalArena;

//-------------------------------------------------------------------------------------------------
// String content

struct InternedString {
    // See InternedStringContent below for how we fill in these fields safely.
    start: *const u8,
    len: usize,
}

const INITIAL_STRING_CAPACITY: usize = 512;

/// The content of each `InternedString` is stored in one of the buffers inside of a
/// `InternedStringContent` instance, following the trick [described by Aleksey Kladov][interner].
///
/// The buffers stored in this type are preallocated, and are never allowed to grow.  That ensures
/// that pointers into the buffer are stable, as long as the buffer has not been destroyed.
/// (`InternedString` instances are also stored in an arena, ensuring that the strings that we hand
/// out don't outlive the buffers.)
///
/// [interner]: https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html
struct InternedStringContent {
    current_buffer: Vec<u8>,
    full_buffers: Vec<Vec<u8>>,
}

impl InternedStringContent {
    fn new() -> InternedStringContent {
        InternedStringContent {
            current_buffer: Vec::with_capacity(INITIAL_STRING_CAPACITY),
            full_buffers: Vec::new(),
        }
    }

    // Adds a new string.  This does not check whether we've already stored a string with the same
    // content; that is handled down below in `StackGraph::add_symbol` and `add_file`.
    fn add(&mut self, value: &str) -> InternedString {
        // Is there enough room in current_buffer to hold this string?
        let value = value.as_bytes();
        let len = value.len();
        let capacity = self.current_buffer.capacity();
        let remaining_capacity = capacity - self.current_buffer.len();
        if len > remaining_capacity {
            // If not, move current_buffer over into full_buffers (so that we hang onto it until
            // we're dropped) and allocate a new current_buffer that's at least big enough to hold
            // this string.
            let new_capacity = (capacity.max(len) + 1).next_power_of_two();
            let new_buffer = Vec::with_capacity(new_capacity);
            let old_buffer = std::mem::replace(&mut self.current_buffer, new_buffer);
            self.full_buffers.push(old_buffer);
        }

        // Copy the string's content into current_buffer and return a pointer to it.  That pointer
        // is stable since we never allow the current_buffer to be resized — once we run out of
        // room, we allocate a _completely new buffer_ to replace it.
        let start_index = self.current_buffer.len();
        self.current_buffer.extend_from_slice(value);
        let start = &self.current_buffer[start_index] as *const _;
        InternedString { start, len }
    }
}

impl InternedString {
    /// Returns the content of this string as a `str`.  This is safe as long as the lifetime of the
    /// InternedString is outlived by the lifetime of the InternedStringContent that holds its
    /// data.  That is guaranteed because we store the InternedStrings in an Arena alongside the
    /// InternedStringContent, and only hand out references to them.
    fn as_str(&self) -> &str {
        unsafe {
            let bytes = std::slice::from_raw_parts(self.start, self.len);
            std::str::from_utf8_unchecked(bytes)
        }
    }

    // Returns a supposedly 'static reference to the string's data.  The string data isn't really
    // static, but we are careful only to use this as a key in the HashMap that StackGraph uses to
    // track whether we've stored a particular symbol already.  That HashMap lives alongside the
    // InternedStringContent that holds the data, so we can get away with a technically incorrect
    // 'static lifetime here.  As an extra precaution, this method is is marked as unsafe so that
    // we don't inadvertently call it from anywhere else in the crate.
    unsafe fn as_hash_key(&self) -> &'static str {
        let bytes = std::slice::from_raw_parts(self.start, self.len);
        std::str::from_utf8_unchecked(bytes)
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
///
/// We deduplicate `Symbol` instances in a `StackGraph` — that is, we ensure that there are never
/// multiple `Symbol` instances with the same content.  That means that you can compare _handles_
/// to symbols using simple equality, without having to dereference into the `StackGraph` arena.
pub struct Symbol {
    content: InternedString,
}

impl Symbol {
    fn as_str(&self) -> &str {
        self.content.as_str()
    }
}

impl PartialEq<&str> for Symbol {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl StackGraph {
    /// Adds a symbol to the stack graph, ensuring that there's only ever one copy of a particular
    /// symbol stored in the graph.
    pub fn add_symbol<S: AsRef<str> + ?Sized>(&mut self, symbol: &S) -> Handle<Symbol> {
        let symbol = symbol.as_ref();
        if let Some(handle) = self.symbol_handles.get(symbol) {
            return *handle;
        }

        let interned = self.interned_strings.add(symbol);
        let hash_key = unsafe { interned.as_hash_key() };
        let handle = self.symbols.add(Symbol { content: interned });
        self.symbol_handles.insert(hash_key, handle);
        handle
    }

    /// Returns an iterator over all of the handles of all of the symbols in this stack graph.
    /// (Note that because we're only returning _handles_, this iterator does not retain a
    /// reference to the `StackGraph`.)
    pub fn iter_symbols(&self) -> impl Iterator<Item = Handle<Symbol>> {
        self.symbols.iter_handles()
    }
}

impl Index<Handle<Symbol>> for StackGraph {
    type Output = str;
    #[inline(always)]
    fn index(&self, handle: Handle<Symbol>) -> &str {
        self.symbols.get(handle).as_str()
    }
}

#[doc(hidden)]
pub struct DisplaySymbol<'a> {
    wrapped: Handle<Symbol>,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplaySymbol<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.graph[self.wrapped])
    }
}

impl Handle<Symbol> {
    pub fn display(self, graph: &StackGraph) -> impl Display + '_ {
        DisplaySymbol {
            wrapped: self,
            graph,
        }
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
pub struct File {
    /// The name of this source file.
    name: InternedString,
}

impl File {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl StackGraph {
    /// Adds a file to the stack graph.  There can only ever be one file with a particular name in
    /// the graph.  If a file with the requested name already exists, we return `Err`; if it
    /// doesn't already exist, we return `Ok`.  In both cases, the value of the result is the
    /// file's handle.
    pub fn add_file<S: AsRef<str> + ?Sized>(
        &mut self,
        name: &S,
    ) -> Result<Handle<File>, Handle<File>> {
        let name = name.as_ref();
        if let Some(handle) = self.file_handles.get(name) {
            return Err(*handle);
        }

        let interned = self.interned_strings.add(name);
        let hash_key = unsafe { interned.as_hash_key() };
        let handle = self.files.add(File { name: interned });
        self.file_handles.insert(hash_key, handle);
        Ok(handle)
    }

    /// Adds a file to the stack graph, returning its handle.  There can only ever be one file with
    /// a particular name in the graph, so if you call this multiple times with the same name,
    /// you'll get the same handle each time.
    #[inline(always)]
    pub fn get_or_create_file<S: AsRef<str> + ?Sized>(&mut self, name: &S) -> Handle<File> {
        self.add_file(name).unwrap_or_else(|handle| handle)
    }

    /// Returns the file with a particular name.  Panics if there is no file with the requested
    /// name.
    pub fn get_file_unchecked<S: AsRef<str> + ?Sized>(&self, name: &S) -> Handle<File> {
        let name = name.as_ref();
        self.file_handles.get(name).copied().expect("Missing file")
    }
}

impl StackGraph {
    /// Returns an iterator of all of the nodes that belong to a particular file.  Note that this
    /// does **_not_** include the singleton _root_ or _jump to scope_ nodes.
    pub fn nodes_for_file(&self, file: Handle<File>) -> impl Iterator<Item = Handle<Node>> + '_ {
        self.node_id_handles.nodes_for_file(file)
    }

    /// Returns an iterator over all of the handles of all of the files in this stack graph.  (Note
    /// that because we're only returning _handles_, this iterator does not retain a reference to
    /// the `StackGraph`.)
    pub fn iter_files(&self) -> impl Iterator<Item = Handle<File>> + '_ {
        self.files.iter_handles()
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Index<Handle<File>> for StackGraph {
    type Output = File;
    #[inline(always)]
    fn index(&self, handle: Handle<File>) -> &File {
        &self.files.get(handle)
    }
}

#[doc(hidden)]
pub struct DisplayFile<'a> {
    wrapped: Handle<File>,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayFile<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.graph[self.wrapped])
    }
}

impl Handle<File> {
    pub fn display(self, graph: &StackGraph) -> impl Display + '_ {
        DisplayFile {
            wrapped: self,
            graph,
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Nodes

/// Uniquely identifies a node in a stack graph.
///
/// Each node (except for the _root node_ and _jump to scope_ node) lives in a file, and has a
/// _local ID_ that must be unique within its file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeID {
    /// The file that the node comes from.
    pub file: Handle<File>,
    /// The unique identity of the node within its file.
    pub local_id: u32,
}

#[doc(hidden)]
pub struct DisplayNodeID<'a> {
    wrapped: NodeID,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayNodeID<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}({})",
            self.wrapped.file.display(self.graph),
            self.wrapped.local_id,
        )
    }
}

impl NodeID {
    pub fn display(self, graph: &StackGraph) -> impl Display + '_ {
        DisplayNodeID {
            wrapped: self,
            graph,
        }
    }
}

/// A node in a stack graph.
pub enum Node {
    DropScopes(DropScopesNode),
    ExportedScope(ExportedScopeNode),
    InternalScope(InternalScopeNode),
    JumpTo(JumpToNode),
    PushScopedSymbol(PushScopedSymbolNode),
    PushSymbol(PushSymbolNode),
    PopScopedSymbol(PopScopedSymbolNode),
    PopSymbol(PopSymbolNode),
    Root(RootNode),
    Unknown(UnknownNode),
}

impl Node {
    #[inline(always)]
    pub fn is_definition(&self) -> bool {
        match self {
            Node::PopScopedSymbol(node) => node.is_definition,
            Node::PopSymbol(node) => node.is_definition,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn is_reference(&self) -> bool {
        match self {
            Node::PushScopedSymbol(node) => node.is_reference,
            Node::PushSymbol(node) => node.is_reference,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn is_jump_to(&self) -> bool {
        matches!(self, Node::JumpTo(_))
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool {
        matches!(self, Node::Root(_))
    }

    /// Returns this node's symbol, if it has one.  (_Pop symbol_, _pop scoped symbol_, _push
    /// symbol_, and _push scoped symbol_ nodes have symbols.)
    pub fn symbol(&self) -> Option<Handle<Symbol>> {
        match self {
            Node::PushScopedSymbol(node) => Some(node.symbol),
            Node::PushSymbol(node) => Some(node.symbol),
            Node::PopScopedSymbol(node) => Some(node.symbol),
            Node::PopSymbol(node) => Some(node.symbol),
            _ => None,
        }
    }

    /// Returns the ID of this node.  Returns `None` for the singleton _root_ and _jump to scope_
    /// nodes, which don't have IDs.
    pub fn id(&self) -> Option<NodeID> {
        match self {
            Node::DropScopes(node) => Some(node.id),
            Node::ExportedScope(node) => Some(node.id),
            Node::InternalScope(node) => Some(node.id),
            Node::PushScopedSymbol(node) => Some(node.id),
            Node::PushSymbol(node) => Some(node.id),
            Node::PopScopedSymbol(node) => Some(node.id),
            Node::PopSymbol(node) => Some(node.id),
            Node::Unknown(node) => Some(node.id),
            _ => None,
        }
    }

    /// Returns the file that this node belongs to.  Returns `None` for the singleton _root_ and
    /// _jump to scope_ nodes, which belong to all files.
    pub fn file(&self) -> Option<Handle<File>> {
        self.id().map(|id| id.file)
    }

    /// Returns whether a node belongs to a particular file.  Always returns `true` for the
    /// singleton _root_ and _jump to scope_ nodes, which belong to all files.
    pub fn is_in_file(&self, file: Handle<File>) -> bool {
        self.file()
            .map(|self_file| self_file == file)
            .unwrap_or(true)
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayNode {
            wrapped: self,
            graph,
        }
    }
}

impl StackGraph {
    /// Returns a handle to the stack graph's singleton _jump to scope_ node.
    #[inline(always)]
    pub fn jump_to_node(&self) -> Handle<Node> {
        self.jump_to_node
    }

    /// Returns a handle to the stack graph's singleton _root node_.
    #[inline(always)]
    pub fn root_node(&self) -> Handle<Node> {
        self.root_node
    }

    /// Returns an unused _local node ID_ for the given file.
    pub fn new_node_id(&mut self, file: Handle<File>) -> NodeID {
        self.node_id_handles.unused_id(file)
    }

    /// Returns an iterator of all of the nodes in the graph.  (Note that because we're only
    /// returning _handles_, this iterator does not retain a reference to the `StackGraph`.)
    pub fn iter_nodes(&self) -> impl Iterator<Item = Handle<Node>> {
        self.nodes.iter_handles()
    }

    /// Returns the handle to the node with a particular ID, if it exists.
    pub fn node_for_id(&self, node_id: NodeID) -> Option<Handle<Node>> {
        self.node_id_handles.try_handle_for_id(node_id)
    }

    fn add_node(&mut self, node_id: NodeID, node: Node) -> Option<Handle<Node>> {
        if let Some(_) = self.node_id_handles.handle_for_id(node_id) {
            return None;
        }
        let handle = self.nodes.add(node);
        self.node_id_handles.set_handle_for_id(node_id, handle);
        Some(handle)
    }
}

#[doc(hidden)]
pub struct DisplayNode<'a> {
    wrapped: &'a Node,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.wrapped {
            Node::DropScopes(node) => node.display(self.graph).fmt(f),
            Node::ExportedScope(node) => node.display(self.graph).fmt(f),
            Node::InternalScope(node) => node.display(self.graph).fmt(f),
            Node::JumpTo(node) => node.fmt(f),
            Node::PushScopedSymbol(node) => node.display(self.graph).fmt(f),
            Node::PushSymbol(node) => node.display(self.graph).fmt(f),
            Node::PopScopedSymbol(node) => node.display(self.graph).fmt(f),
            Node::PopSymbol(node) => node.display(self.graph).fmt(f),
            Node::Root(node) => node.fmt(f),
            Node::Unknown(node) => node.display(self.graph).fmt(f),
        }
    }
}

impl Handle<Node> {
    pub fn display(self, graph: &StackGraph) -> impl Display + '_ {
        DisplayNode {
            wrapped: &graph[self],
            graph,
        }
    }
}

impl Index<Handle<Node>> for StackGraph {
    type Output = Node;
    #[inline(always)]
    fn index(&self, handle: Handle<Node>) -> &Node {
        self.nodes.get(handle)
    }
}

impl IndexMut<Handle<Node>> for StackGraph {
    #[inline(always)]
    fn index_mut(&mut self, handle: Handle<Node>) -> &mut Node {
        self.nodes.get_mut(handle)
    }
}

/// Removes everything from the current scope stack.
pub struct DropScopesNode {
    /// The unique identifier for this node.
    pub id: NodeID,
}

impl From<DropScopesNode> for Node {
    fn from(node: DropScopesNode) -> Node {
        Node::DropScopes(node)
    }
}

impl DropScopesNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayDropScopesNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayDropScopesNode<'a> {
    wrapped: &'a DropScopesNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayDropScopesNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(f, "[{} drop scopes]", self.wrapped.id.display(self.graph))
        }
    }
}

/// A node that can be referred to on the scope stack, which allows "jump to" nodes in any other
/// part of the graph can jump back here.
pub struct ExportedScopeNode {
    /// The unique identifier for this node.
    pub id: NodeID,
}

impl From<ExportedScopeNode> for Node {
    fn from(node: ExportedScopeNode) -> Node {
        Node::ExportedScope(node)
    }
}

impl ExportedScopeNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayExportedScopeNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayExportedScopeNode<'a> {
    wrapped: &'a ExportedScopeNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayExportedScopeNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{} exported scope]",
                self.wrapped.id.display(self.graph),
            )
        }
    }
}

/// A node internal to a single file.  This node has no effect on the symbol or scope stacks;
/// it's just used to add structure to the graph.
pub struct InternalScopeNode {
    /// The unique identifier for this node.
    pub id: NodeID,
}

impl From<InternalScopeNode> for Node {
    fn from(node: InternalScopeNode) -> Node {
        Node::InternalScope(node)
    }
}

impl InternalScopeNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayInternalScopeNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayInternalScopeNode<'a> {
    wrapped: &'a InternalScopeNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayInternalScopeNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{} internal scope]",
                self.wrapped.id.display(self.graph),
            )
        }
    }
}

/// The singleton "jump to" node, which allows a name binding path to jump back to another part of
/// the graph.
pub struct JumpToNode;

impl From<JumpToNode> for Node {
    fn from(node: JumpToNode) -> Node {
        Node::JumpTo(node)
    }
}

impl Display for JumpToNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[jump to scope]")
    }
}

/// Pops a scoped symbol from the symbol stack.  If the top of the symbol stack doesn't match the
/// requested symbol, or if the top of the symbol stack doesn't have an attached scope list, then
/// the path is not allowed to enter this node.
pub struct PopScopedSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to pop off the symbol stack.
    pub symbol: Handle<Symbol>,
    /// Whether this node represents a reference in the source language.
    pub is_definition: bool,
}

impl From<PopScopedSymbolNode> for Node {
    fn from(node: PopScopedSymbolNode) -> Node {
        Node::PopScopedSymbol(node)
    }
}

impl PopScopedSymbolNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayPopScopedSymbolNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayPopScopedSymbolNode<'a> {
    wrapped: &'a PopScopedSymbolNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayPopScopedSymbolNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{} {} {}]",
                self.wrapped.id.display(self.graph),
                if self.wrapped.is_definition {
                    "scoped definition"
                } else {
                    "pop scoped"
                },
                self.wrapped.symbol.display(self.graph),
            )
        }
    }
}

/// Pops a symbol from the symbol stack.  If the top of the symbol stack doesn't match the
/// requested symbol, then the path is not allowed to enter this node.
pub struct PopSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to pop off the symbol stack.
    pub symbol: Handle<Symbol>,
    /// Whether this node represents a reference in the source language.
    pub is_definition: bool,
}

impl From<PopSymbolNode> for Node {
    fn from(node: PopSymbolNode) -> Node {
        Node::PopSymbol(node)
    }
}

impl PopSymbolNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayPopSymbolNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayPopSymbolNode<'a> {
    wrapped: &'a PopSymbolNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayPopSymbolNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{} {} {}]",
                self.wrapped.id.display(self.graph),
                if self.wrapped.is_definition {
                    "definition"
                } else {
                    "pop"
                },
                self.wrapped.symbol.display(self.graph),
            )
        }
    }
}

/// Pushes a scoped symbol onto the symbol stack.
pub struct PushScopedSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to push onto the symbol stack.
    pub symbol: Handle<Symbol>,
    /// The exported scope node that should be attached to the scoped symbol.  The Handle<Node> must
    /// refer to an exported scope node.
    pub scope: Handle<Node>,
    /// Whether this node represents a reference in the source language.
    pub is_reference: bool,
}

impl From<PushScopedSymbolNode> for Node {
    fn from(node: PushScopedSymbolNode) -> Node {
        Node::PushScopedSymbol(node)
    }
}

impl PushScopedSymbolNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayPushScopedSymbolNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayPushScopedSymbolNode<'a> {
    wrapped: &'a PushScopedSymbolNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayPushScopedSymbolNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{} {} {} {}]",
                self.wrapped.id.display(self.graph),
                if self.wrapped.is_reference {
                    "scoped reference"
                } else {
                    "push scoped"
                },
                self.wrapped.symbol.display(self.graph),
                self.wrapped.scope.display(self.graph),
            )
        }
    }
}

/// Pushes a symbol onto the symbol stack.
pub struct PushSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to push onto the symbol stack.
    pub symbol: Handle<Symbol>,
    /// Whether this node represents a reference in the source language.
    pub is_reference: bool,
}

impl From<PushSymbolNode> for Node {
    fn from(node: PushSymbolNode) -> Node {
        Node::PushSymbol(node)
    }
}

impl PushSymbolNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayPushSymbolNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayPushSymbolNode<'a> {
    wrapped: &'a PushSymbolNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayPushSymbolNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{} {} {}]",
                self.wrapped.id.display(self.graph),
                if self.wrapped.is_reference {
                    "reference"
                } else {
                    "push"
                },
                self.wrapped.symbol.display(self.graph),
            )
        }
    }
}

/// The singleton root node, which allows a name binding path to cross between files.
pub struct RootNode;

impl From<RootNode> for Node {
    fn from(node: RootNode) -> Node {
        Node::Root(node)
    }
}

impl Display for RootNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[root]")
    }
}

/// A placeholder for a node that you know needs to exist, but don't yet know what kind of node it
/// will be.  Before you can use the graph, you must use [`resolve_unknown_node`][] to replace this
/// placeholder with a "real" node.
///
/// [`resolve_unknown_node`]: struct.StackGraph.html#method.resolve_unknown_node
pub struct UnknownNode {
    /// The unique identifier for this node.
    pub id: NodeID,
}

impl From<UnknownNode> for Node {
    fn from(node: UnknownNode) -> Node {
        Node::Unknown(node)
    }
}

impl UnknownNode {
    /// Adds the node to a stack graph.
    pub fn add_to_graph(self, graph: &mut StackGraph) -> Option<Handle<Node>> {
        graph.add_node(self.id, self.into())
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayUnknownNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayUnknownNode<'a> {
    wrapped: &'a UnknownNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayUnknownNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(f, "[{} unknown]", self.wrapped.id.display(self.graph))
        }
    }
}

impl StackGraph {
    /// Resolves an _unknown_ node with a "real" node.  Panics if there isn't a node in the arena
    /// with the same ID as `node`.  Returns an error is that node is not an _unknown_ node.
    pub fn resolve_unknown_node(&mut self, node: Node) -> Result<(), &Node> {
        let id = node.id().unwrap();
        let handle = self.node_id_handles.handle_for_id(id).unwrap();
        let arena_node = &mut self[handle];
        if !matches!(arena_node, Node::Unknown(_)) {
            return Err(arena_node);
        }
        *arena_node = node;
        Ok(())
    }
}

struct NodeIDHandles {
    files: SupplementalArena<File, Vec<Option<Handle<Node>>>>,
}

impl NodeIDHandles {
    fn new() -> NodeIDHandles {
        NodeIDHandles {
            files: SupplementalArena::new(),
        }
    }

    fn try_handle_for_id(&self, node_id: NodeID) -> Option<Handle<Node>> {
        let file_entry = self.files.get(node_id.file)?;
        let node_index = node_id.local_id as usize;
        if node_index >= file_entry.len() {
            return None;
        }
        file_entry[node_index]
    }

    fn handle_for_id(&mut self, node_id: NodeID) -> Option<Handle<Node>> {
        let file_entry = &mut self.files[node_id.file];
        let node_index = node_id.local_id as usize;
        if node_index >= file_entry.len() {
            file_entry.resize(node_index + 1, None);
        }
        file_entry[node_index]
    }

    fn set_handle_for_id(&mut self, node_id: NodeID, handle: Handle<Node>) {
        let file_entry = &mut self.files[node_id.file];
        let node_index = node_id.local_id as usize;
        file_entry[node_index] = Some(handle);
    }

    fn unused_id(&mut self, file: Handle<File>) -> NodeID {
        let local_id = self
            .files
            .get(file)
            .map(|file_entry| file_entry.len() as u32)
            .unwrap_or(0);
        NodeID { file, local_id }
    }

    fn nodes_for_file(&self, file: Handle<File>) -> impl Iterator<Item = Handle<Node>> + '_ {
        let file_entry = match self.files.get(file) {
            Some(file_entry) => file_entry,
            None => return Either::Left(std::iter::empty()),
        };
        Either::Right(file_entry.iter().filter_map(|entry| *entry))
    }
}

//-------------------------------------------------------------------------------------------------
// Edges

/// Connects two nodes in a stack graph.
///
/// These edges provide the basic graph connectivity that allow us to search for name binding paths
/// in a stack graph.  (Though not all sequence of edges is a well-formed name binding: the nodes
/// that you encounter along the path must also satisfy all of the rules for maintaining correct
/// symbol and scope stacks.)
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub source: Handle<Node>,
    pub sink: Handle<Node>,
}

impl StackGraph {
    /// Adds a new edge to the stack graph.
    pub fn add_edge(&mut self, edge: Edge) {
        let edges = &mut self.outgoing_edges[edge.source];
        if let Err(index) = edges.binary_search(&edge.sink) {
            edges.insert(index, edge.sink);
        }
    }

    /// Removes an edge from the stack graph.
    pub fn remove_edge(&mut self, edge: Edge) {
        let edges = &mut self.outgoing_edges[edge.source];
        if let Ok(index) = edges.binary_search(&edge.sink) {
            edges.remove(index);
        }
    }

    /// Returns an iterator of all of the edges that begin at a particular source node.
    pub fn outgoing_edges(&self, source: Handle<Node>) -> impl Iterator<Item = Edge> + '_ {
        match self.outgoing_edges.get(source) {
            Some(edges) => Either::Right(edges.iter().map(move |sink| Edge {
                source,
                sink: *sink,
            })),
            None => Either::Left(std::iter::empty()),
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Stack graphs

/// Contains all of the nodes and edges that make up a stack graph.
pub struct StackGraph {
    interned_strings: InternedStringContent,
    symbols: Arena<Symbol>,
    symbol_handles: FxHashMap<&'static str, Handle<Symbol>>,
    files: Arena<File>,
    file_handles: FxHashMap<&'static str, Handle<File>>,
    nodes: Arena<Node>,
    node_id_handles: NodeIDHandles,
    jump_to_node: Handle<Node>,
    root_node: Handle<Node>,
    outgoing_edges: SupplementalArena<Node, SmallVec<[Handle<Node>; 8]>>,
}

impl StackGraph {
    /// Creates a new, initially empty stack graph.
    pub fn new() -> StackGraph {
        let mut nodes = Arena::new();
        let root_node = nodes.add(RootNode.into());
        let jump_to_node = nodes.add(JumpToNode.into());

        StackGraph {
            interned_strings: InternedStringContent::new(),
            symbols: Arena::new(),
            symbol_handles: FxHashMap::default(),
            files: Arena::new(),
            file_handles: FxHashMap::default(),
            nodes,
            node_id_handles: NodeIDHandles::new(),
            jump_to_node,
            root_node,
            outgoing_edges: SupplementalArena::new(),
        }
    }
}
