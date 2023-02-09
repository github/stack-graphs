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

use std::collections::{HashMap, VecDeque};

use smallvec::SmallVec;

use crate::arena::Handle;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::paths::Path;

/// Helps detect cycles in the path-finding algorithm.
pub struct CycleDetector<P> {
    paths: HashMap<PathKey, SmallVec<[P; 8]>>,
}

#[doc(hidden)]
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PathKey {
    start_node: Handle<Node>,
    end_node: Handle<Node>,
}

#[doc(hidden)]
pub trait HasPathKey: Clone {
    fn key(&self) -> PathKey;
    fn is_shorter_than(&self, other: &Self) -> bool;
}

impl HasPathKey for Path {
    fn key(&self) -> PathKey {
        PathKey {
            start_node: self.start_node,
            end_node: self.end_node,
        }
    }

    fn is_shorter_than(&self, other: &Self) -> bool {
        self.edges.len() < other.edges.len() && self.symbol_stack.len() <= other.symbol_stack.len()
    }
}

impl HasPathKey for PartialPath {
    fn key(&self) -> PathKey {
        PathKey {
            start_node: self.start_node,
            end_node: self.end_node,
        }
    }

    fn is_shorter_than(&self, other: &Self) -> bool {
        self.edges.len() < other.edges.len()
            && (self.symbol_stack_precondition.len() + self.symbol_stack_postcondition.len())
                <= (other.symbol_stack_precondition.len() + other.symbol_stack_postcondition.len())
    }
}

const MAX_SIMILAR_PATH_COUNT: usize = 13;

impl<P> CycleDetector<P>
where
    P: HasPathKey,
{
    /// Creates a new, empty cycle detector.
    pub fn new() -> CycleDetector<P> {
        CycleDetector {
            paths: HashMap::new(),
        }
    }

    /// Determines whether we should process this path during the path-finding algorithm.  If our
    /// heuristics decide that this path is a duplicate, or is "non-productive", then we return
    /// `false`, and the path-finding algorithm will skip this path.
    pub fn should_process_path<F>(&mut self, path: &P, cmp: F) -> bool
    where
        F: FnMut(&P) -> std::cmp::Ordering,
    {
        let key = path.key();
        let paths_with_same_nodes = self.paths.entry(key).or_default();
        let index = match paths_with_same_nodes.binary_search_by(cmp) {
            // We've already seen this exact path before; no need to process it again.
            Ok(_) => return false,
            // Otherwise add it to the list.
            Err(index) => index,
        };

        // Count how many paths we've already processed that have the same endpoints and are
        // "shorter".
        let similar_path_count = paths_with_same_nodes
            .iter()
            .filter(|similar_path| similar_path.is_shorter_than(path))
            .count();
        if similar_path_count > MAX_SIMILAR_PATH_COUNT {
            return false;
        }

        paths_with_same_nodes.insert(index, path.clone());
        true
    }
}

#[derive(Clone)]
pub struct AppendingCycleDetector {
    states: VecDeque<AppendingPathState>,
}

#[derive(Clone, Copy)]
struct AppendingPathState {
    node: Handle<Node>,
}

impl AppendingCycleDetector {
    pub fn from_node(node: Handle<Node>) -> Self {
        let mut states = Vec::new();
        states.push(AppendingPathState { node });
        Self {
            states: states.into(),
        }
    }

    pub fn appended(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        node: Handle<Node>,
        new_path: &PartialPath,
    ) -> bool {
        if let Some(i) = self.states.iter().position(|s| s.node == node) {
            let mut rhs = PartialPath::from_node(graph, partials, node);
            for s in self.states.range(i + 1..) {
                graph[s.node]
                    .apply_to_partial_stacks(
                        graph,
                        partials,
                        &mut rhs.symbol_stack_precondition,
                        &mut rhs.scope_stack_precondition,
                        &mut rhs.symbol_stack_postcondition,
                        &mut rhs.scope_stack_postcondition,
                    )
                    .unwrap();
            }
            let mut loop_path = new_path.clone();
            if loop_path.concatenate(graph, partials, &rhs).is_ok()
                && loop_path.symbol_stack_postcondition.len()
                    > new_path.symbol_stack_postcondition.len()
            {
                return false;
            }
        }
        true
    }
}

#[derive(Clone)]
pub struct JoiningCycleDetector {
    states: VecDeque<JoiningPathState>,
}

#[derive(Clone)]
struct JoiningPathState {
    path: PartialPath, // FIXME replace with handle into db
}

impl JoiningCycleDetector {
    pub fn from_partial_path(
        _graph: &StackGraph,
        _partials: &mut PartialPaths,
        path: PartialPath,
    ) -> Self {
        let mut states = Vec::new();
        states.push(JoiningPathState { path });
        Self {
            states: states.into(),
        }
    }

    pub fn joined(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        new_path: &PartialPath,
    ) -> bool {
        if let Some(i) = self
            .states
            .iter()
            .position(|s| s.path.start_node == new_path.end_node)
        {
            let mut rhs = self.states[i].path.clone();
            for s in self.states.range(i + 1..) {
                rhs.concatenate(graph, partials, &s.path).unwrap();
            }
            let mut loop_path = new_path.clone();
            if loop_path.concatenate(graph, partials, &rhs).is_ok()
                && loop_path.symbol_stack_postcondition.len()
                    > new_path.symbol_stack_postcondition.len()
            {
                return false;
            }
        }
        true
    }
}
