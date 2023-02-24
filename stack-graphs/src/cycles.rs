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

use std::collections::HashMap;
use std::collections::VecDeque;

use smallvec::SmallVec;

use crate::arena::Handle;
use crate::graph::Edge;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::paths::Extend;
use crate::paths::Path;
use crate::stitching::Database;
use crate::stitching::OwnedOrDatabasePath;

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

const MAX_SIMILAR_PATH_COUNT: usize = 7;

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
pub struct EdgeAppendingCycleDetector {
    edges: VecDeque<Edge>,
}

impl EdgeAppendingCycleDetector {
    pub fn new() -> Self {
        Self {
            edges: vec![].into(),
        }
    }

    pub fn append_edge(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        edge: Edge,
    ) -> Result<(), ()> {
        let end_node = edge.sink;
        self.edges.push(edge);

        let mut cyclic_path = PartialPath::from_node(graph, partials, end_node);
        let mut last_idx = self.edges.len();
        for idx in (0..last_idx).rev() {
            let edge = self.edges[idx];
            if edge.source != cyclic_path.end_node {
                continue;
            }

            let mut prefix_path = PartialPath::from_node(graph, partials, edge.source);
            for idx in idx..last_idx {
                let edge = self.edges[idx];
                prefix_path
                    .resolve_to(graph, partials, edge.source)
                    .unwrap();
                prefix_path.append(graph, partials, edge).unwrap();
            }
            last_idx = idx;
            prefix_path.ensure_no_overlapping_variables(partials, &cyclic_path);
            prefix_path
                .concatenate(graph, partials, &cyclic_path)
                .unwrap();
            cyclic_path = prefix_path;

            if !cyclic_path.is_productive(graph, partials) {
                return Err(());
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct PartialPathAppendingCycleDetector {
    paths: VecDeque<OwnedOrDatabasePath>,
}

impl PartialPathAppendingCycleDetector {
    pub fn from_partial_path(
        _graph: &StackGraph,
        _partials: &mut PartialPaths,
        _db: &mut Database,
        path: OwnedOrDatabasePath,
    ) -> Self {
        Self {
            paths: vec![path].into(),
        }
    }

    pub fn append_partial_path(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &Database,
        path: OwnedOrDatabasePath,
    ) -> Result<(), ()> {
        let end_node = path.get(db).end_node;
        self.paths.push(path);

        let mut cyclic_path = PartialPath::from_node(graph, partials, end_node);
        let mut last_idx = self.paths.len();
        for idx in (0..last_idx).rev() {
            let path = self.paths[idx].get(db);
            if path.start_node != cyclic_path.end_node {
                continue;
            }

            let mut prefix_path = path.clone();
            for idx in (idx + 1)..last_idx {
                let path = self.paths[idx].get(db);
                prefix_path
                    .resolve_to(graph, partials, path.start_node)
                    .unwrap();
                prefix_path.ensure_no_overlapping_variables(partials, path);
                prefix_path.concatenate(graph, partials, path).unwrap();
            }
            last_idx = idx;
            prefix_path
                .resolve_to(graph, partials, cyclic_path.start_node)
                .unwrap();
            prefix_path.ensure_no_overlapping_variables(partials, &cyclic_path);
            prefix_path
                .concatenate(graph, partials, &cyclic_path)
                .unwrap();
            cyclic_path = prefix_path;

            if !cyclic_path.is_productive(graph, partials) {
                return Err(());
            }
        }

        Ok(())
    }
}
