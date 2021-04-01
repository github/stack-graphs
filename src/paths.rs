// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Paths represent name bindings in a source language.
//!
//! With the set of rules we have for constructing stack graphs, bindings between references and
//! definitions are represented by paths within the graph.  Each edge in the path must leave the
//! symbol and scopes stacks in a valid state — otherwise we have violated some name binding rule
//! in the source language.  The symbol and scope stacks must be empty at the beginning and end of
//! the path.  The reference's _push symbol_ node "seeds" the symbol stack with the first thing
//! that we want to look for, and once we (hopefully) reach the definition that reference refers
//! to, its pop node will remove that symbol from the symbol stack, leaving both stacks empty.

use std::collections::VecDeque;
use std::fmt::Display;

use crate::arena::Handle;
use crate::arena::List;
use crate::arena::ListArena;
use crate::graph::Edge;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::graph::Symbol;

//-------------------------------------------------------------------------------------------------
// Displaying stuff

/// This trait only exists because:
///
///   - we need `Display` implementations that dereference arena handles from our `StackGraph` and
///     `Paths` bags o' crap,
///   - many of our arena-managed types can handles to _other_ arena-managed data, which we need to
///     recursively display as part of displaying the "outer" instance, and
///   - in particular, we sometimes need `&mut` access to the `Paths` arenas.
///
/// The borrow checker is not very happy with us having all of these constraints at the same time —
/// in particular, the last one.
///
/// This trait gets around the problem by breaking up the display operation into two steps:
///
///   - First, each data instance has a chance to "prepare" itself with `&mut` access to whatever
///     arenas it needs.  (Anything containing a `Deque`, for instance, uses this step to ensure
///     that our copy of the deque is pointed in the right direction, since reversing requires
///     `&mut` access to the arena.)
///
///   - Once everything has been prepared, we return a value that implements `Display`, and
///     contains _non-mutable_ references to the arena.  Because our arena references are
///     non-mutable, we don't run into any problems with the borrow checker while recursively
///     displaying the contents of the data instance.
trait DisplayWithPaths {
    fn prepare(&mut self, _graph: &StackGraph, _paths: &mut Paths) {}

    fn display_with(
        &self,
        graph: &StackGraph,
        paths: &Paths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result;
}

/// Prepares and returns a `Display` implementation for a type `D` that implements
/// `DisplayWithPaths`.  We only require `&mut` access to the `PartialPath` arenas while
/// creating the `Display` instance; the `Display` instance itself will only retain shared access
/// to the arenas.
fn display_with<'a, D>(
    mut value: D,
    graph: &'a StackGraph,
    paths: &'a mut Paths,
) -> impl Display + 'a
where
    D: DisplayWithPaths + 'a,
{
    value.prepare(graph, paths);
    DisplayWithPathsWrapper {
        value,
        graph,
        paths,
    }
}

/// Returns a `Display` implementation that you can use inside of your `display_with` method to
/// display any recursive fields.  This assumes that the recursive fields have already been
/// prepared.
fn display_prepared<'a, D>(value: D, graph: &'a StackGraph, paths: &'a Paths) -> impl Display + 'a
where
    D: DisplayWithPaths + 'a,
{
    DisplayWithPathsWrapper {
        value,
        graph,
        paths,
    }
}

#[doc(hidden)]
struct DisplayWithPathsWrapper<'a, D> {
    value: D,
    graph: &'a StackGraph,
    paths: &'a Paths,
}

impl<'a, D> Display for DisplayWithPathsWrapper<'a, D>
where
    D: DisplayWithPaths,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.value.display_with(self.graph, self.paths, f)
    }
}

//-------------------------------------------------------------------------------------------------
// Symbol stacks

/// A symbol with a possibly empty list of exported scopes attached to it.
#[derive(Clone, Copy)]
pub struct ScopedSymbol {
    pub symbol: Handle<Symbol>,
    pub scopes: Option<ScopeStack>,
}

impl ScopedSymbol {
    pub fn display<'a>(self, graph: &'a StackGraph, paths: &'a mut Paths) -> impl Display + 'a {
        display_with(self, graph, paths)
    }
}

impl DisplayWithPaths for ScopedSymbol {
    fn prepare(&mut self, graph: &StackGraph, paths: &mut Paths) {
        if let Some(scopes) = &mut self.scopes {
            scopes.prepare(graph, paths);
        }
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        paths: &Paths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match self.scopes {
            Some(scopes) => write!(
                f,
                "{}/{}",
                self.symbol.display(graph),
                display_prepared(scopes, graph, paths),
            ),
            None => write!(f, "{}", self.symbol.display(graph)),
        }
    }
}

/// A sequence of symbols that describe what we are currently looking for while in the middle of
/// the path-finding algorithm.
#[derive(Clone, Copy)]
pub struct SymbolStack {
    list: List<ScopedSymbol>,
}

impl SymbolStack {
    /// Returns whether this symbol stack is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns an empty symbol stack.
    pub fn empty() -> SymbolStack {
        SymbolStack {
            list: List::empty(),
        }
    }

    /// Pushes a new [`ScopedSymbol`][] onto the front of this symbol stack.
    ///
    /// [`ScopedSymbol`]: struct.ScopedSymbol.html
    pub fn push_front(&mut self, paths: &mut Paths, scoped_symbol: ScopedSymbol) {
        self.list
            .push_front(&mut paths.symbol_stacks, scoped_symbol);
    }

    /// Removes and returns the [`ScopedSymbol`][] at the front of this symbol stack.  If the stack
    /// is empty, returns `None`.
    pub fn pop_front(&mut self, paths: &Paths) -> Option<ScopedSymbol> {
        self.list.pop_front(&paths.symbol_stacks).copied()
    }

    pub fn display<'a>(self, graph: &'a StackGraph, paths: &'a mut Paths) -> impl Display + 'a {
        display_with(self, graph, paths)
    }

    pub fn iter<'a>(&'a self, paths: &'a Paths) -> impl Iterator<Item = ScopedSymbol> + 'a {
        self.list.iter(&paths.symbol_stacks).copied()
    }
}

impl DisplayWithPaths for SymbolStack {
    fn prepare(&mut self, graph: &StackGraph, paths: &mut Paths) {
        let stack = self;
        while let Some(mut symbol) = stack.pop_front(paths) {
            symbol.prepare(graph, paths);
        }
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        paths: &Paths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        for symbol in self.iter(paths) {
            symbol.display_with(graph, paths, f)?;
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Scope stacks

/// A sequence of exported scopes, used to pass name-binding context around a stack graph.
#[derive(Clone, Copy)]
pub struct ScopeStack {
    list: List<Handle<Node>>,
}

impl ScopeStack {
    /// Returns whether this scope stack is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns an empty scope stack.
    pub fn empty() -> ScopeStack {
        ScopeStack {
            list: List::empty(),
        }
    }

    /// Pushes a new [`Node`][] onto the front of this scope stack.  The node must be an _exported
    /// scope node_.
    ///
    /// [`Node`]: ../graph/enum.Node.html
    pub fn push_front(&mut self, paths: &mut Paths, node: Handle<Node>) {
        self.list.push_front(&mut paths.scope_stacks, node);
    }

    /// Removes and returns the [`Node`][] at the front of this scope stack.  If the stack is
    /// empty, returns `None`.
    pub fn pop_front(&mut self, paths: &Paths) -> Option<Handle<Node>> {
        self.list.pop_front(&paths.scope_stacks).copied()
    }

    pub fn display<'a>(self, graph: &'a StackGraph, paths: &'a mut Paths) -> impl Display + 'a {
        display_with(self, graph, paths)
    }

    pub fn iter<'a>(&'a self, paths: &'a Paths) -> impl Iterator<Item = Handle<Node>> + 'a {
        self.list.iter(&paths.scope_stacks).copied()
    }
}

impl DisplayWithPaths for ScopeStack {
    fn display_with(
        &self,
        graph: &StackGraph,
        paths: &Paths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        for scope in self.iter(paths) {
            write!(f, "{:#}", scope.display(graph))?;
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Paths

/// A sequence of edges from a stack graph.  A _complete_ path represents a full name binding in a
/// source language.
#[derive(Clone)]
pub struct Path {
    pub start_node: Handle<Node>,
    pub end_node: Handle<Node>,
    pub symbol_stack: SymbolStack,
    pub scope_stack: ScopeStack,
    pub edge_count: usize,
}

impl Path {
    /// Creates a new empty path starting at a stack graph node.  The starting node must be a _push
    /// symbol_ node, and will typically be a _reference_ node in particular.
    pub fn from_node(graph: &StackGraph, paths: &mut Paths, node: Handle<Node>) -> Option<Path> {
        let mut scope_stack = ScopeStack::empty();
        let scoped_symbol = match &graph[node] {
            Node::PushScopedSymbol(node) => {
                scope_stack.push_front(paths, node.scope);
                ScopedSymbol {
                    symbol: node.symbol,
                    scopes: Some(scope_stack),
                }
            }
            Node::PushSymbol(node) => ScopedSymbol {
                symbol: node.symbol,
                scopes: None,
            },
            _ => return None,
        };
        let mut symbol_stack = SymbolStack::empty();
        symbol_stack.push_front(paths, scoped_symbol);
        Some(Path {
            start_node: node,
            end_node: node,
            symbol_stack,
            scope_stack,
            edge_count: 0,
        })
    }

    /// A _complete_ path represents a full name binding that resolves a reference to a definition.
    pub fn is_complete(&self, graph: &StackGraph) -> bool {
        if !graph[self.start_node].is_reference() {
            return false;
        } else if !graph[self.end_node].is_definition() {
            return false;
        } else if !self.symbol_stack.is_empty() {
            return false;
        } else if !self.scope_stack.is_empty() {
            return false;
        } else {
            true
        }
    }

    pub fn display<'a>(&'a self, graph: &'a StackGraph, paths: &'a mut Paths) -> impl Display + 'a {
        display_with(self, graph, paths)
    }
}

impl<'a> DisplayWithPaths for &'a Path {
    fn prepare(&mut self, graph: &StackGraph, paths: &mut Paths) {
        self.symbol_stack.clone().prepare(graph, paths);
        self.scope_stack.clone().prepare(graph, paths);
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        paths: &Paths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(
            f,
            "{} -> {}",
            self.start_node.display(graph),
            self.end_node.display(graph),
        )?;
        if !self.symbol_stack.is_empty() || !self.scope_stack.is_empty() {
            write!(
                f,
                " <{}> ({})",
                display_prepared(self.symbol_stack, graph, paths),
                display_prepared(self.scope_stack, graph, paths),
            )?;
        }
        Ok(())
    }
}

/// Errors that can occur during the path resolution process.
#[derive(Debug)]
pub enum PathResolutionError {
    /// The path contains a _jump to scope_ node, but there are no scopes on the scope stack to
    /// jump to.
    EmptyScopeStack,
    /// The path contains a _pop symbol_ or _pop scoped symbol_ node, but there are no symbols on
    /// the symbol stack to pop off.
    EmptySymbolStack,
    /// The path contains a _pop symbol_ or _pop scoped symbol_ node, but the symbol at the top of
    /// the symbol stack does not match.
    IncorrectPoppedSymbol,
    /// The path contains an edge whose source node does not match the sink node of the preceding
    /// edge.
    IncorrectSourceNode,
    /// The path contains a _pop scoped symbol_ node, but the symbol at the top of the symbol stack
    /// does not have an attached scope list to pop off.
    MissingAttachedScopeList,
    /// The path contains a _pop symbol_ node, but the symbol at the top of the symbol stack has an
    /// attached scope list that we weren't expecting.
    UnexpectedAttachedScopeList,
}

impl Path {
    /// Attempts to append an edge to the end of a path.  If the edge is not a valid extension of
    /// this path, we return an error describing why.
    pub fn append(
        &mut self,
        graph: &StackGraph,
        paths: &mut Paths,
        edge: Edge,
    ) -> Result<(), PathResolutionError> {
        if edge.source != self.end_node {
            return Err(PathResolutionError::IncorrectSourceNode);
        }

        let sink = &graph[edge.sink];
        if let Node::PushSymbol(sink) = sink {
            let sink_symbol = sink.symbol;
            let scoped_symbol = ScopedSymbol {
                symbol: sink_symbol,
                scopes: None,
            };
            self.symbol_stack.push_front(paths, scoped_symbol);
        } else if let Node::PushScopedSymbol(sink) = sink {
            let sink_symbol = sink.symbol;
            let sink_scope = sink.scope;
            let mut attached_scopes = self.scope_stack;
            attached_scopes.push_front(paths, sink_scope);
            let scoped_symbol = ScopedSymbol {
                symbol: sink_symbol,
                scopes: Some(attached_scopes),
            };
            self.symbol_stack.push_front(paths, scoped_symbol);
        } else if let Node::PopSymbol(sink) = sink {
            let top = match self.symbol_stack.pop_front(paths) {
                Some(top) => top,
                None => return Err(PathResolutionError::EmptySymbolStack),
            };
            if top.symbol != sink.symbol {
                return Err(PathResolutionError::IncorrectPoppedSymbol);
            }
            if top.scopes.is_some() {
                return Err(PathResolutionError::UnexpectedAttachedScopeList);
            }
        } else if let Node::PopScopedSymbol(sink) = sink {
            let top = match self.symbol_stack.pop_front(paths) {
                Some(top) => top,
                None => return Err(PathResolutionError::EmptySymbolStack),
            };
            if top.symbol != sink.symbol {
                return Err(PathResolutionError::IncorrectPoppedSymbol);
            }
            let new_scope_stack = match top.scopes {
                Some(scopes) => scopes,
                None => return Err(PathResolutionError::MissingAttachedScopeList),
            };
            self.scope_stack = new_scope_stack;
        } else if let Node::DropScopes(_) = sink {
            self.scope_stack = ScopeStack::empty();
        }

        self.end_node = edge.sink;
        self.edge_count += 1;
        Ok(())
    }

    /// Attempts to resolve any _jump to scope_ node at the end of a path.  If the path does not
    /// end in a _jump to scope_ node, we do nothing.  If it does, and we cannot resolve it, then
    /// we return an error describing why.
    pub fn resolve(
        &mut self,
        graph: &StackGraph,
        paths: &Paths,
    ) -> Result<(), PathResolutionError> {
        if !graph[self.end_node].is_jump_to() {
            return Ok(());
        }
        let top_scope = match self.scope_stack.pop_front(paths) {
            Some(scope) => scope,
            None => return Err(PathResolutionError::EmptyScopeStack),
        };
        self.end_node = top_scope;
        self.edge_count += 1;
        Ok(())
    }

    /// Attempts to extend one path as part of the path-finding algorithm.  When calling this
    /// function, you are responsible for ensuring that `graph` already contains data for all of
    /// the possible edges that we might want to extend `path` with.
    ///
    /// The resulting extended paths will be added to `result`.  We have you pass that in as a
    /// parameter, instead of building it up ourselves, so that you have control over which
    /// particular collection type to use, and so that you can reuse result collections across
    /// multiple calls.
    pub fn extend<R: Extend<Path>>(&self, graph: &StackGraph, paths: &mut Paths, result: &mut R) {
        let extensions = graph.outgoing_edges(self.end_node);
        result.reserve(extensions.size_hint().0);
        for extension in extensions {
            let mut new_path = self.clone();
            // If there are errors adding this edge to the path, or resolving the resulting path,
            // just skip the edge — it's not a fatal error.
            if new_path.append(graph, paths, extension).is_err() {
                continue;
            }
            if new_path.resolve(graph, paths).is_err() {
                continue;
            }
            result.push(new_path);
        }
    }
}

impl Paths {
    /// Finds all paths reachable from a set of starting nodes, calling the `visit` closure for
    /// each one.
    ///
    /// This function will not return until all reachable paths have been processed, so `graph`
    /// must already contain a complete stack graph.  If you have a very large stack graph stored
    /// in some other storage system, and want more control over lazily loading only the necessary
    /// pieces, then you should code up your own loop that calls [`Path::extend`][] manually.
    ///
    /// [`Path::extend`]: struct.Path.html#method.extend
    pub fn find_all_paths<I, F>(&mut self, graph: &StackGraph, starting_nodes: I, mut visit: F)
    where
        I: IntoIterator<Item = Handle<Node>>,
        F: FnMut(&StackGraph, &mut Paths, Path),
    {
        let mut queue = starting_nodes
            .into_iter()
            .filter_map(|node| Path::from_node(graph, self, node))
            .collect::<VecDeque<_>>();
        while let Some(path) = queue.pop_front() {
            path.extend(graph, self, &mut queue);
            visit(graph, self, path);
        }
    }
}

/// A collection that can be used to receive the results of the [`Path::extend`][] method.
///
/// Note: There's an [open issue][std-extend] to add these methods to std's `Extend` trait.  If
/// that gets merged, we can drop this trait and use the std one instead.
///
/// [std-extend]: https://github.com/rust-lang/rust/issues/72631
pub trait Extend<T> {
    /// Reserve space for `additional` elements in the collection.
    fn reserve(&mut self, additional: usize);
    /// Add a new element to the collection.
    fn push(&mut self, item: T);
}

impl<T> Extend<T> for Vec<T> {
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn push(&mut self, item: T) {
        self.push(item);
    }
}

impl<T> Extend<T> for VecDeque<T> {
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn push(&mut self, item: T) {
        self.push_back(item);
    }
}

//-------------------------------------------------------------------------------------------------
// Path resolution state

/// Manages the state of a collection of paths built up as part of the path-finding algorithm.
pub struct Paths {
    scope_stacks: ListArena<Handle<Node>>,
    symbol_stacks: ListArena<ScopedSymbol>,
}

impl Paths {
    pub fn new() -> Paths {
        Paths {
            scope_stacks: List::new_arena(),
            symbol_stacks: List::new_arena(),
        }
    }
}
