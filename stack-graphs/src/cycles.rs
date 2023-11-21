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
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::arena::Arena;
use crate::arena::Handle;
use crate::arena::List;
use crate::arena::ListArena;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::Cyclicity;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::paths::PathResolutionError;
use crate::stats::FrequencyDistribution;
use crate::stitching::Appendable;
use crate::stitching::ToAppendable;

/// Helps detect similar paths in the path-finding algorithm.
pub struct SimilarPathDetector<P> {
    paths: HashMap<PathKey, SmallVec<[P; 4]>>,
    counts: Option<HashMap<PathKey, SmallVec<[usize; 4]>>>,
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
            counts: None,
        }
    }

    /// Set whether to collect statistics for this similar path detector.
    pub fn set_collect_stats(&mut self, collect_stats: bool) {
        if !collect_stats {
            self.counts = None;
        } else if self.counts.is_none() {
            self.counts = Some(HashMap::new());
        }
    }

    /// Add a path, and determine whether we should process this path during the path-finding algorithm.
    /// If we have seen a path with the same start and end node, and the same pre- and postcondition, then
    /// we return false. Otherwise, we return true.
    pub fn add_path<Cmp>(
        &mut self,
        _graph: &StackGraph,
        arena: &mut P::Arena,
        path: &P,
        cmp: Cmp,
    ) -> bool
    where
        Cmp: Fn(&mut P::Arena, &P, &P) -> Option<Ordering>,
    {
        let key = path.key();

        // Iterate through the bucket to determine if this paths is better than any already known
        // path. Note that the bucket might be modified during the loop if a path is removed which
        // is shadowed by the new path!
        let possibly_similar_paths = self.paths.entry(key.clone()).or_default();
        let mut possible_similar_counts = self
            .counts
            .as_mut()
            .map(move |cs| cs.entry(key).or_default());
        let mut idx = 0;
        let mut count = 0;
        while idx < possibly_similar_paths.len() {
            let other_path = &mut possibly_similar_paths[idx];
            match cmp(arena, path, other_path) {
                Some(Ordering::Less) => {
                    // the new path is better, remove the old one
                    possibly_similar_paths.remove(idx);
                    if let Some(possible_similar_counts) = possible_similar_counts.as_mut() {
                        count += possible_similar_counts[idx];
                        possible_similar_counts.remove(idx);
                    }
                    // keep `idx` which now points to the next element
                    continue;
                }
                Some(_) => {
                    // the new path is equal or worse, and ignored
                    if let Some(possible_similar_counts) = possible_similar_counts {
                        possible_similar_counts[idx] += 1;
                    }
                    return true;
                }
                None => {
                    idx += 1;
                }
            }
        }

        // this path is either new or better, keep it
        possibly_similar_paths.push(path.clone());
        if let Some(possible_similar_counts) = possible_similar_counts {
            possible_similar_counts.push(count);
        }
        false
    }

    #[cfg(feature = "copious-debugging")]
    pub fn max_bucket_size(&self) -> usize {
        self.paths.iter().map(|b| b.1.len()).max().unwrap_or(0)
    }

    // Returns the distribution of similar path counts.
    pub fn stats(&self) -> SimilarPathStats {
        let mut stats = SimilarPathStats::default();
        if let Some(counts) = &self.counts {
            for bucket in counts.values() {
                stats.similar_path_bucket_size.record(bucket.len());
                for count in bucket.iter() {
                    stats.similar_path_count.record(*count);
                }
            }
        }
        stats
    }
}

#[derive(Clone, Debug, Default)]
pub struct SimilarPathStats {
    // The distribution of the number of similar paths detected
    pub similar_path_count: FrequencyDistribution<usize>,
    // The distribution of the internal bucket sizes in the similar path detector
    pub similar_path_bucket_size: FrequencyDistribution<usize>,
}

impl std::ops::AddAssign<Self> for SimilarPathStats {
    fn add_assign(&mut self, rhs: Self) {
        self.similar_path_bucket_size += rhs.similar_path_bucket_size;
        self.similar_path_count += rhs.similar_path_count;
    }
}

impl std::ops::AddAssign<&Self> for SimilarPathStats {
    fn add_assign(&mut self, rhs: &Self) {
        self.similar_path_bucket_size += &rhs.similar_path_bucket_size;
        self.similar_path_count += &rhs.similar_path_count;
    }
}

// ----------------------------------------------------------------------------
// Cycle detector

/// An arena used by [`AppendingCycleDetector`][] to store the path component lists.
/// The arena is shared between all cycle detectors in a path stitching run, so that
/// the cycle detectors themselves can be small and cheaply cloned.
pub struct Appendables<H> {
    /// List arena for appendable lists
    elements: ListArena<InternedOrHandle<H>>,
    /// Arena for interned partial paths
    interned: Arena<PartialPath>,
}

impl<H> Appendables<H> {
    pub fn new() -> Self {
        Self {
            elements: ListArena::new(),
            interned: Arena::new(),
        }
    }
}

/// Enum that unifies handles to initial paths interned in the cycle detector, and appended
/// handles to appendables in the external database.
#[derive(Clone)]
enum InternedOrHandle<H> {
    Interned(Handle<PartialPath>),
    Database(H),
}

impl<H> InternedOrHandle<H>
where
    H: Clone,
{
    fn append_to<'a, A, Db>(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &'a Db,
        interned: &Arena<PartialPath>,
        path: &mut PartialPath,
    ) -> Result<(), PathResolutionError>
    where
        A: Appendable + 'a,
        Db: ToAppendable<H, A>,
    {
        match self {
            Self::Interned(h) => interned.get(*h).append_to(graph, partials, path),
            Self::Database(h) => db.get_appendable(h).append_to(graph, partials, path),
        }
    }

    fn start_node<'a, A, Db>(&self, db: &'a Db, interned: &Arena<PartialPath>) -> Handle<Node>
    where
        A: Appendable + 'a,
        Db: ToAppendable<H, A>,
    {
        match self {
            Self::Interned(h) => interned.get(*h).start_node,
            Self::Database(h) => db.get_appendable(h).start_node(),
        }
    }

    fn end_node<'a, A, Db>(&self, db: &'a Db, interned: &Arena<PartialPath>) -> Handle<Node>
    where
        A: Appendable + 'a,
        Db: ToAppendable<H, A>,
    {
        match self {
            Self::Interned(h) => interned.get(*h).end_node,
            Self::Database(h) => db.get_appendable(h).end_node(),
        }
    }
}

/// A cycle detector that builds up paths by appending elements to it.
/// Path elements are stored in a shared arena that must be provided
/// when calling methods, so that cloning the cycle detector itself is
/// cheap.
#[derive(Clone)]
pub struct AppendingCycleDetector<H> {
    appendages: List<InternedOrHandle<H>>,
}

impl<H> AppendingCycleDetector<H> {
    pub fn new() -> Self {
        Self {
            appendages: List::empty(),
        }
    }

    pub fn from(appendables: &mut Appendables<H>, path: PartialPath) -> Self {
        let h = appendables.interned.add(path);
        let mut result = Self::new();
        result
            .appendages
            .push_front(&mut appendables.elements, InternedOrHandle::Interned(h));
        result
    }

    pub fn append(&mut self, appendables: &mut Appendables<H>, appendage: H) {
        self.appendages.push_front(
            &mut appendables.elements,
            InternedOrHandle::Database(appendage),
        );
    }
}

impl<H> AppendingCycleDetector<H>
where
    H: Clone,
{
    /// Tests if the path is cyclic. Returns a vector indicating the kind of cycles that were found.
    /// If appending or concatenating all fragments succeeds, this function will never raise and error.
    pub fn is_cyclic<'a, A, Db>(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &'a Db,
        appendables: &mut Appendables<H>,
    ) -> Result<EnumSet<Cyclicity>, PathResolutionError>
    where
        A: Appendable + 'a,
        Db: ToAppendable<H, A>,
    {
        let mut cycles = EnumSet::new();

        let end_node = match self.appendages.clone().pop_front(&mut appendables.elements) {
            Some(appendage) => appendage.end_node(db, &appendables.interned),
            None => return Ok(cycles),
        };

        let mut maybe_cyclic_path = None;
        let mut remaining_appendages = self.appendages;
        // Unlike the stored appendages, which are stored in a shared arena, we use a _local_
        // buffer to collect the prefix appendages that we collect for possible cycles. This is
        // to prevent adding elements to the shared arena for every invocation of this method,
        // because they would remain in the arena after the method returns. We take care to
        // minimize (re)allocations by (a) only allocating when a possible cycle is detected,
        // (b) reserving all necessary space before adding elements, and (c) reusing the buffer
        // between loop iterations.
        let mut prefix_appendages = Vec::new();
        loop {
            // find cycle length
            let mut counting_appendages = remaining_appendages;
            let mut cycle_length = 0usize;
            loop {
                let appendable = counting_appendages.pop_front(&mut appendables.elements);
                match appendable {
                    Some(appendage) => {
                        cycle_length += 1;
                        let is_cycle = appendage.start_node(db, &appendables.interned) == end_node;
                        if is_cycle {
                            break;
                        }
                    }
                    None => return Ok(cycles),
                }
            }

            // collect prefix elements (reversing their order)
            prefix_appendages.clear();
            prefix_appendages.reserve(cycle_length);
            for _ in 0..cycle_length {
                let appendable = remaining_appendages
                    .pop_front(&mut appendables.elements)
                    .expect("")
                    .clone();
                prefix_appendages.push(appendable);
            }

            // build prefix path -- prefix starts at end_node, because this is a cycle
            let mut prefix_path = PartialPath::from_node(graph, partials, end_node);
            while let Some(appendage) = prefix_appendages.pop() {
                appendage.append_to(
                    graph,
                    partials,
                    db,
                    &appendables.interned,
                    &mut prefix_path,
                )?;
            }

            // build cyclic path
            let cyclic_path = maybe_cyclic_path
                .unwrap_or_else(|| PartialPath::from_node(graph, partials, end_node));
            cyclic_path.append_to(graph, partials, &mut prefix_path)?;
            if prefix_path.edges.len() > 0 {
                if let Some(cyclicity) = prefix_path.is_cyclic(graph, partials) {
                    cycles |= cyclicity;
                }
            }
            maybe_cyclic_path = Some(prefix_path);
        }
    }
}
