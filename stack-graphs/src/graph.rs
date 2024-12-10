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
//!   - [_scope_][`ScopeNode`] nodes, which define the name binding structure within a single file
//!   - [_push symbol_][`PushSymbolNode`] and [_push scoped symbol_][`PushScopedSymbolNode`] nodes,
//!     which push onto the symbol stack new things for us to look for
//!   - [_pop symbol_][`PopSymbolNode`] and [_pop scoped symbol_][`PopScopedSymbolNode`] nodes,
//!     which pop things off the symbol stack once they've been found
//!   - [_drop scopes_][`DropScopesNode`] and [_jump to scope_][`JumpToNode`] nodes, which
//!     manipulate the scope stack
//!
//! [`DropScopesNode`]: struct.DropScopesNode.html
//! [`JumpToNode`]: struct.JumpToNode.html
//! [`PushScopedSymbolNode`]: struct.PushScopedSymbolNode.html
//! [`PushSymbolNode`]: struct.PushSymbolNode.html
//! [`PopScopedSymbolNode`]: struct.PopScopedSymbolNode.html
//! [`PopSymbolNode`]: struct.PopSymbolNode.html
//! [`RootNode`]: struct.RootNode.html
//! [`ScopeNode`]: struct.ScopeNode.html
//!
//! All nodes except for the singleton _root node_ and _jump to scope_ node belong to
//! [files][`File`].
//!
//! Nodes are connected via [edges][`Edge`].
//!
//! [`Edge`]: struct.Edge.html
//! [`File`]: struct.File.html

use std::collections::HashMap;
use std::fmt::Display;
use std::num::NonZeroU32;
use std::ops::Index;
use std::ops::IndexMut;

use controlled_option::ControlledOption;
use either::Either;
use fxhash::FxHashMap;
use smallvec::SmallVec;

use crate::arena::Arena;
use crate::arena::Handle;
use crate::arena::SupplementalArena;

//-------------------------------------------------------------------------------------------------
// String content

#[repr(C)]
struct InternedStringContent {
    // See InternedStringArena below for how we fill in these fields safely.
    start: *const u8,
    len: usize,
}

const INITIAL_STRING_CAPACITY: usize = 512;

/// The content of each interned string is stored in one of the buffers inside of a
/// `InternedStringArena` instance, following the trick [described by Aleksey Kladov][interner].
///
/// The buffers stored in this type are preallocated, and are never allowed to grow.  That ensures
/// that pointers into the buffer are stable, as long as the buffer has not been destroyed.
/// (`InternedStringContent` instances are also stored in an arena, ensuring that the strings that
/// we hand out don't outlive the buffers.)
///
/// [interner]: https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html
struct InternedStringArena {
    current_buffer: Vec<u8>,
    full_buffers: Vec<Vec<u8>>,
}

impl InternedStringArena {
    fn new() -> InternedStringArena {
        InternedStringArena {
            current_buffer: Vec::with_capacity(INITIAL_STRING_CAPACITY),
            full_buffers: Vec::new(),
        }
    }

    // Adds a new string.  This does not check whether we've already stored a string with the same
    // content; that is handled down below in `StackGraph::add_symbol` and `add_file`.
    fn add(&mut self, value: &str) -> InternedStringContent {
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
        let start = unsafe { self.current_buffer.as_ptr().add(start_index) };
        InternedStringContent { start, len }
    }
}

impl InternedStringContent {
    /// Returns the content of this string as a `str`.  This is safe as long as the lifetime of the
    /// InternedStringContent is outlived by the lifetime of the InternedStringArena that holds its
    /// data.  That is guaranteed because we store the InternedStrings in an Arena alongside the
    /// InternedStringArena, and only hand out references to them.
    fn as_str(&self) -> &str {
        unsafe {
            let bytes = std::slice::from_raw_parts(self.start, self.len);
            std::str::from_utf8_unchecked(bytes)
        }
    }

    // Returns a supposedly 'static reference to the string's data.  The string data isn't really
    // static, but we are careful only to use this as a key in the HashMap that StackGraph uses to
    // track whether we've stored a particular symbol already.  That HashMap lives alongside the
    // InternedStringArena that holds the data, so we can get away with a technically incorrect
    // 'static lifetime here.  As an extra precaution, this method is is marked as unsafe so that
    // we don't inadvertently call it from anywhere else in the crate.
    unsafe fn as_hash_key(&self) -> &'static str {
        let bytes = std::slice::from_raw_parts(self.start, self.len);
        std::str::from_utf8_unchecked(bytes)
    }
}

unsafe impl Send for InternedStringContent {}
unsafe impl Sync for InternedStringContent {}

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
#[repr(C)]
pub struct Symbol {
    content: InternedStringContent,
}

impl Symbol {
    pub fn as_str(&self) -> &str {
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
// Interned strings

/// Arbitrary string content associated with some part of a stack graph.
#[repr(C)]
pub struct InternedString {
    content: InternedStringContent,
}

impl InternedString {
    fn as_str(&self) -> &str {
        self.content.as_str()
    }
}

impl PartialEq<&str> for InternedString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl StackGraph {
    /// Adds an interned string to the stack graph, ensuring that there's only ever one copy of a
    /// particular string stored in the graph.
    pub fn add_string<S: AsRef<str> + ?Sized>(&mut self, string: &S) -> Handle<InternedString> {
        let string = string.as_ref();
        if let Some(handle) = self.string_handles.get(string) {
            return *handle;
        }

        let interned = self.interned_strings.add(string);
        let hash_key = unsafe { interned.as_hash_key() };
        let handle = self.strings.add(InternedString { content: interned });
        self.string_handles.insert(hash_key, handle);
        handle
    }

    /// Returns an iterator over all of the handles of all of the interned strings in this stack
    /// graph. (Note that because we're only returning _handles_, this iterator does not retain a
    /// reference to the `StackGraph`.)
    pub fn iter_strings(&self) -> impl Iterator<Item = Handle<InternedString>> {
        self.strings.iter_handles()
    }
}

impl Index<Handle<InternedString>> for StackGraph {
    type Output = str;
    #[inline(always)]
    fn index(&self, handle: Handle<InternedString>) -> &str {
        self.strings.get(handle).as_str()
    }
}

#[doc(hidden)]
pub struct DisplayInternedString<'a> {
    wrapped: Handle<InternedString>,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayInternedString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.graph[self.wrapped])
    }
}

impl Handle<InternedString> {
    pub fn display(self, graph: &StackGraph) -> impl Display + '_ {
        DisplayInternedString {
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
    name: InternedStringContent,
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

    /// Returns the file with a particular name, if it exists.
    pub fn get_file<S: AsRef<str> + ?Sized>(&self, name: &S) -> Option<Handle<File>> {
        let name = name.as_ref();
        self.file_handles.get(name).copied()
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
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeID {
    file: ControlledOption<Handle<File>>,
    local_id: u32,
}

pub(crate) const ROOT_NODE_ID: u32 = 1;
pub(crate) const JUMP_TO_NODE_ID: u32 = 2;

impl NodeID {
    /// Returns the ID of the singleton _root node_.
    #[inline(always)]
    pub fn root() -> NodeID {
        NodeID {
            file: ControlledOption::none(),
            local_id: ROOT_NODE_ID,
        }
    }

    /// Returns the ID of the singleton _jump to scope_ node.
    #[inline(always)]
    pub fn jump_to() -> NodeID {
        NodeID {
            file: ControlledOption::none(),
            local_id: JUMP_TO_NODE_ID,
        }
    }

    /// Creates a new file-local node ID.
    #[inline(always)]
    pub fn new_in_file(file: Handle<File>, local_id: u32) -> NodeID {
        NodeID {
            file: ControlledOption::some(file),
            local_id,
        }
    }

    /// Returns whether this ID refers to the singleton _root node_.
    #[inline(always)]
    pub fn is_root(self) -> bool {
        self.file.is_none() && self.local_id == ROOT_NODE_ID
    }

    /// Returns whether this ID refers to the singleton _jump to scope_ node.
    #[inline(always)]
    pub fn is_jump_to(self) -> bool {
        self.file.is_none() && self.local_id == JUMP_TO_NODE_ID
    }

    /// Returns the file that this node belongs to.  Returns `None` for the singleton _root_ and
    /// _jump to scope_ nodes, which belong to all files.
    #[inline(always)]
    pub fn file(self) -> Option<Handle<File>> {
        self.file.into()
    }

    /// Returns the local ID of this node within its file.  Panics if this node ID refers to the
    /// singleton _root_ or _jump to scope_ nodes.
    #[inline(always)]
    pub fn local_id(self) -> u32 {
        self.local_id
    }

    /// Returns whether this node belongs to a particular file.  Always returns `true` for the
    /// singleton _root_ and _jump to scope_ nodes, which belong to all files.
    #[inline(always)]
    pub fn is_in_file(self, file: Handle<File>) -> bool {
        match self.file.into_option() {
            Some(this_file) => this_file == file,
            _ => true,
        }
    }
}

#[doc(hidden)]
pub struct DisplayNodeID<'a> {
    wrapped: NodeID,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayNodeID<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.wrapped.file.into_option() {
            Some(file) => write!(f, "{}({})", file.display(self.graph), self.wrapped.local_id),
            None => {
                if self.wrapped.is_root() {
                    write!(f, "[root]")
                } else if self.wrapped.is_jump_to() {
                    write!(f, "[jump]")
                } else {
                    unreachable!();
                }
            }
        }
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
#[repr(C)]
pub enum Node {
    DropScopes(DropScopesNode),
    JumpTo(JumpToNode),
    PopScopedSymbol(PopScopedSymbolNode),
    PopSymbol(PopSymbolNode),
    PushScopedSymbol(PushScopedSymbolNode),
    PushSymbol(PushSymbolNode),
    Root(RootNode),
    Scope(ScopeNode),
}

impl Node {
    #[inline(always)]
    pub fn is_exported_scope(&self) -> bool {
        match self {
            Node::Scope(node) => node.is_exported,
            _ => false,
        }
    }

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

    #[inline(always)]
    pub fn is_endpoint(&self) -> bool {
        self.is_definition() || self.is_exported_scope() || self.is_reference() || self.is_root()
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

    /// Returns this node's attached scope, if it has one.  (_Push scoped symbol_ nodes have
    /// attached scopes.)
    pub fn scope(&self) -> Option<NodeID> {
        match self {
            Node::PushScopedSymbol(node) => Some(node.scope),
            _ => None,
        }
    }

    /// Returns the ID of this node.
    pub fn id(&self) -> NodeID {
        match self {
            Node::DropScopes(node) => node.id,
            Node::JumpTo(node) => node.id,
            Node::PushScopedSymbol(node) => node.id,
            Node::PushSymbol(node) => node.id,
            Node::PopScopedSymbol(node) => node.id,
            Node::PopSymbol(node) => node.id,
            Node::Root(node) => node.id,
            Node::Scope(node) => node.id,
        }
    }

    /// Returns the file that this node belongs to.  Returns `None` for the singleton _root_ and
    /// _jump to scope_ nodes, which belong to all files.
    #[inline(always)]
    pub fn file(&self) -> Option<Handle<File>> {
        self.id().file()
    }

    /// Returns whether this node belongs to a particular file.  Always returns `true` for the
    /// singleton _root_ and _jump to scope_ nodes, which belong to all files.
    #[inline(always)]
    pub fn is_in_file(&self, file: Handle<File>) -> bool {
        self.id().is_in_file(file)
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
    pub fn jump_to_node() -> Handle<Node> {
        Handle::new(unsafe { NonZeroU32::new_unchecked(2) })
    }

    /// Returns a handle to the stack graph's singleton _root node_.
    #[inline(always)]
    pub fn root_node() -> Handle<Node> {
        Handle::new(unsafe { NonZeroU32::new_unchecked(1) })
    }

    /// Returns an unused [`NodeID`][] for the given file.
    ///
    /// [`NodeID`]: struct.NodeID.html
    pub fn new_node_id(&mut self, file: Handle<File>) -> NodeID {
        self.node_id_handles.unused_id(file)
    }

    /// Returns an iterator of all of the nodes in the graph.  (Note that because we're only
    /// returning _handles_, this iterator does not retain a reference to the `StackGraph`.)
    pub fn iter_nodes(&self) -> impl Iterator<Item = Handle<Node>> {
        self.nodes.iter_handles()
    }

    /// Returns the handle to the node with a particular ID, if it exists.
    pub fn node_for_id(&self, id: NodeID) -> Option<Handle<Node>> {
        if id.file().is_some() {
            self.node_id_handles.try_handle_for_id(id)
        } else if id.is_root() {
            Some(StackGraph::root_node())
        } else if id.is_jump_to() {
            Some(StackGraph::jump_to_node())
        } else {
            None
        }
    }

    pub(crate) fn add_node(&mut self, id: NodeID, node: Node) -> Option<Handle<Node>> {
        if let Some(_) = self.node_id_handles.handle_for_id(id) {
            return None;
        }
        let handle = self.nodes.add(node);
        self.node_id_handles.set_handle_for_id(id, handle);
        Some(handle)
    }

    pub(crate) fn get_or_create_node(&mut self, id: NodeID, node: Node) -> Handle<Node> {
        if let Some(handle) = self.node_id_handles.handle_for_id(id) {
            return handle;
        }
        let handle = self.nodes.add(node);
        self.node_id_handles.set_handle_for_id(id, handle);
        handle
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
            Node::JumpTo(node) => node.fmt(f),
            Node::PushScopedSymbol(node) => node.display(self.graph).fmt(f),
            Node::PushSymbol(node) => node.display(self.graph).fmt(f),
            Node::PopScopedSymbol(node) => node.display(self.graph).fmt(f),
            Node::PopSymbol(node) => node.display(self.graph).fmt(f),
            Node::Root(node) => node.fmt(f),
            Node::Scope(node) => node.display(self.graph).fmt(f),
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
#[repr(C)]
pub struct DropScopesNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    _symbol: ControlledOption<Handle<Symbol>>,
    _scope: NodeID,
    _is_endpoint: bool,
}

impl From<DropScopesNode> for Node {
    fn from(node: DropScopesNode) -> Node {
        Node::DropScopes(node)
    }
}

impl StackGraph {
    /// Adds a _drop scopes_ node to the stack graph.
    pub fn add_drop_scopes_node(&mut self, id: NodeID) -> Option<Handle<Node>> {
        let node = DropScopesNode {
            id,
            _symbol: ControlledOption::none(),
            _scope: NodeID::default(),
            _is_endpoint: false,
        };
        self.add_node(id, node.into())
    }
}

impl DropScopesNode {
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

/// The singleton "jump to" node, which allows a name binding path to jump back to another part of
/// the graph.
#[repr(C)]
pub struct JumpToNode {
    id: NodeID,
    _symbol: ControlledOption<Handle<Symbol>>,
    _scope: NodeID,
    _is_endpoint: bool,
}

impl From<JumpToNode> for Node {
    fn from(node: JumpToNode) -> Node {
        Node::JumpTo(node)
    }
}

impl JumpToNode {
    fn new() -> JumpToNode {
        JumpToNode {
            id: NodeID::jump_to(),
            _symbol: ControlledOption::none(),
            _scope: NodeID::default(),
            _is_endpoint: false,
        }
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
#[repr(C)]
pub struct PopScopedSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to pop off the symbol stack.
    pub symbol: Handle<Symbol>,
    _scope: NodeID,
    /// Whether this node represents a reference in the source language.
    pub is_definition: bool,
}

impl From<PopScopedSymbolNode> for Node {
    fn from(node: PopScopedSymbolNode) -> Node {
        Node::PopScopedSymbol(node)
    }
}

impl StackGraph {
    /// Adds a _pop scoped symbol_ node to the stack graph.
    pub fn add_pop_scoped_symbol_node(
        &mut self,
        id: NodeID,
        symbol: Handle<Symbol>,
        is_definition: bool,
    ) -> Option<Handle<Node>> {
        let node = PopScopedSymbolNode {
            id,
            symbol,
            _scope: NodeID::default(),
            is_definition,
        };
        self.add_node(id, node.into())
    }
}

impl PopScopedSymbolNode {
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
#[repr(C)]
pub struct PopSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to pop off the symbol stack.
    pub symbol: Handle<Symbol>,
    _scope: NodeID,
    /// Whether this node represents a reference in the source language.
    pub is_definition: bool,
}

impl From<PopSymbolNode> for Node {
    fn from(node: PopSymbolNode) -> Node {
        Node::PopSymbol(node)
    }
}

impl StackGraph {
    /// Adds a _pop symbol_ node to the stack graph.
    pub fn add_pop_symbol_node(
        &mut self,
        id: NodeID,
        symbol: Handle<Symbol>,
        is_definition: bool,
    ) -> Option<Handle<Node>> {
        let node = PopSymbolNode {
            id,
            symbol,
            _scope: NodeID::default(),
            is_definition,
        };
        self.add_node(id, node.into())
    }
}

impl PopSymbolNode {
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
#[repr(C)]
pub struct PushScopedSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to push onto the symbol stack.
    pub symbol: Handle<Symbol>,
    /// The exported scope node that should be attached to the scoped symbol.  The node ID must
    /// refer to an exported scope node.
    pub scope: NodeID,
    /// Whether this node represents a reference in the source language.
    pub is_reference: bool,
    _phantom: (),
}

impl From<PushScopedSymbolNode> for Node {
    fn from(node: PushScopedSymbolNode) -> Node {
        Node::PushScopedSymbol(node)
    }
}

impl StackGraph {
    /// Adds a _push scoped symbol_ node to the stack graph.
    pub fn add_push_scoped_symbol_node(
        &mut self,
        id: NodeID,
        symbol: Handle<Symbol>,
        scope: NodeID,
        is_reference: bool,
    ) -> Option<Handle<Node>> {
        let node = PushScopedSymbolNode {
            id,
            symbol,
            scope,
            is_reference,
            _phantom: (),
        };
        self.add_node(id, node.into())
    }
}

impl PushScopedSymbolNode {
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
#[repr(C)]
pub struct PushSymbolNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    /// The symbol to push onto the symbol stack.
    pub symbol: Handle<Symbol>,
    _scope: NodeID,
    /// Whether this node represents a reference in the source language.
    pub is_reference: bool,
}

impl From<PushSymbolNode> for Node {
    fn from(node: PushSymbolNode) -> Node {
        Node::PushSymbol(node)
    }
}

impl StackGraph {
    /// Adds a _push symbol_ node to the stack graph.
    pub fn add_push_symbol_node(
        &mut self,
        id: NodeID,
        symbol: Handle<Symbol>,
        is_reference: bool,
    ) -> Option<Handle<Node>> {
        let node = PushSymbolNode {
            id,
            symbol,
            _scope: NodeID::default(),
            is_reference,
        };
        self.add_node(id, node.into())
    }
}

impl PushSymbolNode {
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
#[repr(C)]
pub struct RootNode {
    id: NodeID,
    _symbol: ControlledOption<Handle<Symbol>>,
    _scope: NodeID,
    _is_endpoint: bool,
}

impl From<RootNode> for Node {
    fn from(node: RootNode) -> Node {
        Node::Root(node)
    }
}

impl RootNode {
    fn new() -> RootNode {
        RootNode {
            id: NodeID::root(),
            _symbol: ControlledOption::none(),
            _scope: NodeID::default(),
            _is_endpoint: false,
        }
    }
}

impl Display for RootNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[root]")
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
        let file_entry = self.files.get(node_id.file().unwrap())?;
        let node_index = node_id.local_id as usize;
        if node_index >= file_entry.len() {
            return None;
        }
        file_entry[node_index]
    }

    fn handle_for_id(&mut self, node_id: NodeID) -> Option<Handle<Node>> {
        let file_entry = &mut self.files[node_id.file().unwrap()];
        let node_index = node_id.local_id as usize;
        if node_index >= file_entry.len() {
            file_entry.resize(node_index + 1, None);
        }
        file_entry[node_index]
    }

    fn set_handle_for_id(&mut self, node_id: NodeID, handle: Handle<Node>) {
        let file_entry = &mut self.files[node_id.file().unwrap()];
        let node_index = node_id.local_id as usize;
        file_entry[node_index] = Some(handle);
    }

    fn unused_id(&mut self, file: Handle<File>) -> NodeID {
        let local_id = self
            .files
            .get(file)
            .map(|file_entry| file_entry.len() as u32)
            .unwrap_or(0);
        NodeID::new_in_file(file, local_id)
    }

    fn nodes_for_file(&self, file: Handle<File>) -> impl Iterator<Item = Handle<Node>> + '_ {
        let file_entry = match self.files.get(file) {
            Some(file_entry) => file_entry,
            None => return Either::Left(std::iter::empty()),
        };
        Either::Right(file_entry.iter().filter_map(|entry| *entry))
    }
}

/// A node that adds structure to the graph. If the node is exported, it can be
/// referred to on the scope stack, which allows "jump to" nodes in any other
/// part of the graph can jump back here.
#[repr(C)]
pub struct ScopeNode {
    /// The unique identifier for this node.
    pub id: NodeID,
    _symbol: ControlledOption<Handle<Symbol>>,
    _scope: NodeID,
    pub is_exported: bool,
}

impl From<ScopeNode> for Node {
    fn from(node: ScopeNode) -> Node {
        Node::Scope(node)
    }
}

impl StackGraph {
    /// Adds a _scope_ node to the stack graph.
    pub fn add_scope_node(&mut self, id: NodeID, is_exported: bool) -> Option<Handle<Node>> {
        let node = ScopeNode {
            id,
            _symbol: ControlledOption::none(),
            _scope: NodeID::default(),
            is_exported,
        };
        self.add_node(id, node.into())
    }
}

impl ScopeNode {
    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl Display + 'a {
        DisplayScopeNode {
            wrapped: self,
            graph,
        }
    }
}

#[doc(hidden)]
pub struct DisplayScopeNode<'a> {
    wrapped: &'a ScopeNode,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplayScopeNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "[{}]", self.wrapped.id.display(self.graph))
        } else {
            write!(
                f,
                "[{}{} scope]",
                self.wrapped.id.display(self.graph),
                if self.wrapped.is_exported {
                    " exported"
                } else {
                    ""
                },
            )
        }
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
    pub precedence: i32,
}

pub(crate) struct OutgoingEdge {
    sink: Handle<Node>,
    precedence: i32,
}

impl StackGraph {
    /// Adds a new edge to the stack graph.
    pub fn add_edge(&mut self, source: Handle<Node>, sink: Handle<Node>, precedence: i32) {
        let edges = &mut self.outgoing_edges[source];
        if let Err(index) = edges.binary_search_by_key(&sink, |o| o.sink) {
            edges.insert(index, OutgoingEdge { sink, precedence });
            self.incoming_edges[sink] += Degree::One;
        }
    }

    /// Sets edge precedence of the given edge.
    pub fn set_edge_precedence(
        &mut self,
        source: Handle<Node>,
        sink: Handle<Node>,
        precedence: i32,
    ) {
        let edges = &mut self.outgoing_edges[source];
        if let Ok(index) = edges.binary_search_by_key(&sink, |o| o.sink) {
            edges[index].precedence = precedence;
        }
    }

    /// Returns an iterator of all of the edges that begin at a particular source node.
    pub fn outgoing_edges(&self, source: Handle<Node>) -> impl Iterator<Item = Edge> + '_ {
        match self.outgoing_edges.get(source) {
            Some(edges) => Either::Right(edges.iter().map(move |o| Edge {
                source,
                sink: o.sink,
                precedence: o.precedence,
            })),
            None => Either::Left(std::iter::empty()),
        }
    }

    /// Returns the number of edges that end at a particular sink node.
    pub fn incoming_edge_degree(&self, sink: Handle<Node>) -> Degree {
        self.incoming_edges
            .get(sink)
            .cloned()
            .unwrap_or(Degree::Zero)
    }
}

//-------------------------------------------------------------------------------------------------
// Source code

/// Contains information about a range of code in a source code file.
#[repr(C)]
#[derive(Default)]
pub struct SourceInfo {
    /// The location in its containing file of the source code that this node represents.
    pub span: lsp_positions::Span,
    /// The kind of syntax entity this node represents (e.g. `function`, `class`, `method`, etc.).
    pub syntax_type: ControlledOption<Handle<InternedString>>,
    /// The full content of the line containing this node in its source file.
    pub containing_line: ControlledOption<Handle<InternedString>>,
    /// The location in its containing file of the source code that this node's definiens represents.
    /// This is used for things like the bodies of functions, rather than the RHSes of equations.
    /// If you need one of these to make the type checker happy, but you don't have one, just use
    /// lsp_positions::Span::default(), as this will correspond to the all-0s spans which mean "no definiens".
    pub definiens_span: lsp_positions::Span,
    /// The fully qualified name is a representation of the symbol that captures its name and its
    /// embedded context (e.g. `foo.bar` for the symbol `bar` defined in the module `foo`).
    pub fully_qualified_name: ControlledOption<Handle<InternedString>>,
}

impl StackGraph {
    /// Returns information about the source code that a stack graph node represents.
    pub fn source_info(&self, node: Handle<Node>) -> Option<&SourceInfo> {
        self.source_info.get(node)
    }

    /// Returns a mutable reference to the information about the source code that a stack graph
    /// node represents.
    pub fn source_info_mut(&mut self, node: Handle<Node>) -> &mut SourceInfo {
        &mut self.source_info[node]
    }
}

//-------------------------------------------------------------------------------------------------
// Debug info

/// Contains debug info about a stack graph node as key-value pairs of strings.
#[derive(Default)]
pub struct DebugInfo {
    entries: Vec<DebugEntry>,
}

impl DebugInfo {
    pub fn add(&mut self, key: Handle<InternedString>, value: Handle<InternedString>) {
        self.entries.push(DebugEntry { key, value });
    }

    pub fn iter(&self) -> std::slice::Iter<DebugEntry> {
        self.entries.iter()
    }
}

/// A debug entry consisting of a string key-value air of strings.
pub struct DebugEntry {
    pub key: Handle<InternedString>,
    pub value: Handle<InternedString>,
}

impl StackGraph {
    /// Returns debug information about the stack graph node.
    pub fn node_debug_info(&self, node: Handle<Node>) -> Option<&DebugInfo> {
        self.node_debug_info.get(node)
    }

    /// Returns a mutable reference to the debug info about the stack graph node.
    pub fn node_debug_info_mut(&mut self, node: Handle<Node>) -> &mut DebugInfo {
        &mut self.node_debug_info[node]
    }

    /// Returns debug information about the stack graph edge.
    pub fn edge_debug_info(&self, source: Handle<Node>, sink: Handle<Node>) -> Option<&DebugInfo> {
        self.edge_debug_info.get(source).and_then(|es| {
            match es.binary_search_by_key(&sink, |e| e.0) {
                Ok(idx) => Some(&es[idx].1),
                Err(_) => None,
            }
        })
    }

    /// Returns a mutable reference to the debug info about the stack graph edge.
    pub fn edge_debug_info_mut(
        &mut self,
        source: Handle<Node>,
        sink: Handle<Node>,
    ) -> &mut DebugInfo {
        let es = &mut self.edge_debug_info[source];
        let idx = match es.binary_search_by_key(&sink, |e| e.0) {
            Ok(idx) => idx,
            Err(idx) => {
                es.insert(idx, (sink, DebugInfo::default()));
                idx
            }
        };
        &mut es[idx].1
    }
}

//-------------------------------------------------------------------------------------------------
// Stack graphs

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Degree {
    Zero,
    One,
    Multiple,
}

impl Default for Degree {
    fn default() -> Self {
        Self::Zero
    }
}

impl std::ops::Add for Degree {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Zero, result) | (result, Self::Zero) => result,
            _ => Self::Multiple,
        }
    }
}

impl std::ops::AddAssign for Degree {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// Contains all of the nodes and edges that make up a stack graph.
pub struct StackGraph {
    interned_strings: InternedStringArena,
    pub(crate) symbols: Arena<Symbol>,
    symbol_handles: FxHashMap<&'static str, Handle<Symbol>>,
    pub(crate) strings: Arena<InternedString>,
    string_handles: FxHashMap<&'static str, Handle<InternedString>>,
    pub(crate) files: Arena<File>,
    file_handles: FxHashMap<&'static str, Handle<File>>,
    pub(crate) nodes: Arena<Node>,
    pub(crate) source_info: SupplementalArena<Node, SourceInfo>,
    node_id_handles: NodeIDHandles,
    outgoing_edges: SupplementalArena<Node, SmallVec<[OutgoingEdge; 4]>>,
    incoming_edges: SupplementalArena<Node, Degree>,
    pub(crate) node_debug_info: SupplementalArena<Node, DebugInfo>,
    pub(crate) edge_debug_info: SupplementalArena<Node, SmallVec<[(Handle<Node>, DebugInfo); 4]>>,
}

impl StackGraph {
    /// Creates a new, initially empty stack graph.
    pub fn new() -> StackGraph {
        StackGraph::default()
    }

    /// Copies the given stack graph into this stack graph. Panics if any of the files
    /// in the other stack graph are already defined in the current one.
    pub fn add_from_graph(
        &mut self,
        other: &StackGraph,
    ) -> Result<Vec<Handle<File>>, Handle<File>> {
        let mut files = HashMap::new();
        for other_file in other.iter_files() {
            let file = self.add_file(other[other_file].name())?;
            files.insert(other_file, file);
        }
        let files = files;
        let node_id = |other_node_id: NodeID| {
            if other_node_id.is_root() {
                NodeID::root()
            } else if other_node_id.is_jump_to() {
                NodeID::jump_to()
            } else {
                NodeID::new_in_file(
                    files[&other_node_id.file.into_option().unwrap()],
                    other_node_id.local_id,
                )
            }
        };
        let mut nodes = HashMap::new();
        nodes.insert(Self::root_node(), Self::root_node());
        nodes.insert(Self::jump_to_node(), Self::jump_to_node());
        for other_file in files.keys().cloned() {
            let file = files[&other_file];
            for other_node in other.nodes_for_file(other_file) {
                let value: Node = match other[other_node] {
                    Node::DropScopes(DropScopesNode { id, .. }) => DropScopesNode {
                        id: NodeID::new_in_file(file, id.local_id),
                        _symbol: ControlledOption::default(),
                        _scope: NodeID::default(),
                        _is_endpoint: bool::default(),
                    }
                    .into(),
                    Node::JumpTo(JumpToNode { .. }) => JumpToNode {
                        id: NodeID::jump_to(),
                        _symbol: ControlledOption::default(),
                        _scope: NodeID::default(),
                        _is_endpoint: bool::default(),
                    }
                    .into(),
                    Node::PopScopedSymbol(PopScopedSymbolNode {
                        id,
                        symbol,
                        is_definition,
                        ..
                    }) => PopScopedSymbolNode {
                        id: NodeID::new_in_file(file, id.local_id),
                        symbol: self.add_symbol(&other[symbol]),
                        _scope: NodeID::default(),
                        is_definition: is_definition,
                    }
                    .into(),
                    Node::PopSymbol(PopSymbolNode {
                        id,
                        symbol,
                        is_definition,
                        ..
                    }) => PopSymbolNode {
                        id: NodeID::new_in_file(file, id.local_id),
                        symbol: self.add_symbol(&other[symbol]),
                        _scope: NodeID::default(),
                        is_definition: is_definition,
                    }
                    .into(),
                    Node::PushScopedSymbol(PushScopedSymbolNode {
                        id,
                        symbol,
                        scope,
                        is_reference,
                        ..
                    }) => PushScopedSymbolNode {
                        id: NodeID::new_in_file(file, id.local_id),
                        symbol: self.add_symbol(&other[symbol]),
                        scope: node_id(scope),
                        is_reference: is_reference,
                        _phantom: (),
                    }
                    .into(),
                    Node::PushSymbol(PushSymbolNode {
                        id,
                        symbol,
                        is_reference,
                        ..
                    }) => PushSymbolNode {
                        id: NodeID::new_in_file(file, id.local_id),
                        symbol: self.add_symbol(&other[symbol]),
                        _scope: NodeID::default(),
                        is_reference: is_reference,
                    }
                    .into(),
                    Node::Root(RootNode { .. }) => RootNode {
                        id: NodeID::root(),
                        _symbol: ControlledOption::default(),
                        _scope: NodeID::default(),
                        _is_endpoint: bool::default(),
                    }
                    .into(),
                    Node::Scope(ScopeNode {
                        id, is_exported, ..
                    }) => ScopeNode {
                        id: NodeID::new_in_file(file, id.local_id),
                        _symbol: ControlledOption::default(),
                        _scope: NodeID::default(),
                        is_exported: is_exported,
                    }
                    .into(),
                };
                let node = self.add_node(value.id(), value).unwrap();
                nodes.insert(other_node, node);
                if let Some(source_info) = other.source_info(other_node) {
                    *self.source_info_mut(node) = SourceInfo {
                        span: source_info.span.clone(),
                        syntax_type: source_info
                            .syntax_type
                            .into_option()
                            .map(|st| self.add_string(&other[st]))
                            .into(),
                        containing_line: source_info
                            .containing_line
                            .into_option()
                            .map(|cl| self.add_string(&other[cl]))
                            .into(),
                        definiens_span: source_info.definiens_span.clone(),
                        fully_qualified_name: ControlledOption::default(),
                    };
                }
                if let Some(debug_info) = other.node_debug_info(other_node) {
                    *self.node_debug_info_mut(node) = DebugInfo {
                        entries: debug_info
                            .entries
                            .iter()
                            .map(|e| DebugEntry {
                                key: self.add_string(&other[e.key]),
                                value: self.add_string(&other[e.value]),
                            })
                            .collect::<Vec<_>>(),
                    };
                }
            }
            for other_node in nodes.keys().cloned() {
                for other_edge in other.outgoing_edges(other_node) {
                    self.add_edge(
                        nodes[&other_edge.source],
                        nodes[&other_edge.sink],
                        other_edge.precedence,
                    );
                }
            }
        }
        Ok(files.into_values().collect())
    }
}

impl Default for StackGraph {
    fn default() -> StackGraph {
        let mut nodes = Arena::new();
        nodes.add(RootNode::new().into());
        nodes.add(JumpToNode::new().into());

        StackGraph {
            interned_strings: InternedStringArena::new(),
            symbols: Arena::new(),
            symbol_handles: FxHashMap::default(),
            strings: Arena::new(),
            string_handles: FxHashMap::default(),
            files: Arena::new(),
            file_handles: FxHashMap::default(),
            nodes,
            source_info: SupplementalArena::new(),
            node_id_handles: NodeIDHandles::new(),
            outgoing_edges: SupplementalArena::new(),
            incoming_edges: SupplementalArena::new(),
            node_debug_info: SupplementalArena::new(),
            edge_debug_info: SupplementalArena::new(),
        }
    }
}
