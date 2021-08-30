// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Partial paths are "snippets" of paths that we can precalculate for each file that we analyze.
//!
//! Stack graphs are _incremental_, since we can produce a subgraph for each file without having
//! to look at the contents of any other file in the repo, or in any upstream or downstream
//! dependencies.
//!
//! This is great, because it means that when we receive a new commit for a repository, we only
//! have to examine, and generate new stack subgraphs for, the files that are changed as part of
//! that commit.
//!
//! Having done that, one possible way to find name binding paths would be to load in all of the
//! subgraphs for the files that belong to the current commit, union them together into the
//! combined graph for that commit, and run the [path-finding algorithm][] on that combined graph.
//! However, we think that this will require too much computation at query time.
//!
//! [path-finding algorithm]: ../paths/index.html
//!
//! Instead, we want to precompute parts of the path-finding algorithm, by calculating _partial
//! paths_ for each file.  Because stack graphs have limited places where a path can cross from one
//! file into another, we can calculate all of the possible partial paths that reach those
//! “import/export” points.
//!
//! At query time, we can then load in the _partial paths_ for each file, instead of the files'
//! full stack graph structure.  We can efficiently [concatenate][] partial paths together,
//! producing the original "full" path that represents a name binding.
//!
//! [concatenate]: struct.PartialPath.html#method.concatenate

use std::collections::VecDeque;
use std::convert::TryFrom;
use std::fmt::Display;
use std::num::NonZeroU32;

use controlled_option::ControlledOption;
use controlled_option::Niche;
use smallvec::SmallVec;

use crate::arena::Deque;
use crate::arena::DequeArena;
use crate::arena::Handle;
use crate::cycles::CycleDetector;
use crate::graph::Edge;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::NodeID;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::paths::Extend;
use crate::paths::Path;
use crate::paths::PathEdge;
use crate::paths::PathEdgeList;
use crate::paths::PathResolutionError;
use crate::paths::Paths;
use crate::paths::ScopeStack;
use crate::paths::ScopedSymbol;
use crate::paths::SymbolStack;
use crate::utils::cmp_option;
use crate::utils::equals_option;

//-------------------------------------------------------------------------------------------------
// Displaying stuff

/// This trait only exists because:
///
///   - we need `Display` implementations that dereference arena handles from our `StackGraph` and
///     `PartialPaths` bags o' crap,
///   - many of our arena-managed types can handles to _other_ arena-managed data, which we need to
///     recursively display as part of displaying the "outer" instance, and
///   - in particular, we sometimes need `&mut` access to the `PartialPaths` arenas.
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
trait DisplayWithPartialPaths {
    fn prepare(&mut self, _graph: &StackGraph, _partials: &mut PartialPaths) {}

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result;
}

/// Prepares and returns a `Display` implementation for a type `D` that implements
/// `DisplayWithPartialPaths`.  We only require `&mut` access to the `PartialPath` arenas while
/// creating the `Display` instance; the `Display` instance itself will only retain shared access
/// to the arenas.
fn display_with<'a, D>(
    mut value: D,
    graph: &'a StackGraph,
    partials: &'a mut PartialPaths,
) -> impl Display + 'a
where
    D: DisplayWithPartialPaths + 'a,
{
    value.prepare(graph, partials);
    DisplayWithPartialPathsWrapper {
        value,
        graph,
        partials,
    }
}

/// Returns a `Display` implementation that you can use inside of your `display_with` method to
/// display any recursive fields.  This assumes that the recursive fields have already been
/// prepared.
fn display_prepared<'a, D>(
    value: D,
    graph: &'a StackGraph,
    partials: &'a PartialPaths,
) -> impl Display + 'a
where
    D: DisplayWithPartialPaths + 'a,
{
    DisplayWithPartialPathsWrapper {
        value,
        graph,
        partials,
    }
}

#[doc(hidden)]
struct DisplayWithPartialPathsWrapper<'a, D> {
    value: D,
    graph: &'a StackGraph,
    partials: &'a PartialPaths,
}

impl<'a, D> Display for DisplayWithPartialPathsWrapper<'a, D>
where
    D: DisplayWithPartialPaths,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.value.display_with(self.graph, self.partials, f)
    }
}

//-------------------------------------------------------------------------------------------------
// Symbol stack variables

/// Represents an unknown list of scoped symbols.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Niche, Ord, PartialEq, PartialOrd)]
pub struct SymbolStackVariable(#[niche] NonZeroU32);

impl SymbolStackVariable {
    pub fn new(variable: u32) -> Option<SymbolStackVariable> {
        NonZeroU32::new(variable).map(SymbolStackVariable)
    }

    /// Creates a new symbol stack variable.  This constructor is used when creating a new, empty
    /// partial path, since there aren't any other variables that we need to be fresher than.
    fn initial() -> SymbolStackVariable {
        SymbolStackVariable(unsafe { NonZeroU32::new_unchecked(1) })
    }

    fn as_usize(self) -> usize {
        self.0.get() as usize
    }
}

impl Display for SymbolStackVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "%{}", self.0.get())
    }
}

impl Into<u32> for SymbolStackVariable {
    fn into(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for SymbolStackVariable {
    type Error = ();
    fn try_from(value: u32) -> Result<SymbolStackVariable, ()> {
        let value = NonZeroU32::new(value).ok_or(())?;
        Ok(SymbolStackVariable(value))
    }
}

//-------------------------------------------------------------------------------------------------
// Scope stack variables

/// Represents an unknown list of exported scopes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Niche, Ord, PartialEq, PartialOrd)]
pub struct ScopeStackVariable(#[niche] NonZeroU32);

impl ScopeStackVariable {
    pub fn new(variable: u32) -> Option<ScopeStackVariable> {
        NonZeroU32::new(variable).map(ScopeStackVariable)
    }

    /// Creates a new scope stack variable.  This constructor is used when creating a new, empty
    /// partial path, since there aren't any other variables that we need to be fresher than.
    fn initial() -> ScopeStackVariable {
        ScopeStackVariable(unsafe { NonZeroU32::new_unchecked(1) })
    }

    /// Creates a new scope stack variable that is fresher than all other variables in a partial
    /// path.  (You must calculate the maximum variable number already in use.)
    fn fresher_than(max_used: u32) -> ScopeStackVariable {
        ScopeStackVariable(unsafe { NonZeroU32::new_unchecked(max_used + 1) })
    }

    fn as_u32(self) -> u32 {
        self.0.get()
    }

    fn as_usize(self) -> usize {
        self.0.get() as usize
    }
}

impl Display for ScopeStackVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "${}", self.0.get())
    }
}

impl Into<u32> for ScopeStackVariable {
    fn into(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for ScopeStackVariable {
    type Error = ();
    fn try_from(value: u32) -> Result<ScopeStackVariable, ()> {
        let value = NonZeroU32::new(value).ok_or(())?;
        Ok(ScopeStackVariable(value))
    }
}

//-------------------------------------------------------------------------------------------------
// Symbol stack bindings

/// A mapping from symbol stack variables to symbol stacks.
pub struct SymbolStackBindings {
    bindings: SmallVec<[Option<SymbolStack>; 4]>,
}

impl SymbolStackBindings {
    /// Creates a new, empty set of symbol stack bindings.
    pub fn new() -> SymbolStackBindings {
        SymbolStackBindings {
            bindings: SmallVec::new(),
        }
    }

    /// Returns the symbol stack that a particular symbol stack variable matched.  Returns an error
    /// if that variable didn't match anything.
    pub fn get(&self, variable: SymbolStackVariable) -> Result<SymbolStack, PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            return Err(PathResolutionError::UnboundSymbolStackVariable);
        }
        self.bindings[index - 1].ok_or(PathResolutionError::UnboundSymbolStackVariable)
    }

    /// Adds a new binding from a symbol stack variable to the symbol stack that it matched.
    /// Returns an error if you try to bind a particular variable more than once.
    pub fn add(
        &mut self,
        variable: SymbolStackVariable,
        symbols: SymbolStack,
    ) -> Result<(), PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            self.bindings.resize_with(index, || None);
        }
        if self.bindings[index - 1].is_some() {
            return Err(PathResolutionError::IncompatibleSymbolStackVariables);
        }
        self.bindings[index - 1] = Some(symbols);
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Scope stack bindings

/// A mapping from scope stack variables to scope stacks.
pub struct ScopeStackBindings {
    bindings: SmallVec<[Option<ScopeStack>; 4]>,
}

impl ScopeStackBindings {
    /// Creates a new, empty set of scope stack bindings.
    pub fn new() -> ScopeStackBindings {
        ScopeStackBindings {
            bindings: SmallVec::new(),
        }
    }

    /// Returns the scope stack that a particular scope stack variable matched.  Returns an error
    /// if that variable didn't match anything.
    pub fn get(&self, variable: ScopeStackVariable) -> Result<ScopeStack, PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            return Err(PathResolutionError::UnboundScopeStackVariable);
        }
        self.bindings[index - 1].ok_or(PathResolutionError::UnboundScopeStackVariable)
    }

    /// Adds a new binding from a scope stack variable to the scope stack that it matched.  Returns
    /// an error if you try to bind a particular variable more than once.
    pub fn add(
        &mut self,
        variable: ScopeStackVariable,
        scopes: ScopeStack,
    ) -> Result<(), PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            self.bindings.resize_with(index, || None);
        }
        if self.bindings[index - 1].is_some() {
            return Err(PathResolutionError::IncompatibleScopeStackVariables);
        }
        self.bindings[index - 1] = Some(scopes);
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Partial symbol stacks

/// A symbol with an unknown, but possibly empty, list of exported scopes attached to it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PartialScopedSymbol {
    pub symbol: Handle<Symbol>,
    // Note that not having an attached scope list is _different_ than having an empty attached
    // scope list.
    pub scopes: ControlledOption<PartialScopeStack>,
}

impl PartialScopedSymbol {
    /// Matches this precondition symbol against a scoped symbol, unifying its contents with an
    /// existing set of bindings.
    pub fn match_symbol(
        self,
        graph: &StackGraph,
        symbol: ScopedSymbol,
        scope_bindings: &mut ScopeStackBindings,
    ) -> Result<(), PathResolutionError> {
        if graph[self.symbol] != graph[symbol.symbol] {
            return Err(PathResolutionError::SymbolStackUnsatisfied);
        }
        if !equals_option(
            self.scopes.into_option(),
            symbol.scopes.into_option(),
            |pre, sym| pre.match_stack(sym, scope_bindings).is_ok(),
        ) {
            return Err(PathResolutionError::SymbolStackUnsatisfied);
        }
        Ok(())
    }

    /// Returns whether two partial scoped symbols "match".  The symbols must be identical, and any
    /// attached scopes must also match.
    pub fn matches(self, partials: &mut PartialPaths, postcondition: PartialScopedSymbol) -> bool {
        if self.symbol != postcondition.symbol {
            return false;
        }

        // If one side has an attached scope but the other doesn't, then the scoped symbols don't
        // match.
        if self.scopes.is_none() != postcondition.scopes.is_none() {
            return false;
        }

        // Otherwise, if both sides have an attached scope, they have to be compatible.
        if let Some(precondition_scopes) = self.scopes.into_option() {
            if let Some(postcondition_scopes) = postcondition.scopes.into_option() {
                return precondition_scopes.matches(partials, postcondition_scopes);
            }
        }

        true
    }

    /// Applies a set of bindings to this partial scoped symbol, producing a new scoped symbol.
    pub fn apply_bindings(
        self,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        scope_bindings: &ScopeStackBindings,
    ) -> Result<ScopedSymbol, PathResolutionError> {
        let scopes = match self.scopes.into_option() {
            Some(scopes) => Some(scopes.apply_bindings(paths, partials, scope_bindings)?),
            None => None,
        };
        Ok(ScopedSymbol {
            symbol: self.symbol,
            scopes: scopes.into(),
        })
    }

    pub fn equals(&self, partials: &mut PartialPaths, other: &PartialScopedSymbol) -> bool {
        self.symbol == other.symbol
            && equals_option(
                self.scopes.into_option(),
                other.scopes.into_option(),
                |a, b| a.equals(partials, b),
            )
    }

    pub fn cmp(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        other: &PartialScopedSymbol,
    ) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
            .then_with(|| graph[self.symbol].cmp(&graph[other.symbol]))
            .then_with(|| {
                cmp_option(
                    self.scopes.into_option(),
                    other.scopes.into_option(),
                    |a, b| a.cmp(partials, b),
                )
            })
    }

    pub fn display<'a>(
        self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }
}

impl DisplayWithPartialPaths for PartialScopedSymbol {
    fn prepare(&mut self, graph: &StackGraph, partials: &mut PartialPaths) {
        if let Some(mut scopes) = self.scopes.into_option() {
            scopes.prepare(graph, partials);
            self.scopes = scopes.into();
        }
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        if let Some(scopes) = self.scopes.into_option() {
            write!(
                f,
                "{}/({})",
                self.symbol.display(graph),
                display_prepared(scopes, graph, partials)
            )
        } else {
            write!(f, "{}", self.symbol.display(graph))
        }
    }
}

/// A pattern that might match against a symbol stack.  Consists of a (possibly empty) list of
/// partial scoped symbols, along with an optional symbol stack variable.
#[repr(C)]
#[derive(Clone, Copy, Niche)]
pub struct PartialSymbolStack {
    #[niche]
    symbols: Deque<PartialScopedSymbol>,
    variable: ControlledOption<SymbolStackVariable>,
}

impl PartialSymbolStack {
    /// Returns whether this partial symbol stack can match the empty symbol stack.
    #[inline(always)]
    pub fn can_match_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Returns whether this partial symbol stack can _only_ match the empty symbol stack.
    #[inline(always)]
    pub fn can_only_match_empty(&self) -> bool {
        self.symbols.is_empty() && self.variable.is_none()
    }

    /// Returns whether this partial symbol stack contains any symbols.
    #[inline(always)]
    pub fn contains_symbols(&self) -> bool {
        !self.symbols.is_empty()
    }

    /// Returns an empty partial symbol stack.
    pub fn empty() -> PartialSymbolStack {
        PartialSymbolStack {
            symbols: Deque::empty(),
            variable: ControlledOption::none(),
        }
    }

    /// Returns a partial symbol stack containing only a symbol stack variable.
    pub fn from_variable(variable: SymbolStackVariable) -> PartialSymbolStack {
        PartialSymbolStack {
            symbols: Deque::empty(),
            variable: ControlledOption::some(variable),
        }
    }

    /// Pushes a new [`PartialScopedSymbol`][] onto the front of this partial symbol stack.
    pub fn push_front(&mut self, partials: &mut PartialPaths, symbol: PartialScopedSymbol) {
        self.symbols
            .push_front(&mut partials.partial_symbol_stacks, symbol);
    }

    /// Pushes a new [`PartialScopedSymbol`][] onto the back of this partial symbol stack.
    pub fn push_back(&mut self, partials: &mut PartialPaths, symbol: PartialScopedSymbol) {
        self.symbols
            .push_back(&mut partials.partial_symbol_stacks, symbol);
    }

    /// Removes and returns the [`PartialScopedSymbol`][] at the front of this partial symbol
    /// stack.  If the stack is empty, returns `None`.
    pub fn pop_front(&mut self, partials: &mut PartialPaths) -> Option<PartialScopedSymbol> {
        self.symbols
            .pop_front(&mut partials.partial_symbol_stacks)
            .copied()
    }

    /// Removes and returns the [`PartialScopedSymbol`][] at the back of this partial symbol stack.
    /// If the stack is empty, returns `None`.
    pub fn pop_back(&mut self, partials: &mut PartialPaths) -> Option<PartialScopedSymbol> {
        self.symbols
            .pop_back(&mut partials.partial_symbol_stacks)
            .copied()
    }

    pub fn display<'a>(
        self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }

    /// Matches this precondition against a symbol stack, stashing away the unmatched portion of
    /// the stack in the bindings.
    pub fn match_stack(
        mut self,
        graph: &StackGraph,
        paths: &Paths,
        partial_paths: &mut PartialPaths,
        mut stack: SymbolStack,
        symbol_bindings: &mut SymbolStackBindings,
        scope_bindings: &mut ScopeStackBindings,
    ) -> Result<(), PathResolutionError> {
        // First verify that every symbol in the precondition has a corresponding matching symbol
        // in the symbol stack.
        while let Some(precondition_symbol) = self.pop_front(partial_paths) {
            match stack.pop_front(paths) {
                // This will update scope_bindings if the precondition symbol has an attached scope
                // stack variable.
                Some(symbol) => precondition_symbol.match_symbol(graph, symbol, scope_bindings)?,
                // The precondition is longer than the symbol stack, which is an error.
                None => return Err(PathResolutionError::SymbolStackUnsatisfied),
            }
        }

        // If there's anything remaining on the symbol stack, there must be a symbol stack variable
        // that can capture those symbols.
        match self.variable.into_option() {
            Some(variable) => symbol_bindings.add(variable, stack),
            None if !stack.is_empty() => Err(PathResolutionError::SymbolStackUnsatisfied),
            _ => Ok(()),
        }
    }

    /// Returns whether two partial symbol stacks "match".  They must be the same length, and each
    /// respective partial scoped symbol must match.
    pub fn matches(mut self, partials: &mut PartialPaths, mut other: PartialSymbolStack) -> bool {
        while let Some(self_element) = self.pop_front(partials) {
            if let Some(other_element) = other.pop_front(partials) {
                if !self_element.matches(partials, other_element) {
                    return false;
                }
            } else {
                // Stacks aren't the same length.
                return false;
            }
        }
        if other.contains_symbols() {
            // Stacks aren't the same length.
            return false;
        }
        self.variable.into_option() == other.variable.into_option()
    }

    /// Applies a set of bindings to this partial symbol stack, producing a new symbol stack.
    pub fn apply_bindings(
        mut self,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        symbol_bindings: &SymbolStackBindings,
        scope_bindings: &ScopeStackBindings,
    ) -> Result<SymbolStack, PathResolutionError> {
        let mut result = match self.variable.into_option() {
            Some(variable) => symbol_bindings.get(variable)?,
            None => SymbolStack::empty(),
        };
        while let Some(partial_symbol) = self.pop_back(partials) {
            let symbol = partial_symbol.apply_bindings(paths, partials, scope_bindings)?;
            result.push_front(paths, symbol);
        }
        Ok(result)
    }

    pub fn equals(mut self, partials: &mut PartialPaths, mut other: PartialSymbolStack) -> bool {
        while let Some(self_symbol) = self.pop_front(partials) {
            if let Some(other_symbol) = other.pop_front(partials) {
                if !self_symbol.equals(partials, &other_symbol) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if !other.symbols.is_empty() {
            return false;
        }
        equals_option(
            self.variable.into_option(),
            other.variable.into_option(),
            |a, b| a == b,
        )
    }

    pub fn cmp(
        mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        mut other: PartialSymbolStack,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        while let Some(self_symbol) = self.pop_front(partials) {
            if let Some(other_symbol) = other.pop_front(partials) {
                match self_symbol.cmp(graph, partials, &other_symbol) {
                    Ordering::Equal => continue,
                    result @ _ => return result,
                }
            } else {
                return Ordering::Greater;
            }
        }
        if !other.symbols.is_empty() {
            return Ordering::Less;
        }
        cmp_option(
            self.variable.into_option(),
            other.variable.into_option(),
            |a, b| a.cmp(&b),
        )
    }

    /// Returns an iterator over the contents of this partial symbol stack.
    pub fn iter<'a>(
        &self,
        partials: &'a mut PartialPaths,
    ) -> impl Iterator<Item = PartialScopedSymbol> + 'a {
        self.symbols
            .iter(&mut partials.partial_symbol_stacks)
            .copied()
    }

    /// Returns an iterator over the contents of this partial symbol stack, with no guarantee
    /// about the ordering of the elements.
    pub fn iter_unordered<'a>(
        &self,
        partials: &'a PartialPaths,
    ) -> impl Iterator<Item = PartialScopedSymbol> + 'a {
        self.symbols
            .iter_unordered(&partials.partial_symbol_stacks)
            .copied()
    }

    fn ensure_both_directions(&mut self, partials: &mut PartialPaths) {
        self.symbols
            .ensure_backwards(&mut partials.partial_symbol_stacks);
        self.symbols
            .ensure_forwards(&mut partials.partial_symbol_stacks);
    }
}

impl DisplayWithPartialPaths for PartialSymbolStack {
    fn prepare(&mut self, graph: &StackGraph, partials: &mut PartialPaths) {
        // Ensure that our deque is pointed forwards while we still have a mutable reference to the
        // arena.
        self.symbols
            .ensure_forwards(&mut partials.partial_symbol_stacks);
        // And then prepare each symbol in the stack.
        let mut symbols = self.symbols;
        while let Some(mut symbol) = symbols
            .pop_front(&mut partials.partial_symbol_stacks)
            .copied()
        {
            symbol.prepare(graph, partials);
        }
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        for symbol in self.symbols.iter_reused(&partials.partial_symbol_stacks) {
            symbol.display_with(graph, partials, f)?;
        }
        if let Some(variable) = self.variable.into_option() {
            if self.symbols.is_empty() {
                write!(f, "{}", variable)?;
            } else {
                write!(f, ",{}", variable)?;
            }
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Partial scope stacks

/// A pattern that might match against a scope stack.  Consists of a (possibly empty) list of
/// exported scopes, along with an optional scope stack variable.
#[repr(C)]
#[derive(Clone, Copy, Niche)]
pub struct PartialScopeStack {
    #[niche]
    scopes: Deque<Handle<Node>>,
    variable: ControlledOption<ScopeStackVariable>,
}

impl PartialScopeStack {
    /// Returns whether this partial scope stack can _only_ match the empty scope stack.
    #[inline(always)]
    pub fn can_only_match_empty(&self) -> bool {
        self.scopes.is_empty() && self.variable.is_none()
    }

    /// Returns whether this partial scope stack contains any scopes.
    #[inline(always)]
    pub fn contains_scopes(&self) -> bool {
        !self.scopes.is_empty()
    }

    /// Returns an empty partial scope stack.
    pub fn empty() -> PartialScopeStack {
        PartialScopeStack {
            scopes: Deque::empty(),
            variable: ControlledOption::none(),
        }
    }

    /// Returns a partial scope stack containing only a scope stack variable.
    pub fn from_variable(variable: ScopeStackVariable) -> PartialScopeStack {
        PartialScopeStack {
            scopes: Deque::empty(),
            variable: ControlledOption::some(variable),
        }
    }

    /// Matches this partial scope stack against a scope stack, unifying any scope stack variables
    /// with an existing set of bindings.
    pub fn match_stack(
        &self,
        stack: ScopeStack,
        bindings: &mut ScopeStackBindings,
    ) -> Result<(), PathResolutionError> {
        // TODO: We realized that we're assuming, but not checking, that the partial scope stack's
        // scope prefix is empty.  No current test cases fail with this assumption, but we should
        // validate this more carefully.
        assert!(self.scopes.is_empty());
        match self.variable.into_option() {
            Some(variable) => bindings.add(variable, stack),
            None if !stack.is_empty() => Err(PathResolutionError::ScopeStackUnsatisfied),
            _ => Ok(()),
        }
    }

    /// Returns whether two partial scope stacks match exactly the same set of scope stacks.
    pub fn matches(mut self, partials: &mut PartialPaths, mut other: PartialScopeStack) -> bool {
        while let Some(self_element) = self.pop_front(partials) {
            if let Some(other_element) = other.pop_front(partials) {
                if self_element != other_element {
                    return false;
                }
            } else {
                // Stacks aren't the same length.
                return false;
            }
        }
        if other.contains_scopes() {
            // Stacks aren't the same length.
            return false;
        }
        self.variable.into_option() == other.variable.into_option()
    }

    /// Applies a set of scope stack bindings to this partial scope stack, producing a new scope
    /// stack.
    pub fn apply_bindings(
        mut self,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        bindings: &ScopeStackBindings,
    ) -> Result<ScopeStack, PathResolutionError> {
        let mut result = match self.variable.into_option() {
            Some(variable) => bindings.get(variable)?,
            None => ScopeStack::empty(),
        };
        while let Some(scope) = self.pop_back(partials) {
            result.push_front(paths, scope);
        }
        Ok(result)
    }

    /// Given two partial scope stacks, returns the largest possible partial scope stack such that
    /// any scope stack that satisfies the result also satisfies both inputs.  This takes into
    /// account any existing variable assignments, and updates those variable assignments with
    /// whatever constraints are necessary to produce a correct result.
    ///
    /// Note that this operation is commutative.  (Concatenating partial paths, defined in
    /// [`PartialPath::concatenate`][], is not.)
    pub fn unify(
        self,
        partials: &mut PartialPaths,
        mut rhs: PartialScopeStack,
        bindings: &mut PartialScopeStackBindings,
    ) -> Result<PartialScopeStack, PathResolutionError> {
        let mut lhs = self;
        let original_rhs = rhs;

        // First, look at the shortest common prefix of lhs and rhs, and verify that they match.
        while lhs.contains_scopes() && rhs.contains_scopes() {
            let lhs_front = lhs.pop_front(partials).unwrap();
            let rhs_front = rhs.pop_front(partials).unwrap();
            if lhs_front != rhs_front {
                return Err(PathResolutionError::ScopeStackUnsatisfied);
            }
        }

        // Now at most one stack still has scopes.  Zero, one, or both of them have variables.
        // Let's do a case analysis on all of those possibilities.

        // CASE 1:
        // Both lhs and rhs have no more scopes.  The answer is always yes, and any variables that
        // are present get bound.  (If both sides have variables, then one variable gets bound to
        // the other, since both lhs and rhs will match _any other scope stack_ at this point.  If
        // only one side has a variable, then the variable gets bound to the empty stack.)
        //
        //     lhs           rhs
        // ============  ============
        //  ()            ()            => yes either
        //  ()            () $2         => yes rhs, $2 => ()
        //  () $1         ()            => yes lhs, $1 => ()
        //  () $1         () $2         => yes lhs, $2 => $1
        if !lhs.contains_scopes() && !rhs.contains_scopes() {
            match (lhs.variable.into_option(), rhs.variable.into_option()) {
                (None, None) => return Ok(self),
                (None, Some(var)) => {
                    bindings.add(partials, var, PartialScopeStack::empty())?;
                    return Ok(original_rhs);
                }
                (Some(var), None) => {
                    bindings.add(partials, var, PartialScopeStack::empty())?;
                    return Ok(self);
                }
                (Some(lhs_var), Some(rhs_var)) => {
                    bindings.add(partials, rhs_var, PartialScopeStack::from_variable(lhs_var))?;
                    return Ok(self);
                }
            }
        }

        // CASE 2:
        // One of the stacks contains scopes and the other doesn't, and the “empty” stack doesn't
        // have a variable.  Since there's no variable on the empty side to capture the remaining
        // content on the non-empty side, the answer is always no.
        //
        //     lhs           rhs
        // ============  ============
        //  ()            (stuff)       => NO
        //  ()            (stuff) $2    => NO
        //  (stuff)       ()            => NO
        //  (stuff) $1    ()            => NO
        if !lhs.contains_scopes() && lhs.variable.is_none() {
            return Err(PathResolutionError::ScopeStackUnsatisfied);
        }
        if !rhs.contains_scopes() && rhs.variable.is_none() {
            return Err(PathResolutionError::ScopeStackUnsatisfied);
        }

        // CASE 3:
        // One of the stacks contains scopes and the other doesn't, and the “empty” stack _does_
        // have a variable.  That means the answer is YES, and the “empty” side's variable needs to
        // capture the entirety of the non-empty side.
        //
        //     lhs           rhs
        // ============  ============
        //  () $1         (stuff)       => yes rhs,  $1 => rhs
        //  () $1         (stuff) $2    => yes rhs,  $1 => rhs
        //  (stuff)       () $2         => yes lhs,  $2 => lhs
        //  (stuff) $1    () $2         => yes lhs,  $2 => lhs
        if lhs.contains_scopes() {
            let rhs_variable = rhs.variable.into_option().unwrap();
            bindings.add(partials, rhs_variable, lhs)?;
            return Ok(self);
        }
        if rhs.contains_scopes() {
            let lhs_variable = lhs.variable.into_option().unwrap();
            bindings.add(partials, lhs_variable, rhs)?;
            return Ok(original_rhs);
        }

        unreachable!();
    }

    /// Pushes a new [`Node`][] onto the front of this partial scope stack.  The node must be an
    /// _exported scope node_.
    ///
    /// [`Node`]: ../graph/enum.Node.html
    pub fn push_front(&mut self, partials: &mut PartialPaths, node: Handle<Node>) {
        self.scopes
            .push_front(&mut partials.partial_scope_stacks, node);
    }

    /// Pushes a new [`Node`][] onto the back of this partial scope stack.  The node must be an
    /// _exported scope node_.
    ///
    /// [`Node`]: ../graph/enum.Node.html
    pub fn push_back(&mut self, partials: &mut PartialPaths, node: Handle<Node>) {
        self.scopes
            .push_back(&mut partials.partial_scope_stacks, node);
    }

    /// Removes and returns the [`Node`][] at the front of this partial scope stack.  If the stack
    /// does not contain any exported scope nodes, returns `None`.
    pub fn pop_front(&mut self, partials: &mut PartialPaths) -> Option<Handle<Node>> {
        self.scopes
            .pop_front(&mut partials.partial_scope_stacks)
            .copied()
    }

    /// Removes and returns the [`Node`][] at the back of this partial scope stack.  If the stack
    /// does not contain any exported scope nodes, returns `None`.
    pub fn pop_back(&mut self, partials: &mut PartialPaths) -> Option<Handle<Node>> {
        self.scopes
            .pop_back(&mut partials.partial_scope_stacks)
            .copied()
    }

    /// Returns the scope stack variable at the end of this partial scope stack.  If the stack does
    /// not contain a scope stack variable, returns `None`.
    pub fn variable(&self) -> Option<ScopeStackVariable> {
        self.variable.into_option()
    }

    pub fn equals(self, partials: &mut PartialPaths, other: PartialScopeStack) -> bool {
        self.scopes
            .equals_with(&mut partials.partial_scope_stacks, other.scopes, |a, b| {
                *a == *b
            })
            && equals_option(
                self.variable.into_option(),
                other.variable.into_option(),
                |a, b| a == b,
            )
    }

    pub fn cmp(self, partials: &mut PartialPaths, other: PartialScopeStack) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
            .then_with(|| {
                self.scopes
                    .cmp_with(&mut partials.partial_scope_stacks, other.scopes, |a, b| {
                        a.cmp(b)
                    })
            })
            .then_with(|| {
                cmp_option(
                    self.variable.into_option(),
                    other.variable.into_option(),
                    |a, b| a.cmp(&b),
                )
            })
    }

    /// Returns an iterator over the scopes in this partial scope stack.
    pub fn iter_scopes<'a>(
        &self,
        partials: &'a mut PartialPaths,
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        self.scopes
            .iter(&mut partials.partial_scope_stacks)
            .copied()
    }

    /// Returns an iterator over the contents of this partial scope stack, with no guarantee
    /// about the ordering of the elements.
    pub fn iter_unordered<'a>(
        &self,
        partials: &'a PartialPaths,
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        self.scopes
            .iter_unordered(&partials.partial_scope_stacks)
            .copied()
    }

    pub fn display<'a>(
        self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }

    fn ensure_both_directions(&mut self, partials: &mut PartialPaths) {
        self.scopes
            .ensure_backwards(&mut partials.partial_scope_stacks);
        self.scopes
            .ensure_forwards(&mut partials.partial_scope_stacks);
    }
}

impl DisplayWithPartialPaths for PartialScopeStack {
    fn prepare(&mut self, _graph: &StackGraph, partials: &mut PartialPaths) {
        self.scopes
            .ensure_forwards(&mut partials.partial_scope_stacks);
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        let mut first = true;
        for scope in self.scopes.iter_reused(&partials.partial_scope_stacks) {
            if first {
                first = false;
            } else {
                write!(f, ",")?;
            }
            write!(f, "{:#}", scope.display(graph))?;
        }
        if let Some(variable) = self.variable.into_option() {
            if self.scopes.is_empty() {
                write!(f, "{}", variable)?;
            } else {
                write!(f, ",{}", variable)?;
            }
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Partial symbol bindings

pub struct PartialSymbolStackBindings {
    bindings: SmallVec<[Option<PartialSymbolStack>; 4]>,
}

impl PartialSymbolStackBindings {
    /// Creates a new, empty set of partial symbol stack bindings.
    pub fn new() -> PartialSymbolStackBindings {
        PartialSymbolStackBindings {
            bindings: SmallVec::new(),
        }
    }

    /// Returns the partial symbol stack that a particular symbol stack variable matched.  Returns an
    /// error if that variable didn't match anything.
    pub fn get(
        &self,
        variable: SymbolStackVariable,
    ) -> Result<PartialSymbolStack, PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            return Err(PathResolutionError::UnboundSymbolStackVariable);
        }
        self.bindings[index - 1].ok_or(PathResolutionError::UnboundSymbolStackVariable)
    }

    /// Adds a new binding from a symbol stack variable to the partial symbol stack that it
    /// matched.  Returns an error if you try to bind a particular variable more than once.
    pub fn add(
        &mut self,
        variable: SymbolStackVariable,
        symbols: PartialSymbolStack,
    ) -> Result<(), PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            self.bindings.resize_with(index, || None);
        }
        if self.bindings[index - 1].is_some() {
            return Err(PathResolutionError::IncompatibleSymbolStackVariables);
        }
        self.bindings[index - 1] = Some(symbols);
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Partial scope bindings

pub struct PartialScopeStackBindings {
    bindings: SmallVec<[Option<PartialScopeStack>; 4]>,
}

impl PartialScopeStackBindings {
    /// Creates a new, empty set of partial scope stack bindings.
    pub fn new() -> PartialScopeStackBindings {
        PartialScopeStackBindings {
            bindings: SmallVec::new(),
        }
    }

    /// Returns the partial scope stack that a particular scope stack variable matched.  Returns an error
    /// if that variable didn't match anything.
    pub fn get(
        &self,
        variable: ScopeStackVariable,
    ) -> Result<PartialScopeStack, PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            return Err(PathResolutionError::UnboundScopeStackVariable);
        }
        self.bindings[index - 1].ok_or(PathResolutionError::UnboundScopeStackVariable)
    }

    /// Adds a new binding from a scope stack variable to the partial scope stack that it matched.  Returns
    /// an error if you try to bind a particular variable more than once.
    pub fn add(
        &mut self,
        partials: &mut PartialPaths,
        variable: ScopeStackVariable,
        mut scopes: PartialScopeStack,
    ) -> Result<(), PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            self.bindings.resize_with(index, || None);
        }
        if let Some(old_binding) = self.bindings[index - 1] {
            scopes = scopes.unify(partials, old_binding, self)?;
        }
        self.bindings[index - 1] = Some(scopes);
        Ok(())
    }

    pub fn display<'a>(
        &'a mut self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }
}

impl<'a> DisplayWithPartialPaths for &'a mut PartialScopeStackBindings {
    fn prepare(&mut self, graph: &StackGraph, partials: &mut PartialPaths) {
        for binding in &mut self.bindings {
            if let Some(binding) = binding.as_mut() {
                binding.prepare(graph, partials);
            }
        }
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (idx, binding) in self.bindings.iter().enumerate() {
            if let Some(binding) = binding.as_ref() {
                if first {
                    first = false;
                } else {
                    write!(f, ", ")?;
                }
                write!(
                    f,
                    "${} => ({})",
                    idx + 1,
                    display_prepared(*binding, graph, partials)
                )?;
            }
        }
        write!(f, "}}")
    }
}

//-------------------------------------------------------------------------------------------------
// Edge lists

#[repr(C)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PartialPathEdge {
    pub source_node_id: NodeID,
    pub precedence: i32,
}

impl From<PartialPathEdge> for PathEdge {
    fn from(other: PartialPathEdge) -> PathEdge {
        PathEdge {
            source_node_id: other.source_node_id,
            precedence: other.precedence,
        }
    }
}

impl PartialPathEdge {
    /// Returns whether one edge shadows another.  Note that shadowing is not commutative — if path
    /// A shadows path B, the reverse is not true.
    pub fn shadows(self, other: PartialPathEdge) -> bool {
        self.source_node_id == other.source_node_id && self.precedence > other.precedence
    }

    pub fn display<'a>(
        self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }
}

impl DisplayWithPartialPaths for PartialPathEdge {
    fn display_with(
        &self,
        graph: &StackGraph,
        _partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match graph.node_for_id(self.source_node_id) {
            Some(node) => write!(f, "{:#}", node.display(graph))?,
            None => write!(f, "[missing]")?,
        }
        if self.precedence != 0 {
            write!(f, "({})", self.precedence)?;
        }
        Ok(())
    }
}

/// The edges in a path keep track of precedence information so that we can correctly handle
/// shadowed definitions.
#[repr(C)]
#[derive(Clone, Copy, Niche)]
pub struct PartialPathEdgeList {
    #[niche]
    edges: Deque<PartialPathEdge>,
    length: usize,
}

impl PartialPathEdgeList {
    /// Returns whether this edge list is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns an empty edge list.
    pub fn empty() -> PartialPathEdgeList {
        PartialPathEdgeList {
            edges: Deque::empty(),
            length: 0,
        }
    }

    /// Pushes a new edge onto the front of this edge list.
    pub fn push_front(&mut self, partials: &mut PartialPaths, edge: PartialPathEdge) {
        self.length += 1;
        self.edges
            .push_front(&mut partials.partial_path_edges, edge);
    }

    /// Pushes a new edge onto the back of this edge list.
    pub fn push_back(&mut self, partials: &mut PartialPaths, edge: PartialPathEdge) {
        self.length += 1;
        self.edges.push_back(&mut partials.partial_path_edges, edge);
    }

    /// Removes and returns the edge at the front of this edge list.  If the list is empty, returns
    /// `None`.
    pub fn pop_front(&mut self, partials: &mut PartialPaths) -> Option<PartialPathEdge> {
        let result = self.edges.pop_front(&mut partials.partial_path_edges);
        if result.is_some() {
            self.length -= 1;
        }
        result.copied()
    }

    /// Removes and returns the edge at the back of this edge list.  If the list is empty, returns
    /// `None`.
    pub fn pop_back(&mut self, partials: &mut PartialPaths) -> Option<PartialPathEdge> {
        let result = self.edges.pop_back(&mut partials.partial_path_edges);
        if result.is_some() {
            self.length -= 1;
        }
        result.copied()
    }

    pub fn display<'a>(
        self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }

    /// Returns whether one edge list shadows another.  Note that shadowing is not commutative — if
    /// path A shadows path B, the reverse is not true.
    pub fn shadows(mut self, partials: &mut PartialPaths, mut other: PartialPathEdgeList) -> bool {
        while let Some(self_edge) = self.pop_front(partials) {
            if let Some(other_edge) = other.pop_front(partials) {
                if self_edge.shadows(other_edge) {
                    return true;
                }
            } else {
                return false;
            }
        }
        false
    }

    pub fn equals(mut self, partials: &mut PartialPaths, mut other: PartialPathEdgeList) -> bool {
        while let Some(self_edge) = self.pop_front(partials) {
            if let Some(other_edge) = other.pop_front(partials) {
                if self_edge != other_edge {
                    return false;
                }
            } else {
                return false;
            }
        }
        other.edges.is_empty()
    }

    pub fn cmp(
        mut self,
        partials: &mut PartialPaths,
        mut other: PartialPathEdgeList,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        while let Some(self_edge) = self.pop_front(partials) {
            if let Some(other_edge) = other.pop_front(partials) {
                match self_edge.cmp(&other_edge) {
                    Ordering::Equal => continue,
                    result @ _ => return result,
                }
            } else {
                return Ordering::Greater;
            }
        }
        if other.edges.is_empty() {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }

    /// Returns an iterator over the contents of this edge list.
    pub fn iter<'a>(
        &self,
        partials: &'a mut PartialPaths,
    ) -> impl Iterator<Item = PartialPathEdge> + 'a {
        self.edges.iter(&mut partials.partial_path_edges).copied()
    }

    /// Returns an iterator over the contents of this edge list, with no guarantee about the
    /// ordering of the elements.
    pub fn iter_unordered<'a>(
        &self,
        partials: &'a PartialPaths,
    ) -> impl Iterator<Item = PartialPathEdge> + 'a {
        self.edges
            .iter_unordered(&partials.partial_path_edges)
            .copied()
    }

    fn ensure_both_directions(&mut self, partials: &mut PartialPaths) {
        self.edges
            .ensure_backwards(&mut partials.partial_path_edges);
        self.edges.ensure_forwards(&mut partials.partial_path_edges);
    }
}

impl DisplayWithPartialPaths for PartialPathEdgeList {
    fn prepare(&mut self, graph: &StackGraph, partials: &mut PartialPaths) {
        self.edges.ensure_forwards(&mut partials.partial_path_edges);
        let mut edges = self.edges;
        while let Some(mut edge) = edges.pop_front(&mut partials.partial_path_edges).copied() {
            edge.prepare(graph, partials);
        }
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        for edge in self.edges.iter_reused(&partials.partial_path_edges) {
            edge.display_with(graph, partials, f)?;
        }
        Ok(())
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
#[derive(Clone)]
pub struct PartialPath {
    pub start_node: Handle<Node>,
    pub end_node: Handle<Node>,
    pub symbol_stack_precondition: PartialSymbolStack,
    pub symbol_stack_postcondition: PartialSymbolStack,
    pub scope_stack_precondition: PartialScopeStack,
    pub scope_stack_postcondition: PartialScopeStack,
    pub edges: PartialPathEdgeList,
}

impl PartialPath {
    /// Creates a new empty partial path starting at a stack graph node.
    pub fn from_node(
        graph: &StackGraph,
        partials: &mut PartialPaths,
        node: Handle<Node>,
    ) -> Result<PartialPath, PathResolutionError> {
        let initial_symbol_stack = SymbolStackVariable::initial();
        let initial_scope_stack = ScopeStackVariable::initial();
        let symbol_stack_precondition = PartialSymbolStack::from_variable(initial_symbol_stack);
        let mut symbol_stack_postcondition =
            PartialSymbolStack::from_variable(initial_symbol_stack);
        let mut scope_stack_precondition = PartialScopeStack::from_variable(initial_scope_stack);
        let mut scope_stack_postcondition = PartialScopeStack::from_variable(initial_scope_stack);

        let inner_node = &graph[node];
        if let Node::PushScopedSymbol(inner_node) = inner_node {
            scope_stack_precondition = PartialScopeStack::empty();
            scope_stack_postcondition = PartialScopeStack::empty();
            let scope = graph
                .node_for_id(inner_node.scope)
                .ok_or(PathResolutionError::UnknownAttachedScope)?;
            scope_stack_postcondition.push_front(partials, scope);
            let initial_symbol = PartialScopedSymbol {
                symbol: inner_node.symbol,
                scopes: ControlledOption::some(scope_stack_postcondition),
            };
            symbol_stack_postcondition.push_front(partials, initial_symbol);
        } else if let Node::PushSymbol(inner_node) = inner_node {
            scope_stack_precondition = PartialScopeStack::empty();
            scope_stack_postcondition = PartialScopeStack::empty();
            let initial_symbol = PartialScopedSymbol {
                symbol: inner_node.symbol,
                scopes: ControlledOption::none(),
            };
            symbol_stack_postcondition.push_front(partials, initial_symbol);
        }

        Ok(PartialPath {
            start_node: node,
            end_node: node,
            symbol_stack_precondition,
            symbol_stack_postcondition,
            scope_stack_precondition,
            scope_stack_postcondition,
            edges: PartialPathEdgeList::empty(),
        })
    }

    /// Returns whether one path shadows another.  Note that shadowing is not commutative — if path
    /// A shadows path B, the reverse is not true.
    pub fn shadows(&self, partials: &mut PartialPaths, other: &PartialPath) -> bool {
        self.edges.shadows(partials, other.edges)
    }

    pub fn equals(&self, partials: &mut PartialPaths, other: &PartialPath) -> bool {
        self.start_node == other.start_node
            && self.end_node == other.end_node
            && self
                .symbol_stack_precondition
                .equals(partials, other.symbol_stack_precondition)
            && self
                .symbol_stack_postcondition
                .equals(partials, other.symbol_stack_postcondition)
            && self
                .scope_stack_precondition
                .equals(partials, other.scope_stack_precondition)
            && self
                .scope_stack_postcondition
                .equals(partials, other.scope_stack_postcondition)
            && self.edges.equals(partials, other.edges)
    }

    pub fn cmp(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        other: &PartialPath,
    ) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
            .then_with(|| self.start_node.cmp(&other.start_node))
            .then_with(|| self.end_node.cmp(&other.end_node))
            .then_with(|| {
                self.symbol_stack_precondition
                    .cmp(graph, partials, other.symbol_stack_precondition)
            })
            .then_with(|| {
                self.symbol_stack_postcondition.cmp(
                    graph,
                    partials,
                    other.symbol_stack_postcondition,
                )
            })
            .then_with(|| {
                self.scope_stack_precondition
                    .cmp(partials, other.scope_stack_precondition)
            })
            .then_with(|| {
                self.scope_stack_postcondition
                    .cmp(partials, other.scope_stack_postcondition)
            })
            .then_with(|| self.edges.cmp(partials, other.edges))
    }

    /// A partial path is _as complete as possible_ if we cannot extend it any further within the
    /// current file.  This represents the maximal amount of work that we can pre-compute at index
    /// time.
    pub fn is_complete_as_possible(&self, graph: &StackGraph) -> bool {
        match &graph[self.start_node] {
            Node::Root(_) => (),
            Node::ExportedScope(_) => (),
            node @ Node::PushScopedSymbol(_) | node @ Node::PushSymbol(_) => {
                if !node.is_reference() {
                    return false;
                } else if !self.symbol_stack_precondition.can_match_empty() {
                    return false;
                }
            }
            _ => return false,
        }

        match &graph[self.end_node] {
            Node::Root(_) => (),
            Node::JumpTo(_) => (),
            node @ Node::PopScopedSymbol(_) | node @ Node::PopSymbol(_) => {
                if !node.is_definition() {
                    return false;
                } else if !self.symbol_stack_postcondition.can_match_empty() {
                    return false;
                }
            }
            _ => return false,
        }

        true
    }

    /// Returns whether a partial path is "productive" — that is, whether it adds useful
    /// information to a path.  Non-productive paths are ignored.
    pub fn is_productive(&self, partials: &mut PartialPaths) -> bool {
        // StackGraph ensures that there are no nodes with duplicate IDs, so we can do a simple
        // comparison of node handles here.
        if self.start_node != self.end_node {
            return true;
        }
        if !self
            .symbol_stack_precondition
            .matches(partials, self.symbol_stack_postcondition)
        {
            return true;
        }
        if !self
            .scope_stack_precondition
            .matches(partials, self.scope_stack_postcondition)
        {
            return true;
        }
        false
    }

    /// Ensures that the content of this partial path is available in both forwards and backwards
    /// directions.
    pub fn ensure_both_directions(&mut self, partials: &mut PartialPaths) {
        self.symbol_stack_precondition
            .ensure_both_directions(partials);
        self.symbol_stack_postcondition
            .ensure_both_directions(partials);
        self.scope_stack_precondition
            .ensure_both_directions(partials);
        self.scope_stack_postcondition
            .ensure_both_directions(partials);
        self.edges.ensure_both_directions(partials);
    }

    /// Returns a fresh scope stack variable that is not already used anywhere in this partial
    /// path.
    pub fn fresh_scope_stack_variable(&self, partials: &mut PartialPaths) -> ScopeStackVariable {
        // We don't have to check the postconditions, because it's not valid for a postcondition to
        // refer to a variable that doesn't exist in the precondition.
        let symbol_stack_precondition_variables = self
            .symbol_stack_precondition
            .iter_unordered(partials)
            .filter_map(|symbol| symbol.scopes.into_option())
            .filter_map(|scopes| scopes.variable.into_option())
            .map(ScopeStackVariable::as_u32);
        let scope_stack_precondition_variables = self
            .scope_stack_precondition
            .variable
            .into_option()
            .map(ScopeStackVariable::as_u32);
        let max_used_variable = std::iter::empty()
            .chain(symbol_stack_precondition_variables)
            .chain(scope_stack_precondition_variables)
            .max()
            .unwrap_or(0);
        ScopeStackVariable::fresher_than(max_used_variable)
    }

    pub fn display<'a>(
        &'a self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }
}

impl<'a> DisplayWithPartialPaths for &'a PartialPath {
    fn prepare(&mut self, graph: &StackGraph, partials: &mut PartialPaths) {
        self.symbol_stack_precondition
            .clone()
            .prepare(graph, partials);
        self.symbol_stack_postcondition
            .clone()
            .prepare(graph, partials);
        self.scope_stack_precondition
            .clone()
            .prepare(graph, partials);
        self.scope_stack_postcondition
            .clone()
            .prepare(graph, partials);
    }

    fn display_with(
        &self,
        graph: &StackGraph,
        partials: &PartialPaths,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(
            f,
            "<{}> ({}) {} -> {} <{}> ({})",
            display_prepared(self.symbol_stack_precondition, graph, partials),
            display_prepared(self.scope_stack_precondition, graph, partials),
            self.start_node.display(graph),
            self.end_node.display(graph),
            display_prepared(self.symbol_stack_postcondition, graph, partials),
            display_prepared(self.scope_stack_postcondition, graph, partials),
        )
    }
}

impl PartialPath {
    /// Attempts to append an edge to the end of a partial path.  If the edge is not a valid
    /// extension of this partial path, we return an error describing why.
    pub fn append(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        edge: Edge,
    ) -> Result<(), PathResolutionError> {
        if edge.source != self.end_node {
            return Err(PathResolutionError::IncorrectSourceNode);
        }

        let sink = &graph[edge.sink];
        if let Node::PushSymbol(sink) = sink {
            // The symbol stack postcondition is our representation of the path's symbol stack.
            // Pushing the symbol onto our postcondition indicates that using this partial path
            // would push the symbol onto the path's symbol stack.
            let sink_symbol = sink.symbol;
            let postcondition_symbol = PartialScopedSymbol {
                symbol: sink_symbol,
                scopes: ControlledOption::none(),
            };
            self.symbol_stack_postcondition
                .push_front(partials, postcondition_symbol);
        } else if let Node::PushScopedSymbol(sink) = sink {
            // The symbol stack postcondition is our representation of the path's symbol stack.
            // Pushing the scoped symbol onto our postcondition indicates that using this partial
            // path would push the scoped symbol onto the path's symbol stack.
            let sink_symbol = sink.symbol;
            let sink_scope = graph
                .node_for_id(sink.scope)
                .ok_or(PathResolutionError::UnknownAttachedScope)?;
            let mut attached_scopes = self.scope_stack_postcondition;
            attached_scopes.push_front(partials, sink_scope);
            let postcondition_symbol = PartialScopedSymbol {
                symbol: sink_symbol,
                scopes: ControlledOption::some(attached_scopes),
            };
            self.symbol_stack_postcondition
                .push_front(partials, postcondition_symbol);
        } else if let Node::PopSymbol(sink) = sink {
            // Ideally we want to pop sink's symbol off from top of the symbol stack postcondition.
            if let Some(top) = self.symbol_stack_postcondition.pop_front(partials) {
                if top.symbol != sink.symbol {
                    return Err(PathResolutionError::IncorrectPoppedSymbol);
                }
                if top.scopes.is_some() {
                    return Err(PathResolutionError::UnexpectedAttachedScopeList);
                }
            } else {
                // If the symbol stack postcondition is empty, then we need to update the
                // _precondition_ to indicate that the symbol stack needs to contain this symbol in
                // order to successfully use this partial path.
                let precondition_symbol = PartialScopedSymbol {
                    symbol: sink.symbol,
                    scopes: ControlledOption::none(),
                };
                self.symbol_stack_precondition
                    .push_back(partials, precondition_symbol);
            }
        } else if let Node::PopScopedSymbol(sink) = sink {
            // Ideally we want to pop sink's scoped symbol off from top of the symbol stack
            // postcondition.
            if let Some(top) = self.symbol_stack_postcondition.pop_front(partials) {
                if top.symbol != sink.symbol {
                    return Err(PathResolutionError::IncorrectPoppedSymbol);
                }
                let new_scope_stack = match top.scopes.into_option() {
                    Some(scopes) => scopes,
                    None => return Err(PathResolutionError::MissingAttachedScopeList),
                };
                self.scope_stack_postcondition = new_scope_stack;
            } else {
                // If the symbol stack postcondition is empty, then we need to update the
                // _precondition_ to indicate that the symbol stack needs to contain this scoped
                // symbol in order to successfully use this partial path.
                let scope_stack_variable = self.fresh_scope_stack_variable(partials);
                let precondition_symbol = PartialScopedSymbol {
                    symbol: sink.symbol,
                    scopes: ControlledOption::some(PartialScopeStack::from_variable(
                        scope_stack_variable,
                    )),
                };
                self.symbol_stack_precondition
                    .push_back(partials, precondition_symbol);
                self.scope_stack_postcondition =
                    PartialScopeStack::from_variable(scope_stack_variable);
            }
        } else if let Node::DropScopes(_) = sink {
            self.scope_stack_postcondition = PartialScopeStack::empty();
        }

        self.end_node = edge.sink;
        self.edges.push_back(
            partials,
            PartialPathEdge {
                source_node_id: graph[edge.source].id(),
                precedence: edge.precedence,
            },
        );
        Ok(())
    }

    /// Attempts to resolve any _jump to scope_ node at the end of a partial path.  If the partial
    /// path does not end in a _jump to scope_ node, we do nothing.  If it does, and we cannot
    /// resolve it, then we return an error describing why.
    pub fn resolve(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<(), PathResolutionError> {
        if !graph[self.end_node].is_jump_to() {
            return Ok(());
        }
        if self.scope_stack_postcondition.can_only_match_empty() {
            return Err(PathResolutionError::EmptyScopeStack);
        }
        if !self.scope_stack_postcondition.contains_scopes() {
            return Ok(());
        }
        let top_scope = self.scope_stack_postcondition.pop_front(partials).unwrap();
        self.edges.push_back(
            partials,
            PartialPathEdge {
                source_node_id: graph[self.end_node].id(),
                precedence: 0,
            },
        );
        self.end_node = top_scope;
        Ok(())
    }

    /// Attempts to extend one partial path as part of the partial-path-finding algorithm, using
    /// only outgoing edges that belong to a particular file.  When calling this function, you are
    /// responsible for ensuring that `graph` already contains data for all of the possible edges
    /// that we might want to extend `path` with.
    ///
    /// The resulting extended partial paths will be added to `result`.  We have you pass that in
    /// as a parameter, instead of building it up ourselves, so that you have control over which
    /// particular collection type to use, and so that you can reuse result collections across
    /// multiple calls.
    pub fn extend_from_file<R: Extend<PartialPath>>(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        file: Handle<File>,
        result: &mut R,
    ) {
        let extensions = graph.outgoing_edges(self.end_node);
        result.reserve(extensions.size_hint().0);
        for extension in extensions {
            if !graph[extension.sink].is_in_file(file) {
                continue;
            }
            let mut new_path = self.clone();
            // If there are errors adding this edge to the partial path, or resolving the resulting
            // partial path, just skip the edge — it's not a fatal error.
            if new_path.append(graph, partials, extension).is_err() {
                continue;
            }
            if new_path.resolve(graph, partials).is_err() {
                continue;
            }
            result.push(new_path);
        }
    }
}

impl PartialPaths {
    /// Finds all partial paths in a file, calling the `visit` closure for each one.
    ///
    /// This function will not return until all reachable partial paths have been processed, so
    /// `graph` must already contain a complete stack graph.  If you have a very large stack graph
    /// stored in some other storage system, and want more control over lazily loading only the
    /// necessary pieces, then you should code up your own loop that calls
    /// [`PartialPath::extend`][] manually.
    ///
    /// [`PartialPath::extend`]: struct.PartialPath.html#method.extend
    pub fn find_all_partial_paths_in_file<F>(
        &mut self,
        graph: &StackGraph,
        file: Handle<File>,
        mut visit: F,
    ) where
        F: FnMut(&StackGraph, &mut PartialPaths, PartialPath),
    {
        let mut cycle_detector = CycleDetector::new();
        let mut queue = VecDeque::new();
        queue.push_back(PartialPath::from_node(graph, self, graph.root_node()).unwrap());
        queue.extend(
            graph
                .nodes_for_file(file)
                .filter(|node| match graph[*node] {
                    Node::PushScopedSymbol(_) => true,
                    Node::PushSymbol(_) => true,
                    Node::ExportedScope(_) => true,
                    _ => false,
                })
                .map(|node| PartialPath::from_node(graph, self, node).unwrap()),
        );
        while let Some(path) = queue.pop_front() {
            if !cycle_detector.should_process_path(&path, |probe| probe.cmp(graph, self, &path)) {
                continue;
            }
            path.extend_from_file(graph, self, file, &mut queue);
            visit(graph, self, path);
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Extending paths with partial paths

impl Path {
    /// Promotes a partial path to a path.
    pub fn from_partial_path(
        graph: &StackGraph,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        partial_path: &PartialPath,
    ) -> Option<Path> {
        let mut path = Path {
            start_node: partial_path.start_node,
            end_node: partial_path.start_node,
            symbol_stack: SymbolStack::empty(),
            scope_stack: ScopeStack::empty(),
            edges: PathEdgeList::empty(),
        };
        path.append_partial_path(graph, paths, partials, partial_path)
            .ok()?;
        Some(path)
    }

    /// Attempts to append a partial path to the end of a path.  If the partial path is not
    /// compatible with this path, we return an error describing why.
    pub fn append_partial_path(
        &mut self,
        graph: &StackGraph,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        partial_path: &PartialPath,
    ) -> Result<(), PathResolutionError> {
        if partial_path.start_node != self.end_node {
            return Err(PathResolutionError::IncorrectSourceNode);
        }

        let mut symbol_bindings = SymbolStackBindings::new();
        let mut scope_bindings = ScopeStackBindings::new();
        partial_path
            .scope_stack_precondition
            .match_stack(self.scope_stack, &mut scope_bindings)?;
        partial_path.symbol_stack_precondition.match_stack(
            graph,
            paths,
            partials,
            self.symbol_stack,
            &mut symbol_bindings,
            &mut scope_bindings,
        )?;

        self.symbol_stack = partial_path.symbol_stack_postcondition.apply_bindings(
            paths,
            partials,
            &symbol_bindings,
            &scope_bindings,
        )?;
        self.scope_stack = partial_path.scope_stack_postcondition.apply_bindings(
            paths,
            partials,
            &scope_bindings,
        )?;

        let mut edges = partial_path.edges;
        while let Some(edge) = edges.pop_front(partials) {
            self.edges.push_back(paths, edge.into());
        }
        self.end_node = partial_path.end_node;
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Partial path resolution state

/// Manages the state of a collection of partial paths built up as part of the partial-path-finding
/// algorithm or path-stitching algorithm.
pub struct PartialPaths {
    pub(crate) partial_symbol_stacks: DequeArena<PartialScopedSymbol>,
    pub(crate) partial_scope_stacks: DequeArena<Handle<Node>>,
    pub(crate) partial_path_edges: DequeArena<PartialPathEdge>,
}

impl PartialPaths {
    pub fn new() -> PartialPaths {
        PartialPaths {
            partial_symbol_stacks: Deque::new_arena(),
            partial_scope_stacks: Deque::new_arena(),
            partial_path_edges: Deque::new_arena(),
        }
    }
}
