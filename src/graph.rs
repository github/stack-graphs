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

use std::fmt::Display;
use std::ops::Deref;
use std::ops::Index;

use crate::arena::Arena;
use crate::arena::Handle;

//-------------------------------------------------------------------------------------------------
// Symbols

/// A name that we are trying to resolve using stack graphs.
///
/// This typically represents a portion of an identifier as it appears in the source language.  It
/// can also represent some other "operation" that can occur in source code, and which needs to be
/// modeled in a stack graph — for instance, many languages will use a "fake" symbol named `.` to
/// represent member access.
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
    /// Adds a symbol to the stack graph.
    pub fn add_symbol<S: ToString + ?Sized>(&mut self, symbol: &S) -> Handle<Symbol> {
        self.symbols.add(Symbol {
            symbol: symbol.to_string(),
        })
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
// Stack graphs

/// Contains all of the nodes and edges that make up a stack graph.
pub struct StackGraph {
    symbols: Arena<Symbol>,
}

impl StackGraph {
    /// Creates a new, initially empty stack graph.
    pub fn new() -> StackGraph {
        StackGraph {
            symbols: Arena::new(),
        }
    }
}
