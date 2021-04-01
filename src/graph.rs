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
use std::ops::Deref;
use std::ops::Index;

use either::Either;
use fxhash::FxHashMap;
use smallvec::SmallVec;

use crate::arena::Arena;
use crate::arena::Handle;
use crate::arena::SupplementalArena;

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
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Symbol {
    symbol: String,
}

impl Symbol {
    pub fn as_str(&self) -> &str {
        &self.symbol
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.symbol
    }
}

impl Deref for Symbol {
    type Target = str;
    fn deref(&self) -> &str {
        &self.symbol
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl PartialEq<&str> for Symbol {
    fn eq(&self, other: &&str) -> bool {
        self.symbol == **other
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
        let symbol_value = symbol.to_string();
        let symbol = Symbol {
            symbol: symbol_value.clone(),
        };
        let handle = self.symbols.add(symbol);
        self.symbol_handles.insert(symbol_value, handle);
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
    type Output = Symbol;
    #[inline(always)]
    fn index(&self, handle: Handle<Symbol>) -> &Symbol {
        &self.symbols.get(handle)
    }
}

#[doc(hidden)]
pub struct DisplaySymbol<'a> {
    wrapped: Handle<Symbol>,
    graph: &'a StackGraph,
}

impl<'a> Display for DisplaySymbol<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.graph[self.wrapped])
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
    pub name: String,
}

impl StackGraph {
    /// Adds a file to the stack graph, ensuring that there's only ever one file with a particular
    /// name in the graph.
    pub fn add_file<S: AsRef<str> + ?Sized>(&mut self, name: &S) -> Handle<File> {
        let name = name.as_ref();
        if let Some(handle) = self.file_handles.get(name) {
            return *handle;
        }
        let name_value = name.to_string();
        let file = File {
            name: name_value.clone(),
        };
        let handle = self.files.add(file);
        self.file_handles.insert(name_value, handle);
        handle
    }
}

impl Index<Handle<File>> for StackGraph {
    type Output = File;
    #[inline(always)]
    fn index(&self, handle: Handle<File>) -> &File {
        &self.files.get(handle)
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

    fn add_node(&mut self, node_id: NodeID, node: Node) -> Option<Handle<Node>> {
        if let Some(_) = self.node_id_handles.handle_for_id(node_id) {
            return None;
        }
        let handle = self.nodes.add(node);
        self.node_id_handles.set_handle_for_id(node_id, handle);
        Some(handle)
    }
}

impl Index<Handle<Node>> for StackGraph {
    type Output = Node;
    #[inline(always)]
    fn index(&self, handle: Handle<Node>) -> &Node {
        &self.nodes.get(handle)
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
}

/// The singleton "jump to" node, which allows a name binding path to jump back to another part of
/// the graph.
pub struct JumpToNode;

impl From<JumpToNode> for Node {
    fn from(node: JumpToNode) -> Node {
        Node::JumpTo(node)
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
}

/// The singleton root node, which allows a name binding path to cross between files.
pub struct RootNode;

impl From<RootNode> for Node {
    fn from(node: RootNode) -> Node {
        Node::Root(node)
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
}

//-------------------------------------------------------------------------------------------------
// Edges

/// Connects two nodes in a stack graph.
///
/// These edges provide the basic graph connectivity that allow us to search for name binding paths
/// in a stack graph.  (Though not all sequence of edges is a well-formed name binding: the nodes
/// that you encounter along the path must also satisfy all of the rules for maintaining correct
/// symbol and scope stacks.)
#[derive(Clone, Copy, Debug)]
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
    // TODO: We're currently storing the content of each symbol twice.  Find a way to only store
    // the content once, most likely using the trick described at
    // https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html
    symbols: Arena<Symbol>,
    symbol_handles: FxHashMap<String, Handle<Symbol>>,
    files: Arena<File>,
    file_handles: FxHashMap<String, Handle<File>>,
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
