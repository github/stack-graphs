// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Detect and avoid cycles in our path-finding algorithm.
//!
//! Cycles in a stack graph can indicate many things.  Your language might allow mutually recursive
//! imports.  If you are modeling dataflow through function calls, then any recursion in your
//! function calls will lead to cycles in your stack graph.  And if you have any control-flow paths
//! that lead to infinite loops at runtime, we'll probably discover those as stack graph paths
//! during the path-finding algorithm.
//!
//! (Note that we're only considering cycles in well-formed paths.  For instance, _pop symbol_
//! nodes are "guards" that don't allow you to progress into a node if the top of the symbol stack
//! doesn't match.  We don't consider that a valid path, and so we don't have to worry about
//! whether it contains any cycles.)
//!
//! This module implements a cycle detector that lets us detect these situations and "cut off"
//! these paths, not trying to extend them any further.  Note that any cycle detection logic we
//! implement will be a heuristic.  In particular, since our path-finding algorithm will mimic any
//! runtime recursion, a "complete" cycle detection logic would be equivalent to the Halting
//! Problem.
//!
//! Right now, we implement a simple heuristic where we limit the number of distinct paths that we
//! process that have the same start and end nodes.  We do not make any guarantees that we will
//! always use this particular heuristic, however!  We reserve the right to change the heuristic at
//! any time.

use enumset::EnumSet;
use smallvec::SmallVec;
use std::collections::HashMap;

use crate::arena::Handle;
use crate::arena::List;
use crate::arena::ListArena;
use crate::graph::Edge;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::Cyclicity;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::paths::PathResolutionError;
use crate::stitching::Database;
use crate::stitching::OwnedOrDatabasePath;

/// Helps detect similar paths in the path-finding algorithm.
pub struct SimilarPathDetector<P> {
    paths: HashMap<PathKey, SmallVec<[P; 8]>>,
}

#[doc(hidden)]
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PathKey {
    start_node: Handle<Node>,
    end_node: Handle<Node>,
    symbol_stack_precondition_len: usize,
    scope_stack_precondition_len: usize,
    symbol_stack_postcondition_len: usize,
    scope_stack_postcondition_len: usize,
}

#[doc(hidden)]
pub trait HasPathKey: Clone {
    type Arena;
    fn key(&self) -> PathKey;
}

impl HasPathKey for PartialPath {
    type Arena = PartialPaths;

    fn key(&self) -> PathKey {
        PathKey {
            start_node: self.start_node,
            end_node: self.end_node,
            symbol_stack_precondition_len: self.symbol_stack_precondition.len(),
            scope_stack_precondition_len: self.scope_stack_precondition.len(),
            symbol_stack_postcondition_len: self.symbol_stack_postcondition.len(),
            scope_stack_postcondition_len: self.scope_stack_postcondition.len(),
        }
    }
}

impl<P> SimilarPathDetector<P>
where
    P: HasPathKey,
{
    /// Creates a new, empty cycle detector.
    pub fn new() -> SimilarPathDetector<P> {
        SimilarPathDetector {
            paths: HashMap::new(),
        }
    }

    /// Determines whether we should process this path during the path-finding algorithm.  If we have seen
    /// a path with the same start and end node, and the same pre- and postcondition, then we return false.
    /// Otherwise, we return true.
    pub fn has_similar_path<Eq>(
        &mut self,
        _graph: &StackGraph,
        arena: &mut P::Arena,
        path: &P,
        eq: Eq,
    ) -> bool
    where
        Eq: Fn(&mut P::Arena, &P, &P) -> bool,
    {
        let key = path.key();

        let possibly_similar_paths = self.paths.entry(key).or_default();
        for other_path in possibly_similar_paths.iter() {
            if eq(arena, path, other_path) {
                return true;
            }
        }

        possibly_similar_paths.push(path.clone());
        false
    }

    #[cfg(feature = "copious-debugging")]
    pub fn max_bucket_size(&self) -> usize {
        self.paths.iter().map(|b| b.1.len()).max().unwrap_or(0)
    }
}

// ----------------------------------------------------------------------------
// Cycle detector

pub trait Appendable {
    type Ctx;

    fn append_to(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        ctx: &mut Self::Ctx,
        path: &mut PartialPath,
    ) -> Result<(), PathResolutionError>;
    fn start_node(&self, ctx: &mut Self::Ctx) -> Handle<Node>;
    fn end_node(&self, ctx: &mut Self::Ctx) -> Handle<Node>;
}

#[derive(Clone)]
pub struct AppendingCycleDetector<A> {
    appendages: List<A>,
}

pub type Appendables<A> = ListArena<A>;

impl<A: Appendable + Clone> AppendingCycleDetector<A> {
    pub fn new() -> Self {
        Self {
            appendages: List::empty(),
        }
    }

    pub fn from(appendables: &mut Appendables<A>, appendage: A) -> Self {
        let mut result = Self::new();
        result.appendages.push_front(appendables, appendage);
        result
    }

    pub fn append(&mut self, appendables: &mut Appendables<A>, appendage: A) {
        self.appendages.push_front(appendables, appendage);
    }

    /// Tests if the path is cyclic. Returns a vector indicating the kind of cycles that were found.
    /// If appending or concatenating all fragments succeeds, this function will never raise and error.
    pub fn is_cyclic(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        ctx: &mut A::Ctx,
        appendables: &mut Appendables<A>,
    ) -> Result<EnumSet<Cyclicity>, PathResolutionError> {
        let mut cycles = EnumSet::new();

        let end_node = match self.appendages.clone().pop_front(appendables) {
            Some(appendage) => appendage.end_node(ctx),
            None => return Ok(cycles),
        };

        let mut maybe_cyclic_path = None;
        let mut appendages = self.appendages;
        loop {
            // get prefix elements
            let mut prefix_appendages = List::empty();
            loop {
                let appendable = appendages.pop_front(appendables).cloned();
                match appendable {
                    Some(appendage) => {
                        let is_cycle = appendage.start_node(ctx) == end_node;
                        prefix_appendages.push_front(appendables, appendage);
                        if is_cycle {
                            break;
                        }
                    }
                    None => return Ok(cycles),
                }
            }

            // build prefix path -- prefix starts at end_node, because this is a cycle
            let mut prefix_path = PartialPath::from_node(graph, partials, end_node);
            while let Some(appendage) = prefix_appendages.pop_front(appendables) {
                prefix_path.resolve_to_node(graph, partials, appendage.start_node(ctx))?;
                appendage.append_to(graph, partials, ctx, &mut prefix_path)?;
            }

            // build cyclic path
            let cyclic_path = maybe_cyclic_path
                .unwrap_or_else(|| PartialPath::from_node(graph, partials, end_node));
            prefix_path.resolve_to_node(graph, partials, cyclic_path.start_node)?;
            prefix_path.ensure_no_overlapping_variables(partials, &cyclic_path);
            prefix_path.concatenate(graph, partials, &cyclic_path)?;
            if let Some(cyclicity) = prefix_path.is_cyclic(graph, partials) {
                cycles |= cyclicity;
            }
            maybe_cyclic_path = Some(prefix_path);
        }
    }
}

impl Appendable for Edge {
    type Ctx = ();

    fn append_to(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        _: &mut (),
        path: &mut PartialPath,
    ) -> Result<(), PathResolutionError> {
        path.append(graph, partials, *self)
    }

    fn start_node(&self, _: &mut ()) -> Handle<Node> {
        self.source
    }

    fn end_node(&self, _: &mut ()) -> Handle<Node> {
        self.sink
    }
}

impl Appendable for OwnedOrDatabasePath {
    type Ctx = Database;

    fn append_to(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        path: &mut PartialPath,
    ) -> Result<(), PathResolutionError> {
        path.ensure_no_overlapping_variables(partials, self.get(db));
        path.concatenate(graph, partials, self.get(db))
    }

    fn start_node(&self, db: &mut Database) -> Handle<Node> {
        self.get(db).start_node
    }

    fn end_node(&self, db: &mut Database) -> Handle<Node> {
        self.get(db).end_node
    }
}
