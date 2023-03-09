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

/// Errors that can occur during the path resolution process.
#[derive(Debug)]
pub enum PathResolutionError {
    /// The path is cyclic, and the cycle is disallowed.
    DisallowedCycle,
    /// The path contains a _jump to scope_ node, but there are no scopes on the scope stack to
    /// jump to.
    EmptyScopeStack,
    /// The path contains a _pop symbol_ or _pop scoped symbol_ node, but there are no symbols on
    /// the symbol stack to pop off.
    EmptySymbolStack,
    /// The partial path contains multiple references to a scope stack variable, and those
    /// references can't unify on a single scope stack.
    IncompatibleScopeStackVariables,
    /// The partial path contains multiple references to a symbol stack variable, and those
    /// references can't unify on a single symbol stack.
    IncompatibleSymbolStackVariables,
    /// The partial path contains edges from multiple files.
    IncorrectFile,
    /// The path contains a _pop symbol_ or _pop scoped symbol_ node, but the symbol at the top of
    /// the symbol stack does not match.
    IncorrectPoppedSymbol,
    /// The path contains an edge whose source node does not match the sink node of the preceding
    /// edge.
    IncorrectSourceNode,
    /// The path contains a _pop scoped symbol_ node, but the symbol at the top of the symbol stack
    /// does not have an attached scope list to pop off.
    MissingAttachedScopeList,
    /// The path's scope stack does not satisfy the partial path's scope stack precondition.
    ScopeStackUnsatisfied,
    /// The path's symbol stack does not satisfy the partial path's symbol stack precondition.
    SymbolStackUnsatisfied,
    /// The partial path's postcondition references a symbol stack variable that isn't present in
    /// the precondition.
    UnboundSymbolStackVariable,
    /// The partial path's postcondition references a scope stack variable that isn't present in
    /// the precondition.
    UnboundScopeStackVariable,
    /// The path contains a _pop symbol_ node, but the symbol at the top of the symbol stack has an
    /// attached scope list that we weren't expecting.
    UnexpectedAttachedScopeList,
    /// A _push scoped symbol_ node referes to an exported scope node that doesn't exist.
    UnknownAttachedScope,
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
