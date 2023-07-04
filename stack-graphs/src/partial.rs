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

use std::convert::TryFrom;
use std::fmt::Display;
use std::num::NonZeroU32;

use controlled_option::ControlledOption;
use controlled_option::Niche;
use enumset::EnumSetType;
use smallvec::SmallVec;

use crate::arena::Deque;
use crate::arena::DequeArena;
use crate::arena::Handle;
use crate::graph::Edge;
use crate::graph::Node;
use crate::graph::NodeID;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::paths::PathResolutionError;
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
    pub(crate) fn initial() -> SymbolStackVariable {
        SymbolStackVariable(unsafe { NonZeroU32::new_unchecked(1) })
    }

    /// Applies an offset to this variable.
    ///
    /// When concatenating partial paths, we have to ensure that the left- and right-hand sides
    /// have non-overlapping sets of variables.  To do this, we find the maximum value of any
    /// variable on the left-hand side, and add this “offset” to the values of all of the variables
    /// on the right-hand side.
    pub fn with_offset(self, symbol_variable_offset: u32) -> SymbolStackVariable {
        let offset_value = self.0.get() + symbol_variable_offset;
        SymbolStackVariable(unsafe { NonZeroU32::new_unchecked(offset_value) })
    }

    pub(crate) fn as_u32(self) -> u32 {
        self.0.get()
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

impl From<NonZeroU32> for SymbolStackVariable {
    fn from(value: NonZeroU32) -> SymbolStackVariable {
        SymbolStackVariable(value)
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

    /// Applies an offset to this variable.
    ///
    /// When concatenating partial paths, we have to ensure that the left- and right-hand sides
    /// have non-overlapping sets of variables.  To do this, we find the maximum value of any
    /// variable on the left-hand side, and add this “offset” to the values of all of the variables
    /// on the right-hand side.
    pub fn with_offset(self, scope_variable_offset: u32) -> ScopeStackVariable {
        let offset_value = self.0.get() + scope_variable_offset;
        ScopeStackVariable(unsafe { NonZeroU32::new_unchecked(offset_value) })
    }

    pub(crate) fn as_u32(self) -> u32 {
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

impl From<NonZeroU32> for ScopeStackVariable {
    fn from(value: NonZeroU32) -> ScopeStackVariable {
        ScopeStackVariable(value)
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
    /// Applies an offset to this scoped symbol.
    ///
    /// When concatenating partial paths, we have to ensure that the left- and right-hand sides
    /// have non-overlapping sets of variables.  To do this, we find the maximum value of any
    /// variable on the left-hand side, and add this “offset” to the values of all of the variables
    /// on the right-hand side.
    pub fn with_offset(mut self, scope_variable_offset: u32) -> PartialScopedSymbol {
        let scopes = self
            .scopes
            .into_option()
            .map(|stack| stack.with_offset(scope_variable_offset));
        self.scopes = ControlledOption::from_option(scopes);
        self
    }

    /// Matches this precondition symbol against another, unifying its contents with an existing
    /// set of bindings.
    pub fn unify(
        &mut self,
        partials: &mut PartialPaths,
        rhs: PartialScopedSymbol,
        scope_bindings: &mut PartialScopeStackBindings,
    ) -> Result<(), PathResolutionError> {
        if self.symbol != rhs.symbol {
            return Err(PathResolutionError::SymbolStackUnsatisfied);
        }
        match (self.scopes.into_option(), rhs.scopes.into_option()) {
            (Some(lhs), Some(rhs)) => {
                let unified = lhs.unify(partials, rhs, scope_bindings)?;
                self.scopes = ControlledOption::some(unified);
            }
            (None, None) => {}
            _ => return Err(PathResolutionError::SymbolStackUnsatisfied),
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
    pub fn apply_partial_bindings(
        mut self,
        partials: &mut PartialPaths,
        scope_bindings: &PartialScopeStackBindings,
    ) -> Result<PartialScopedSymbol, PathResolutionError> {
        let scopes = match self.scopes.into_option() {
            Some(scopes) => Some(scopes.apply_partial_bindings(partials, scope_bindings)?),
            None => None,
        };
        self.scopes = scopes.into();
        Ok(self)
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
    length: u32,
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

    /// Returns whether this partial symbol stack has a symbol stack variable.
    #[inline(always)]
    pub fn has_variable(&self) -> bool {
        self.variable.is_some()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Returns an empty partial symbol stack.
    pub fn empty() -> PartialSymbolStack {
        PartialSymbolStack {
            symbols: Deque::empty(),
            length: 0,
            variable: ControlledOption::none(),
        }
    }

    /// Returns a partial symbol stack containing only a symbol stack variable.
    pub fn from_variable(variable: SymbolStackVariable) -> PartialSymbolStack {
        PartialSymbolStack {
            symbols: Deque::empty(),
            length: 0,
            variable: ControlledOption::some(variable),
        }
    }

    /// Returns whether this partial symbol stack is iterable in both directions without needing
    /// mutable access to the arena.
    pub fn have_reversal(&self, partials: &PartialPaths) -> bool {
        self.symbols.have_reversal(&partials.partial_symbol_stacks)
    }

    /// Applies an offset to this partial symbol stack.
    ///
    /// When concatenating partial paths, we have to ensure that the left- and right-hand sides
    /// have non-overlapping sets of variables.  To do this, we find the maximum value of any
    /// variable on the left-hand side, and add this “offset” to the values of all of the variables
    /// on the right-hand side.
    pub fn with_offset(
        mut self,
        partials: &mut PartialPaths,
        symbol_variable_offset: u32,
        scope_variable_offset: u32,
    ) -> PartialSymbolStack {
        let mut result = match self.variable.into_option() {
            Some(variable) => Self::from_variable(variable.with_offset(symbol_variable_offset)),
            None => Self::empty(),
        };
        while let Some(symbol) = self.pop_front(partials) {
            result.push_back(partials, symbol.with_offset(scope_variable_offset));
        }
        result
    }

    fn prepend(&mut self, partials: &mut PartialPaths, mut head: Deque<PartialScopedSymbol>) {
        while let Some(head) = head.pop_back(&mut partials.partial_symbol_stacks).copied() {
            self.push_front(partials, head);
        }
    }

    /// Pushes a new [`PartialScopedSymbol`][] onto the front of this partial symbol stack.
    pub fn push_front(&mut self, partials: &mut PartialPaths, symbol: PartialScopedSymbol) {
        self.length += 1;
        self.symbols
            .push_front(&mut partials.partial_symbol_stacks, symbol);
    }

    /// Pushes a new [`PartialScopedSymbol`][] onto the back of this partial symbol stack.
    pub fn push_back(&mut self, partials: &mut PartialPaths, symbol: PartialScopedSymbol) {
        self.length += 1;
        self.symbols
            .push_back(&mut partials.partial_symbol_stacks, symbol);
    }

    /// Removes and returns the [`PartialScopedSymbol`][] at the front of this partial symbol
    /// stack.  If the stack is empty, returns `None`.
    pub fn pop_front(&mut self, partials: &mut PartialPaths) -> Option<PartialScopedSymbol> {
        let result = self
            .symbols
            .pop_front(&mut partials.partial_symbol_stacks)
            .copied();
        if result.is_some() {
            self.length -= 1;
        }
        result
    }

    /// Removes and returns the [`PartialScopedSymbol`][] at the back of this partial symbol stack.
    /// If the stack is empty, returns `None`.
    pub fn pop_back(&mut self, partials: &mut PartialPaths) -> Option<PartialScopedSymbol> {
        let result = self
            .symbols
            .pop_back(&mut partials.partial_symbol_stacks)
            .copied();
        if result.is_some() {
            self.length -= 1;
        }
        result
    }

    pub fn display<'a>(
        self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
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

    /// Applies a set of bindings to this partial symbol stack, producing a new partial symbol
    /// stack.
    pub fn apply_partial_bindings(
        mut self,
        partials: &mut PartialPaths,
        symbol_bindings: &PartialSymbolStackBindings,
        scope_bindings: &PartialScopeStackBindings,
    ) -> Result<PartialSymbolStack, PathResolutionError> {
        // If this partial symbol stack ends in a variable, see if we have a binding for it.  If
        // so, substitute that binding in.  If not, leave the variable as-is.
        let mut result = match self.variable.into_option() {
            Some(variable) => match symbol_bindings.get(variable) {
                Some(bound) => bound,
                None => PartialSymbolStack::from_variable(variable),
            },
            None => PartialSymbolStack::empty(),
        };

        // Then prepend all of the scoped symbols that appear at the beginning of this stack,
        // applying the bindings to any attached scopes as well.
        while let Some(partial_symbol) = self.pop_back(partials) {
            let partial_symbol = partial_symbol.apply_partial_bindings(partials, scope_bindings)?;
            result.push_front(partials, partial_symbol);
        }
        Ok(result)
    }

    /// Given two partial symbol stacks, returns the largest possible partial symbol stack such that
    /// any symbol stack that satisfies the result also satisfies both inputs.  This takes into
    /// account any existing variable assignments, and updates those variable assignments with
    /// whatever constraints are necessary to produce a correct result.
    ///
    /// Note that this operation is commutative.  (Concatenating partial paths, defined in
    /// [`PartialPath::concatenate`][], is not.)
    pub fn unify(
        self,
        partials: &mut PartialPaths,
        mut rhs: PartialSymbolStack,
        symbol_bindings: &mut PartialSymbolStackBindings,
        scope_bindings: &mut PartialScopeStackBindings,
    ) -> Result<PartialSymbolStack, PathResolutionError> {
        let mut lhs = self;

        // First, look at the shortest common prefix of lhs and rhs, and verify that they match.
        let mut head = Deque::empty();
        while lhs.contains_symbols() && rhs.contains_symbols() {
            let mut lhs_front = lhs.pop_front(partials).unwrap();
            let rhs_front = rhs.pop_front(partials).unwrap();
            lhs_front.unify(partials, rhs_front, scope_bindings)?;
            head.push_back(&mut partials.partial_symbol_stacks, lhs_front);
        }

        // Now at most one stack still has symbols.  Zero, one, or both of them have variables.
        // Let's do a case analysis on all of those possibilities.

        // CASE 1:
        // Both lhs and rhs have no more symbols.  The answer is always yes, and any variables that
        // are present get bound.  (If both sides have variables, then one variable gets bound to
        // the other, since both lhs and rhs will match _any other symbol stack_ at this point.  If
        // only one side has a variable, then the variable gets bound to the empty stack.)
        //
        //     lhs           rhs
        // ============  ============
        //  ()            ()            => yes either
        //  ()            () $2         => yes rhs, $2 => ()
        //  () $1         ()            => yes lhs, $1 => ()
        //  () $1         () $2         => yes lhs, $2 => $1
        if !lhs.contains_symbols() && !rhs.contains_symbols() {
            match (lhs.variable.into_option(), rhs.variable.into_option()) {
                (None, None) => {
                    lhs.prepend(partials, head);
                    return Ok(lhs);
                }
                (None, Some(var)) => {
                    symbol_bindings.add(
                        partials,
                        var,
                        PartialSymbolStack::empty(),
                        scope_bindings,
                    )?;
                    rhs.prepend(partials, head);
                    return Ok(rhs);
                }
                (Some(var), None) => {
                    symbol_bindings.add(
                        partials,
                        var,
                        PartialSymbolStack::empty(),
                        scope_bindings,
                    )?;
                    lhs.prepend(partials, head);
                    return Ok(lhs);
                }
                (Some(lhs_var), Some(rhs_var)) => {
                    symbol_bindings.add(
                        partials,
                        rhs_var,
                        PartialSymbolStack::from_variable(lhs_var),
                        scope_bindings,
                    )?;
                    lhs.prepend(partials, head);
                    return Ok(lhs);
                }
            }
        }

        // CASE 2:
        // One of the stacks contains symbols and the other doesn't, and the “empty” stack doesn't
        // have a variable.  Since there's no variable on the empty side to capture the remaining
        // content on the non-empty side, the answer is always no.
        //
        //     lhs           rhs
        // ============  ============
        //  ()            (stuff)       => NO
        //  ()            (stuff) $2    => NO
        //  (stuff)       ()            => NO
        //  (stuff) $1    ()            => NO
        if !lhs.contains_symbols() && lhs.variable.is_none() {
            return Err(PathResolutionError::SymbolStackUnsatisfied);
        }
        if !rhs.contains_symbols() && rhs.variable.is_none() {
            return Err(PathResolutionError::SymbolStackUnsatisfied);
        }

        // CASE 3:
        // One of the stacks contains symbols and the other doesn't, and the “empty” stack _does_
        // have a variable.  If both sides have the same variable, the answer is NO. Otherwise,
        // the answer is YES, and the “empty” side's variable needs to capture the entirety of the
        // non-empty side.
        //
        //     lhs           rhs
        // ============  ============
        //  (...) $1      (...) $1      => no
        //  () $1         (stuff)       => yes rhs,  $1 => rhs
        //  () $1         (stuff) $2    => yes rhs,  $1 => rhs
        //  (stuff)       () $2         => yes lhs,  $2 => lhs
        //  (stuff) $1    () $2         => yes lhs,  $2 => lhs
        match (lhs.variable.into_option(), rhs.variable.into_option()) {
            (Some(v1), Some(v2)) if v1 == v2 => {
                return Err(PathResolutionError::ScopeStackUnsatisfied)
            }
            _ => {}
        }
        if lhs.contains_symbols() {
            let rhs_variable = rhs.variable.into_option().unwrap();
            symbol_bindings.add(partials, rhs_variable, lhs, scope_bindings)?;
            lhs.prepend(partials, head);
            return Ok(lhs);
        }
        if rhs.contains_symbols() {
            let lhs_variable = lhs.variable.into_option().unwrap();
            symbol_bindings.add(partials, lhs_variable, rhs, scope_bindings)?;
            rhs.prepend(partials, head);
            return Ok(rhs);
        }

        unreachable!();
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

    pub fn variable(&self) -> Option<SymbolStackVariable> {
        self.variable.clone().into_option()
    }

    fn ensure_both_directions(&mut self, partials: &mut PartialPaths) {
        self.symbols
            .ensure_backwards(&mut partials.partial_symbol_stacks);
        self.symbols
            .ensure_forwards(&mut partials.partial_symbol_stacks);
    }

    fn ensure_forwards(&mut self, partials: &mut PartialPaths) {
        self.symbols
            .ensure_forwards(&mut partials.partial_symbol_stacks);
    }

    /// Returns the largest value of any symbol stack variable in this partial symbol stack.
    pub fn largest_symbol_stack_variable(&self) -> u32 {
        self.variable
            .into_option()
            .map(SymbolStackVariable::as_u32)
            .unwrap_or(0)
    }

    /// Returns the largest value of any scope stack variable in this partial symbol stack.
    pub fn largest_scope_stack_variable(&self, partials: &PartialPaths) -> u32 {
        // We don't have to check the postconditions, because it's not valid for a postcondition to
        // refer to a variable that doesn't exist in the precondition.
        self.iter_unordered(partials)
            .filter_map(|symbol| symbol.scopes.into_option())
            .filter_map(|scopes| scopes.variable.into_option())
            .map(ScopeStackVariable::as_u32)
            .max()
            .unwrap_or(0)
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
    length: u32,
    variable: ControlledOption<ScopeStackVariable>,
}

impl PartialScopeStack {
    /// Returns whether this partial scope stack can match the empty scope stack.
    #[inline(always)]
    pub fn can_match_empty(&self) -> bool {
        self.scopes.is_empty()
    }

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

    /// Returns whether this partial scope stack has a scope stack variable.
    #[inline(always)]
    pub fn has_variable(&self) -> bool {
        self.variable.is_some()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Returns an empty partial scope stack.
    pub fn empty() -> PartialScopeStack {
        PartialScopeStack {
            scopes: Deque::empty(),
            length: 0,
            variable: ControlledOption::none(),
        }
    }

    /// Returns a partial scope stack containing only a scope stack variable.
    pub fn from_variable(variable: ScopeStackVariable) -> PartialScopeStack {
        PartialScopeStack {
            scopes: Deque::empty(),
            length: 0,
            variable: ControlledOption::some(variable),
        }
    }

    /// Returns whether this partial scope stack is iterable in both directions without needing
    /// mutable access to the arena.
    pub fn have_reversal(&self, partials: &PartialPaths) -> bool {
        self.scopes.have_reversal(&partials.partial_scope_stacks)
    }

    /// Applies an offset to this partial scope stack.
    ///
    /// When concatenating partial paths, we have to ensure that the left- and right-hand sides
    /// have non-overlapping sets of variables.  To do this, we find the maximum value of any
    /// variable on the left-hand side, and add this “offset” to the values of all of the variables
    /// on the right-hand side.
    pub fn with_offset(mut self, scope_variable_offset: u32) -> PartialScopeStack {
        match self.variable.into_option() {
            Some(variable) => {
                self.variable = ControlledOption::some(variable.with_offset(scope_variable_offset));
            }
            None => {}
        };
        self
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

    /// Applies a set of partial scope stack bindings to this partial scope stack, producing a new
    /// partial scope stack.
    pub fn apply_partial_bindings(
        mut self,
        partials: &mut PartialPaths,
        scope_bindings: &PartialScopeStackBindings,
    ) -> Result<PartialScopeStack, PathResolutionError> {
        // If this partial scope stack ends in a variable, see if we have a binding for it.  If so,
        // substitute that binding in.  If not, leave the variable as-is.
        let mut result = match self.variable.into_option() {
            Some(variable) => match scope_bindings.get(variable) {
                Some(bound) => bound,
                None => PartialScopeStack::from_variable(variable),
            },
            None => PartialScopeStack::empty(),
        };

        // Then prepend all of the scopes that appear at the beginning of this stack.
        while let Some(scope) = self.pop_back(partials) {
            result.push_front(partials, scope);
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
        // have a variable.  If both sides have the same variable, the answer is NO. Otherwise,
        // the answer is YES, and the “empty” side's variable needs to capture the entirety of the
        // non-empty side.
        //
        //     lhs           rhs
        // ============  ============
        //  (...) $1      (...) $1      => no
        //  () $1         (stuff)       => yes rhs,  $1 => rhs
        //  () $1         (stuff) $2    => yes rhs,  $1 => rhs
        //  (stuff)       () $2         => yes lhs,  $2 => lhs
        //  (stuff) $1    () $2         => yes lhs,  $2 => lhs
        match (lhs.variable.into_option(), rhs.variable.into_option()) {
            (Some(v1), Some(v2)) if v1 == v2 => {
                return Err(PathResolutionError::ScopeStackUnsatisfied)
            }
            _ => {}
        }
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
        self.length += 1;
        self.scopes
            .push_front(&mut partials.partial_scope_stacks, node);
    }

    /// Pushes a new [`Node`][] onto the back of this partial scope stack.  The node must be an
    /// _exported scope node_.
    ///
    /// [`Node`]: ../graph/enum.Node.html
    pub fn push_back(&mut self, partials: &mut PartialPaths, node: Handle<Node>) {
        self.length += 1;
        self.scopes
            .push_back(&mut partials.partial_scope_stacks, node);
    }

    /// Removes and returns the [`Node`][] at the front of this partial scope stack.  If the stack
    /// does not contain any exported scope nodes, returns `None`.
    pub fn pop_front(&mut self, partials: &mut PartialPaths) -> Option<Handle<Node>> {
        let result = self
            .scopes
            .pop_front(&mut partials.partial_scope_stacks)
            .copied();
        if result.is_some() {
            self.length -= 1;
        }
        result
    }

    /// Removes and returns the [`Node`][] at the back of this partial scope stack.  If the stack
    /// does not contain any exported scope nodes, returns `None`.
    pub fn pop_back(&mut self, partials: &mut PartialPaths) -> Option<Handle<Node>> {
        let result = self
            .scopes
            .pop_back(&mut partials.partial_scope_stacks)
            .copied();
        if result.is_some() {
            self.length -= 1;
        }
        result
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

    fn ensure_forwards(&mut self, partials: &mut PartialPaths) {
        self.scopes
            .ensure_forwards(&mut partials.partial_scope_stacks);
    }

    /// Returns the largest value of any scope stack variable in this partial scope stack.
    pub fn largest_scope_stack_variable(&self) -> u32 {
        self.variable
            .into_option()
            .map(ScopeStackVariable::as_u32)
            .unwrap_or(0)
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
    pub fn get(&self, variable: SymbolStackVariable) -> Option<PartialSymbolStack> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            return None;
        }
        self.bindings[index - 1]
    }

    /// Adds a new binding from a symbol stack variable to the partial symbol stack that it
    /// matched.  Returns an error if you try to bind a particular variable more than once.
    pub fn add(
        &mut self,
        partials: &mut PartialPaths,
        variable: SymbolStackVariable,
        mut symbols: PartialSymbolStack,
        scope_bindings: &mut PartialScopeStackBindings,
    ) -> Result<(), PathResolutionError> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            self.bindings.resize_with(index, || None);
        }
        if let Some(old_binding) = self.bindings[index - 1] {
            symbols = symbols.unify(partials, old_binding, self, scope_bindings)?;
        }
        self.bindings[index - 1] = Some(symbols);
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

impl<'a> DisplayWithPartialPaths for &'a mut PartialSymbolStackBindings {
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
                    "%{} => <{}>",
                    idx + 1,
                    display_prepared(*binding, graph, partials)
                )?;
            }
        }
        write!(f, "}}")
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
    pub fn get(&self, variable: ScopeStackVariable) -> Option<PartialScopeStack> {
        let index = variable.as_usize();
        if self.bindings.len() < index {
            return None;
        }
        self.bindings[index - 1]
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
    length: u32,
}

impl PartialPathEdgeList {
    /// Returns whether this edge list is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Returns whether this edge list is iterable in both directions without needing mutable
    /// access to the arena.
    pub fn have_reversal(&self, partials: &PartialPaths) -> bool {
        self.edges.have_reversal(&partials.partial_path_edges)
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
                if self_edge.source_node_id != other_edge.source_node_id {
                    return false;
                } else if self_edge.shadows(other_edge) {
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

    fn ensure_forwards(&mut self, partials: &mut PartialPaths) {
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
    ) -> PartialPath {
        let initial_symbol_stack = SymbolStackVariable::initial();
        let initial_scope_stack = ScopeStackVariable::initial();
        let mut symbol_stack_precondition = PartialSymbolStack::from_variable(initial_symbol_stack);
        let mut symbol_stack_postcondition =
            PartialSymbolStack::from_variable(initial_symbol_stack);
        let mut scope_stack_precondition = PartialScopeStack::from_variable(initial_scope_stack);
        let mut scope_stack_postcondition = PartialScopeStack::from_variable(initial_scope_stack);

        graph[node]
            .append_to_partial_stacks(
                graph,
                partials,
                &mut symbol_stack_precondition,
                &mut scope_stack_precondition,
                &mut symbol_stack_postcondition,
                &mut scope_stack_postcondition,
            )
            .expect("lifting single nodes to partial paths should not fail");

        PartialPath {
            start_node: node,
            end_node: node,
            symbol_stack_precondition,
            symbol_stack_postcondition,
            scope_stack_precondition,
            scope_stack_postcondition,
            edges: PartialPathEdgeList::empty(),
        }
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
    }

    /// Returns whether a partial path represents the start of a name binding from a reference to a
    /// definition.
    pub fn starts_at_reference(&self, graph: &StackGraph) -> bool {
        graph[self.start_node].is_reference()
            && self.symbol_stack_precondition.can_match_empty()
            && self.scope_stack_precondition.can_match_empty()
    }

    /// Returns whether a partial path represents the end of a name binding from a reference to a
    /// definition.
    pub fn ends_at_definition(&self, graph: &StackGraph) -> bool {
        graph[self.end_node].is_definition() && self.symbol_stack_postcondition.can_match_empty()
    }

    /// A _complete_ partial path represents a full name binding that resolves a reference to a
    /// definition.
    pub fn is_complete(&self, graph: &StackGraph) -> bool {
        self.starts_at_reference(graph) && self.ends_at_definition(graph)
    }

    pub fn starts_at_endpoint(&self, graph: &StackGraph) -> bool {
        graph[self.start_node].is_endpoint()
    }

    pub fn ends_at_endpoint(&self, graph: &StackGraph) -> bool {
        graph[self.end_node].is_endpoint()
    }

    pub fn ends_in_jump(&self, graph: &StackGraph) -> bool {
        graph[self.end_node].is_jump_to()
    }

    /// Returns whether a partial path is cyclic---that is, it starts and ends at the same node,
    /// and its postcondition is compatible with its precondition.  If the path is cyclic, a
    /// tuple is returned indicating whether cycle requires strengthening the pre- or postcondition.
    pub fn is_cyclic(&self, graph: &StackGraph, partials: &mut PartialPaths) -> Option<Cyclicity> {
        // StackGraph ensures that there are no nodes with duplicate IDs, so we can do a simple
        // comparison of node handles here.
        if self.start_node != self.end_node {
            return None;
        }

        let lhs = self;
        let mut rhs = self.clone();
        rhs.ensure_no_overlapping_variables(partials, lhs);

        let join = match Self::compute_join(graph, partials, lhs, &rhs) {
            Ok(join) => join,
            Err(_) => return None,
        };

        if lhs
            .symbol_stack_precondition
            .variable
            .into_option()
            .map_or(false, |v| join.symbol_bindings.get(v).iter().len() > 0)
            || lhs
                .scope_stack_precondition
                .variable
                .into_option()
                .map_or(false, |v| join.scope_bindings.get(v).iter().len() > 0)
        {
            Some(Cyclicity::StrengthensPrecondition)
        } else if rhs
            .symbol_stack_postcondition
            .variable
            .into_option()
            .map_or(false, |v| join.symbol_bindings.get(v).iter().len() > 0)
            || rhs
                .scope_stack_postcondition
                .variable
                .into_option()
                .map_or(false, |v| join.scope_bindings.get(v).iter().len() > 0)
        {
            Some(Cyclicity::StrengthensPostcondition)
        } else {
            Some(Cyclicity::Free)
        }
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

        let mut stack = self.symbol_stack_precondition;
        while let Some(symbol) = stack.pop_front(partials) {
            if let Some(mut scopes) = symbol.scopes.into_option() {
                scopes.ensure_both_directions(partials);
            }
        }

        let mut stack = self.symbol_stack_postcondition;
        while let Some(symbol) = stack.pop_front(partials) {
            if let Some(mut scopes) = symbol.scopes.into_option() {
                scopes.ensure_both_directions(partials);
            }
        }
    }

    /// Ensures that the content of this partial path is in forwards direction.
    pub fn ensure_forwards(&mut self, partials: &mut PartialPaths) {
        self.symbol_stack_precondition.ensure_forwards(partials);
        self.symbol_stack_postcondition.ensure_forwards(partials);
        self.scope_stack_precondition.ensure_forwards(partials);
        self.scope_stack_postcondition.ensure_forwards(partials);
        self.edges.ensure_forwards(partials);

        let mut stack = self.symbol_stack_precondition;
        while let Some(symbol) = stack.pop_front(partials) {
            if let Some(mut scopes) = symbol.scopes.into_option() {
                scopes.ensure_forwards(partials);
            }
        }

        let mut stack = self.symbol_stack_postcondition;
        while let Some(symbol) = stack.pop_front(partials) {
            if let Some(mut scopes) = symbol.scopes.into_option() {
                scopes.ensure_forwards(partials);
            }
        }
    }

    /// Returns the largest value of any symbol stack variable in this partial path.
    pub fn largest_symbol_stack_variable(&self) -> u32 {
        // We don't have to check the postconditions, because it's not valid for a postcondition to
        // refer to a variable that doesn't exist in the precondition.
        self.symbol_stack_precondition
            .largest_symbol_stack_variable()
    }

    /// Returns the largest value of any scope stack variable in this partial path.
    pub fn largest_scope_stack_variable(&self, partials: &PartialPaths) -> u32 {
        Self::largest_scope_stack_variable_for_partial_stacks(
            partials,
            &self.symbol_stack_precondition,
            &self.scope_stack_precondition,
        )
    }

    fn largest_scope_stack_variable_for_partial_stacks(
        partials: &PartialPaths,
        symbol_stack_precondition: &PartialSymbolStack,
        scope_stack_precondition: &PartialScopeStack,
    ) -> u32 {
        // We don't have to check the postconditions, because it's not valid for a postcondition to
        // refer to a variable that doesn't exist in the precondition.
        std::cmp::max(
            symbol_stack_precondition.largest_scope_stack_variable(partials),
            scope_stack_precondition.largest_scope_stack_variable(),
        )
    }

    /// Returns a fresh scope stack variable that is not already used anywhere in this partial
    /// path.
    pub fn fresh_scope_stack_variable(&self, partials: &PartialPaths) -> ScopeStackVariable {
        Self::fresh_scope_stack_variable_for_partial_stack(
            partials,
            &self.symbol_stack_precondition,
            &self.scope_stack_precondition,
        )
    }

    fn fresh_scope_stack_variable_for_partial_stack(
        partials: &PartialPaths,
        symbol_stack_precondition: &PartialSymbolStack,
        scope_stack_precondition: &PartialScopeStack,
    ) -> ScopeStackVariable {
        ScopeStackVariable::fresher_than(Self::largest_scope_stack_variable_for_partial_stacks(
            partials,
            symbol_stack_precondition,
            scope_stack_precondition,
        ))
    }

    pub fn display<'a>(
        &'a self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
    ) -> impl Display + 'a {
        display_with(self, graph, partials)
    }
}

#[derive(Debug, EnumSetType)]
pub enum Cyclicity {
    /// The path can be freely concatenated to itself.
    Free,
    /// Concatenating the path to itself strengthens the precondition---symbols are eliminated from the stack.
    StrengthensPrecondition,
    /// Concatenating the path to itself strengthens the postcondition---symbols are introduced on the stack.
    StrengthensPostcondition,
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
    /// Modifies this partial path so that it has no symbol or scope stack variables in common with
    /// another partial path.
    pub fn ensure_no_overlapping_variables(
        &mut self,
        partials: &mut PartialPaths,
        other: &PartialPath,
    ) {
        let symbol_variable_offset = other.largest_symbol_stack_variable();
        let scope_variable_offset = other.largest_scope_stack_variable(partials);
        self.symbol_stack_precondition = self.symbol_stack_precondition.with_offset(
            partials,
            symbol_variable_offset,
            scope_variable_offset,
        );
        self.symbol_stack_postcondition = self.symbol_stack_postcondition.with_offset(
            partials,
            symbol_variable_offset,
            scope_variable_offset,
        );
        self.scope_stack_precondition = self
            .scope_stack_precondition
            .with_offset(scope_variable_offset);
        self.scope_stack_postcondition = self
            .scope_stack_postcondition
            .with_offset(scope_variable_offset);
    }

    /// Replaces stack variables in the precondition with empty stacks.
    pub fn eliminate_precondition_stack_variables(&mut self, partials: &mut PartialPaths) {
        let mut symbol_bindings = PartialSymbolStackBindings::new();
        let mut scope_bindings = PartialScopeStackBindings::new();
        if let Some(symbol_variable) = self.symbol_stack_precondition.variable() {
            symbol_bindings
                .add(
                    partials,
                    symbol_variable,
                    PartialSymbolStack::empty(),
                    &mut scope_bindings,
                )
                .unwrap();
        }
        if let Some(scope_variable) = self.scope_stack_precondition.variable() {
            scope_bindings
                .add(partials, scope_variable, PartialScopeStack::empty())
                .unwrap();
        }

        self.symbol_stack_precondition = self
            .symbol_stack_precondition
            .apply_partial_bindings(partials, &symbol_bindings, &scope_bindings)
            .unwrap();
        self.scope_stack_precondition = self
            .scope_stack_precondition
            .apply_partial_bindings(partials, &scope_bindings)
            .unwrap();

        self.symbol_stack_postcondition = self
            .symbol_stack_postcondition
            .apply_partial_bindings(partials, &symbol_bindings, &scope_bindings)
            .unwrap();
        self.scope_stack_postcondition = self
            .scope_stack_postcondition
            .apply_partial_bindings(partials, &scope_bindings)
            .unwrap();
    }

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

        graph[edge.sink].append_to_partial_stacks(
            graph,
            partials,
            &mut self.symbol_stack_precondition,
            &mut self.scope_stack_precondition,
            &mut self.symbol_stack_postcondition,
            &mut self.scope_stack_postcondition,
        )?;

        self.end_node = edge.sink;
        self.edges.push_back(
            partials,
            PartialPathEdge {
                source_node_id: graph[edge.source].id(),
                precedence: edge.precedence,
            },
        );

        self.resolve_from_postcondition(graph, partials)?;

        Ok(())
    }

    /// Attempts to resolve any _jump to scope_ node at the end of a partial path from the postcondition
    /// scope stack.  If the partial path does not end in a _jump to scope_ node, we do nothing.  If it
    /// does, and we cannot resolve it, then we return an error describing why.
    pub fn resolve_from_postcondition(
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

    /// Resolve any _jump to scope_ node at the end of a partial path to the given node, updating the
    /// precondition to include the given node.  If the partial path does not end in a _jump to scope_
    /// node, we do nothing.  If it does, and we cannot resolve it, then we return an error describing
    /// why.
    pub fn resolve_to_node(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        node: Handle<Node>,
    ) -> Result<(), PathResolutionError> {
        if !graph[self.end_node].is_jump_to() {
            return Ok(());
        }

        let scope_variable = match self.scope_stack_postcondition.variable() {
            Some(scope_variable) => scope_variable,
            None => return Err(PathResolutionError::ScopeStackUnsatisfied),
        };
        let mut scope_stack = PartialScopeStack::from_variable(scope_variable);
        scope_stack.push_front(partials, node);

        let symbol_bindings = PartialSymbolStackBindings::new();
        let mut scope_bindings = PartialScopeStackBindings::new();
        scope_bindings
            .add(partials, scope_variable, scope_stack)
            .unwrap();

        self.symbol_stack_precondition = self
            .symbol_stack_precondition
            .apply_partial_bindings(partials, &symbol_bindings, &scope_bindings)
            .unwrap();
        self.scope_stack_precondition = self
            .scope_stack_precondition
            .apply_partial_bindings(partials, &scope_bindings)
            .unwrap();

        self.end_node = node;

        Ok(())
    }
}

impl Node {
    /// Update the given partial path pre- and postconditions with the effect of
    /// appending this node to that partial path.
    pub(crate) fn append_to_partial_stacks(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        symbol_stack_precondition: &mut PartialSymbolStack,
        scope_stack_precondition: &mut PartialScopeStack,
        symbol_stack_postcondition: &mut PartialSymbolStack,
        scope_stack_postcondition: &mut PartialScopeStack,
    ) -> Result<(), PathResolutionError> {
        match self {
            Self::DropScopes(_) => {
                *scope_stack_postcondition = PartialScopeStack::empty();
            }
            Self::JumpTo(_) => {}
            Self::PopScopedSymbol(sink) => {
                // Ideally we want to pop sink's scoped symbol off from top of the symbol stack
                // postcondition.
                if let Some(top) = symbol_stack_postcondition.pop_front(partials) {
                    if top.symbol != sink.symbol {
                        return Err(PathResolutionError::IncorrectPoppedSymbol);
                    }
                    let new_scope_stack = match top.scopes.into_option() {
                        Some(scopes) => scopes,
                        None => return Err(PathResolutionError::MissingAttachedScopeList),
                    };
                    *scope_stack_postcondition = new_scope_stack;
                } else if symbol_stack_postcondition.has_variable() {
                    // If the symbol stack postcondition is empty but has a variable, then we can update
                    // the _precondition_ to indicate that the symbol stack needs to contain this scoped
                    // symbol in order to successfully use this partial path.
                    let scope_stack_variable =
                        PartialPath::fresh_scope_stack_variable_for_partial_stack(
                            partials,
                            symbol_stack_precondition,
                            scope_stack_precondition,
                        );
                    let precondition_symbol = PartialScopedSymbol {
                        symbol: sink.symbol,
                        scopes: ControlledOption::some(PartialScopeStack::from_variable(
                            scope_stack_variable,
                        )),
                    };
                    // We simply push to the precondition. The official procedure here
                    // is to bind the postcondition symbol stack variable to the symbol
                    // and a fresh variable, and apply that. However, because the variable
                    // can only be bound in the precondition symbol stack, this amounts to
                    // pushing the symbol there directly.
                    symbol_stack_precondition.push_back(partials, precondition_symbol);
                    *scope_stack_postcondition =
                        PartialScopeStack::from_variable(scope_stack_variable);
                } else {
                    // The symbol stack postcondition is empty and has no variable, so we cannot
                    // perform the operation.
                    return Err(PathResolutionError::SymbolStackUnsatisfied);
                }
            }
            Self::PopSymbol(sink) => {
                // Ideally we want to pop sink's symbol off from top of the symbol stack postcondition.
                if let Some(top) = symbol_stack_postcondition.pop_front(partials) {
                    if top.symbol != sink.symbol {
                        return Err(PathResolutionError::IncorrectPoppedSymbol);
                    }
                    if top.scopes.is_some() {
                        return Err(PathResolutionError::UnexpectedAttachedScopeList);
                    }
                } else if symbol_stack_postcondition.has_variable() {
                    // If the symbol stack postcondition is empty but has a variable, then we can update
                    // the _precondition_ to indicate that the symbol stack needs to contain this symbol
                    // in order to successfully use this partial path.
                    let precondition_symbol = PartialScopedSymbol {
                        symbol: sink.symbol,
                        scopes: ControlledOption::none(),
                    };
                    // We simply push to the precondition. The official procedure here
                    // is to bind the postcondition symbol stack variable to the symbol
                    // and a fresh variable, and apply that. However, because the variable
                    // can only be bound in the precondition symbol stack, this amounts to
                    // pushing the symbol there directly.
                    symbol_stack_precondition.push_back(partials, precondition_symbol);
                } else {
                    // The symbol stack postcondition is empty and has no variable, so we cannot
                    // perform the operation.
                    return Err(PathResolutionError::SymbolStackUnsatisfied);
                }
            }
            Self::PushScopedSymbol(sink) => {
                // The symbol stack postcondition is our representation of the path's symbol stack.
                // Pushing the scoped symbol onto our postcondition indicates that using this partial
                // path would push the scoped symbol onto the path's symbol stack.
                let sink_symbol = sink.symbol;
                let sink_scope = graph
                    .node_for_id(sink.scope)
                    .ok_or(PathResolutionError::UnknownAttachedScope)?;
                let mut attached_scopes = scope_stack_postcondition.clone();
                attached_scopes.push_front(partials, sink_scope);
                let postcondition_symbol = PartialScopedSymbol {
                    symbol: sink_symbol,
                    scopes: ControlledOption::some(attached_scopes),
                };
                symbol_stack_postcondition.push_front(partials, postcondition_symbol);
            }
            Self::PushSymbol(sink) => {
                // The symbol stack postcondition is our representation of the path's symbol stack.
                // Pushing the symbol onto our postcondition indicates that using this partial path
                // would push the symbol onto the path's symbol stack.
                let sink_symbol = sink.symbol;
                let postcondition_symbol = PartialScopedSymbol {
                    symbol: sink_symbol,
                    scopes: ControlledOption::none(),
                };
                symbol_stack_postcondition.push_front(partials, postcondition_symbol);
            }
            Self::Root(_) => {}
            Self::Scope(_) => {}
        }
        Ok(())
    }

    /// Ensure the given closed precondition stacks are half-open for this end node.
    ///
    /// Partial paths have closed (cf. a closed interval) pre- and postconditions, which means
    /// the start node is reflected in the precondition, and the end node is reflected in the
    /// postcondition. For example, a path starting with a pop node, has a precondition starting
    /// with the popped symbol. Similarly, a ending with a push node, has a postcondition ending
    /// with the pushed symbol.
    ///
    /// When concatenating two partial paths, their closed pre- and postconditions are not compatible,
    /// because the effect of the join node (i.e., the node shared between the two paths) is present
    /// in both the right and the left path. If two paths join at a push node, the right postcondition
    /// contains the pushed symbol, while the left precondition does not contain it, behaving as if the
    /// symbol was pushed twice. Similarly, when joining at a pop node, the right precondition contains
    /// the popped symbol, while the right postcondition will not anymore, because it was already popped.
    /// Unifying closed pre- and postconditions can result in incorrect concatenation results.
    ///
    /// We can make pre- and postconditions compatible again by making them half-open (cf. open intervals,
    /// but half because we only undo the effect of some node types). A precondition is half-open if it
    /// does not reflect the effect if a start pop node, Similarly, a postcondition is half-open if it
    /// does not reflect the effect of an end push node. Unifying half-open pre- and postconditions results
    /// in the correct behavior for path concatenation.
    fn halfopen_closed_partial_precondition(
        &self,
        partials: &mut PartialPaths,
        symbol_stack: &mut PartialSymbolStack,
        scope_stack: &mut PartialScopeStack,
    ) -> Result<(), PathResolutionError> {
        match self {
            Node::DropScopes(_) => {}
            Node::JumpTo(_) => {}
            Node::PopScopedSymbol(node) => {
                let symbol = symbol_stack
                    .pop_front(partials)
                    .ok_or(PathResolutionError::EmptySymbolStack)?;
                if symbol.symbol != node.symbol {
                    return Err(PathResolutionError::IncorrectPoppedSymbol);
                }
                *scope_stack = symbol.scopes.into_option().unwrap();
            }
            Node::PopSymbol(node) => {
                let symbol = symbol_stack
                    .pop_front(partials)
                    .ok_or(PathResolutionError::EmptySymbolStack)?;
                if symbol.symbol != node.symbol {
                    return Err(PathResolutionError::IncorrectPoppedSymbol);
                }
            }
            Node::PushScopedSymbol(_) => {}
            Node::PushSymbol(_) => {}
            Node::Root(_) => {}
            Node::Scope(_) => {}
        };
        Ok(())
    }

    /// Ensure the given closed postcondition stacks are half-open for this start node.
    ///
    /// Partial paths have closed (cf. a closed interval) pre- and postconditions, which means
    /// the start node is reflected in the precondition, and the end node is reflected in the
    /// postcondition. For example, a path starting with a pop node, has a precondition starting
    /// with the popped symbol. Similarly, a ending with a push node, has a postcondition ending
    /// with the pushed symbol.
    ///
    /// When concatenating two partial paths, their closed pre- and postconditions are not compatible,
    /// because the effect of the join node (i.e., the node shared between the two paths) is present
    /// in both the right and the left path. If two paths join at a push node, the right postcondition
    /// contains the pushed symbol, while the left precondition does not contain it, behaving as if the
    /// symbol was pushed twice. Similarly, when joining at a pop node, the right precondition contains
    /// the popped symbol, while the right postcondition will not anymore, because it was already popped.
    /// Unifying closed pre- and postconditions can result in incorrect concatenation results.
    ///
    /// We can make pre- and postconditions compatible again by making them half-open (cf. open intervals,
    /// but half because we only undo the effect of some node types). A precondition is half-open if it
    /// does not reflect the effect if a start pop node, Similarly, a postcondition is half-open if it
    /// does not reflect the effect of an end push node. Unifying half-open pre- and postconditions results
    /// in the correct behavior for path concatenation.
    fn halfopen_closed_partial_postcondition(
        &self,
        partials: &mut PartialPaths,
        symbol_stack: &mut PartialSymbolStack,
        _scope_stack: &mut PartialScopeStack,
    ) -> Result<(), PathResolutionError> {
        match self {
            Self::DropScopes(_) => {}
            Self::JumpTo(_) => {}
            Self::PopScopedSymbol(_) => {}
            Self::PopSymbol(_) => {}
            Self::PushScopedSymbol(node) => {
                let symbol = symbol_stack
                    .pop_front(partials)
                    .ok_or(PathResolutionError::EmptySymbolStack)?;
                if symbol.symbol != node.symbol {
                    return Err(PathResolutionError::IncorrectPoppedSymbol);
                }
            }
            Self::PushSymbol(node) => {
                let symbol = symbol_stack
                    .pop_front(partials)
                    .ok_or(PathResolutionError::EmptySymbolStack)?;
                if symbol.symbol != node.symbol {
                    return Err(PathResolutionError::IncorrectPoppedSymbol);
                }
            }
            Self::Root(_) => {}
            Self::Scope(_) => {}
        };
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Extending partial paths with partial paths

impl PartialPath {
    /// Attempts to append a partial path to this one.  If the postcondition of the “left” partial path
    /// is not compatible with the precondition of the “right” path, we return an error describing why.
    ///
    /// If the left- and right-hand partial paths have any symbol or scope stack variables in
    /// common, then we ensure that the variables bind to the same values on both sides.  It's your
    /// responsibility to update the two partial paths so that they have no variables in common, if
    /// that's needed for your use case.
    #[cfg_attr(not(feature = "copious-debugging"), allow(unused_variables))]
    pub fn concatenate(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        rhs: &PartialPath,
    ) -> Result<(), PathResolutionError> {
        let lhs = self;

        #[cfg_attr(not(feature = "copious-debugging"), allow(unused_mut))]
        let mut join = Self::compute_join(graph, partials, lhs, rhs)?;
        #[cfg(feature = "copious-debugging")]
        {
            let unified_symbol_stack = join
                .unified_symbol_stack
                .display(graph, partials)
                .to_string();
            let unified_scope_stack = join
                .unified_scope_stack
                .display(graph, partials)
                .to_string();
            let symbol_bindings = join.symbol_bindings.display(graph, partials).to_string();
            let scope_bindings = join.scope_bindings.display(graph, partials).to_string();
            copious_debugging!(
                "       via <{}> ({}) {} {}",
                unified_symbol_stack,
                unified_scope_stack,
                symbol_bindings,
                scope_bindings,
            );
        }

        lhs.symbol_stack_precondition = lhs.symbol_stack_precondition.apply_partial_bindings(
            partials,
            &join.symbol_bindings,
            &join.scope_bindings,
        )?;
        lhs.symbol_stack_postcondition = rhs.symbol_stack_postcondition.apply_partial_bindings(
            partials,
            &join.symbol_bindings,
            &join.scope_bindings,
        )?;

        lhs.scope_stack_precondition = lhs
            .scope_stack_precondition
            .apply_partial_bindings(partials, &join.scope_bindings)?;
        lhs.scope_stack_postcondition = rhs
            .scope_stack_postcondition
            .apply_partial_bindings(partials, &join.scope_bindings)?;

        let mut edges = rhs.edges;
        while let Some(edge) = edges.pop_front(partials) {
            lhs.edges.push_back(partials, edge);
        }
        lhs.end_node = rhs.end_node;

        lhs.resolve_from_postcondition(graph, partials)?;

        Ok(())
    }

    /// Compute the bindings to join to partial paths. It is the caller's responsibility
    /// to ensure non-overlapping variables, if that is required.
    fn compute_join(
        graph: &StackGraph,
        partials: &mut PartialPaths,
        lhs: &PartialPath,
        rhs: &PartialPath,
    ) -> Result<Join, PathResolutionError> {
        if lhs.end_node != rhs.start_node {
            return Err(PathResolutionError::IncorrectSourceNode);
        }

        // Ensure the right post- and left precondition are half-open, so we can unify them.
        let mut lhs_symbol_stack_postcondition = lhs.symbol_stack_postcondition;
        let mut lhs_scope_stack_postcondition = lhs.scope_stack_postcondition;
        let mut rhs_symbol_stack_precondition = rhs.symbol_stack_precondition;
        let mut rhs_scope_stack_precondition = rhs.scope_stack_precondition;
        graph[lhs.end_node]
            .halfopen_closed_partial_postcondition(
                partials,
                &mut lhs_symbol_stack_postcondition,
                &mut lhs_scope_stack_postcondition,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "failed to halfopen postcondition of {}: {:?}",
                    lhs.display(graph, partials),
                    e
                );
            });
        graph[rhs.start_node]
            .halfopen_closed_partial_precondition(
                partials,
                &mut rhs_symbol_stack_precondition,
                &mut rhs_scope_stack_precondition,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "failed to halfopen postcondition of {}: {:?}",
                    rhs.display(graph, partials),
                    e
                );
            });

        let mut symbol_bindings = PartialSymbolStackBindings::new();
        let mut scope_bindings = PartialScopeStackBindings::new();
        let unified_symbol_stack = lhs_symbol_stack_postcondition.unify(
            partials,
            rhs_symbol_stack_precondition,
            &mut symbol_bindings,
            &mut scope_bindings,
        )?;
        let unified_scope_stack = lhs_scope_stack_postcondition.unify(
            partials,
            rhs_scope_stack_precondition,
            &mut scope_bindings,
        )?;

        Ok(Join {
            unified_symbol_stack,
            unified_scope_stack,
            symbol_bindings,
            scope_bindings,
        })
    }
}

struct Join {
    #[cfg_attr(not(feature = "copious-debugging"), allow(dead_code))]
    pub unified_symbol_stack: PartialSymbolStack,
    #[cfg_attr(not(feature = "copious-debugging"), allow(dead_code))]
    pub unified_scope_stack: PartialScopeStack,
    pub symbol_bindings: PartialSymbolStackBindings,
    pub scope_bindings: PartialScopeStackBindings,
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

    #[cfg_attr(not(feature = "storage"), allow(dead_code))]
    pub(crate) fn clear(&mut self) {
        self.partial_symbol_stacks.clear();
        self.partial_scope_stacks.clear();
        self.partial_path_edges.clear();
    }
}
