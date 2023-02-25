// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Partial paths can be "stitched together" to produce name-binding paths.
//!
//! The "path stitching" algorithm defined in this module is how we take a collection of [partial
//! paths][] and use them to build up name-binding paths.  Our conjecture is that by building paths
//! this way, we can precompute a useful amount of work at _index time_ (when we construct the
//! partial paths), to reduce the amount of work that needs to be done at _query time_ (when those
//! partial paths are stitched together into paths).
//!
//! Complicating this story is that for large codebases (especially those with many upstream and
//! downstream dependencies), there is a _very_ large set of partial paths available to us.  We
//! want to be able to load those in _lazily_, during the execution of the path-stitching
//! algorithm.
//!
//! The [`Database`][] and [`PathStitcher`][] types provide this functionality.  `Database`
//! manages a collection of partial paths that have been loaded into this process from some
//! external data store.  `PathStitcher` implements the path-stitching algorithm in _phases_.
//! During each phase, we will process a set of (possibly incomplete) paths, looking in the
//! `Database` for the set of partial paths that are compatible with those paths.  It is your
//! responsibility to make sure that the database contains all of possible extensions of the paths
//! that we're going to process in that phase.  For the first phase, you already know which
//! paths you're starting the search from, and must make sure that the database starts out
//! containing the possible extensions of those "seed" paths.  For subsequent phases, you get to
//! see which paths will be processed in the _next_ phase as part of handling the _current_ phase.
//! This gives you the opporunity to load additional partial paths into the `Database` before
//! allowing the next phase to proceed.
//!
//! [partial paths]: ../partial/index.html
//! [`Database`]: struct.Database.html
//! [`PathStitcher`]: struct.PathStitcher.html

use std::collections::HashMap;
use std::collections::VecDeque;
#[cfg(feature = "copious-debugging")]
use std::fmt::Display;
use std::ops::Index;

use crate::arena::Arena;
use crate::arena::Handle;
use crate::arena::HandleSet;
use crate::arena::List;
use crate::arena::ListArena;
use crate::arena::ListCell;
use crate::arena::SupplementalArena;
use crate::cycles::Appendages;
use crate::cycles::AppendingCycleDetector;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::partial::Cyclicity;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::partial::PartialSymbolStack;
use crate::paths::Path;
use crate::paths::Paths;
use crate::paths::SymbolStack;
use crate::CancellationError;
use crate::CancellationFlag;

//-------------------------------------------------------------------------------------------------
// Databases

/// Contains a "database" of partial paths.
///
/// This type is meant to be a lazily loaded "view" into a proper storage layer.  During the
/// path-stitching algorithm, we repeatedly try to extend a currently incomplete path with any
/// partial paths that are compatible with it.  For large codebases, or projects with a large
/// number of dependencies, it can be prohibitive to load in _all_ of the partial paths up-front.
/// We've written the path-stitching algorithm so that you have a chance to only load in the
/// partial paths that are actually needed, placing them into a `Database` instance as they're
/// needed.
pub struct Database {
    pub(crate) partial_paths: Arena<PartialPath>,
    pub(crate) local_nodes: HandleSet<Node>,
    symbol_stack_keys: ListArena<Handle<Symbol>>,
    symbol_stack_key_cache: HashMap<SymbolStackCacheKey, SymbolStackKeyHandle>,
    paths_by_start_node: SupplementalArena<Node, Vec<Handle<PartialPath>>>,
    root_paths_by_precondition: SupplementalArena<SymbolStackKeyCell, Vec<Handle<PartialPath>>>,
}

impl Database {
    /// Creates a new, empty database.
    pub fn new() -> Database {
        Database {
            partial_paths: Arena::new(),
            local_nodes: HandleSet::new(),
            symbol_stack_keys: List::new_arena(),
            symbol_stack_key_cache: HashMap::new(),
            paths_by_start_node: SupplementalArena::new(),
            root_paths_by_precondition: SupplementalArena::new(),
        }
    }

    /// Adds a partial path to this database.  We do not deduplicate partial paths in any way; it's
    /// your responsibility to only add each partial path once.
    pub fn add_partial_path(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        path: PartialPath,
    ) -> Handle<PartialPath> {
        let start_node = path.start_node;
        copious_debugging!(
            "    Add {} path to database {}",
            if graph[start_node].is_root() {
                "root"
            } else {
                "node"
            },
            path.display(graph, partials)
        );
        let symbol_stack_precondition = path.symbol_stack_precondition;
        let handle = self.partial_paths.add(path);

        // If the partial path starts at the root node, index it by its symbol stack precondition.
        if graph[start_node].is_root() {
            let key = SymbolStackKey::from_partial_symbol_stack(
                partials,
                self,
                symbol_stack_precondition,
            );
            if !key.is_empty() {
                let key_handle = key.back_handle();
                self.root_paths_by_precondition[key_handle].push(handle);
            }
        } else {
            // Otherwise index it by its source node.
            self.paths_by_start_node[start_node].push(handle);
        }

        handle
    }

    /// Find all partial paths in this database that start at the root node, and have a symbol
    /// stack precondition that is compatible with a given symbol stack.
    #[cfg_attr(not(feature = "copious-debugging"), allow(unused_variables))]
    pub fn find_candidate_partial_paths_from_root<R>(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        symbol_stack: Option<SymbolStackKey>,
        result: &mut R,
    ) where
        R: std::iter::Extend<Handle<PartialPath>>,
    {
        // If the path currently ends at the root node, then we need to look up partial paths whose
        // symbol stack precondition is compatible with the path.
        match symbol_stack {
            Some(mut symbol_stack) => loop {
                copious_debugging!(
                    "      Search for symbol stack <{}>",
                    symbol_stack.display(graph, self)
                );
                let key_handle = symbol_stack.back_handle();
                if let Some(paths) = self.root_paths_by_precondition.get(key_handle) {
                    #[cfg(feature = "copious-debugging")]
                    {
                        for path in paths {
                            copious_debugging!(
                                "        Found path {}",
                                self[*path].display(graph, partials)
                            );
                        }
                    }
                    result.extend(paths.iter().copied());
                }
                if symbol_stack.pop_back(self).is_none() {
                    break;
                }
            },
            None => {
                copious_debugging!("      Search for all root paths");
                for (_, paths) in self.root_paths_by_precondition.iter() {
                    #[cfg(feature = "copious-debugging")]
                    {
                        for path in paths {
                            copious_debugging!(
                                "        Found path {}",
                                self[*path].display(graph, partials)
                            );
                        }
                    }
                    result.extend(paths.iter().copied());
                }
            }
        }
    }

    /// Find all partial paths in the database that start at the given node.  We don't filter the
    /// results any further than that, since we have to check each partial path for compatibility
    /// as we try to append it to the current incomplete path anyway, and non-root nodes will
    /// typically have a small number of outgoing edges.
    #[cfg_attr(not(feature = "copious-debugging"), allow(unused_variables))]
    pub fn find_candidate_partial_paths_from_node<R>(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        start_node: Handle<Node>,
        result: &mut R,
    ) where
        R: std::iter::Extend<Handle<PartialPath>>,
    {
        copious_debugging!("      Search for start node {}", start_node.display(graph));
        // Return all of the partial paths that start at the requested node.
        if let Some(paths) = self.paths_by_start_node.get(start_node) {
            #[cfg(feature = "copious-debugging")]
            {
                for path in paths {
                    copious_debugging!(
                        "        Found path {}",
                        self[*path].display(graph, partials)
                    );
                }
            }
            result.extend(paths.iter().copied());
        }
    }

    /// Determines which nodes in the stack graph are “local”, taking into account the partial
    /// paths in this database.
    ///
    /// A local node has no partial path that connects it to the root node in either direction.
    /// That means that it cannot participate in any paths that leave the file.
    ///
    /// This method is meant to be used at index time, to calculate the set of nodes that are local
    /// after having just calculated the set of partial paths for the file.
    pub fn find_local_nodes(&mut self) {
        // Assume that any node that is the start or end of a partial path is local to this file
        // until we see a path connecting the root node to it (in either direction).
        self.local_nodes.clear();
        for handle in self.iter_partial_paths() {
            self.local_nodes.add(self[handle].start_node);
            self.local_nodes.add(self[handle].end_node);
        }

        // The root node and jump-to-scope node are the most obvious non-local nodes.
        let mut nonlocal_start_nodes = HandleSet::new();
        let mut nonlocal_end_nodes = HandleSet::new();
        self.local_nodes.remove(StackGraph::root_node());
        nonlocal_start_nodes.add(StackGraph::root_node());
        nonlocal_end_nodes.add(StackGraph::root_node());
        self.local_nodes.remove(StackGraph::jump_to_node());
        nonlocal_start_nodes.add(StackGraph::jump_to_node());
        nonlocal_end_nodes.add(StackGraph::jump_to_node());

        // Other nodes are non-local if we see any partial path that connects it to another
        // non-local node.  Repeat until we reach a fixed point.
        let mut keep_checking = true;
        while keep_checking {
            keep_checking = false;
            for handle in self.iter_partial_paths() {
                let start_node = self[handle].start_node;
                let end_node = self[handle].end_node;

                // First check forwards paths, where non-localness propagates from the start node
                // of each path.
                let start_node_is_nonlocal = nonlocal_start_nodes.contains(start_node);
                let end_node_is_nonlocal = nonlocal_start_nodes.contains(end_node);
                if start_node_is_nonlocal && !end_node_is_nonlocal {
                    keep_checking = true;
                    nonlocal_start_nodes.add(end_node);
                    self.local_nodes.remove(end_node);
                }

                // Then check reverse paths, where non-localness propagates from the end node of
                // each path.
                let start_node_is_nonlocal = nonlocal_end_nodes.contains(start_node);
                let end_node_is_nonlocal = nonlocal_end_nodes.contains(end_node);
                if !start_node_is_nonlocal && end_node_is_nonlocal {
                    keep_checking = true;
                    nonlocal_end_nodes.add(start_node);
                    self.local_nodes.remove(start_node);
                }
            }
        }
    }

    /// Marks that a stack graph node is local.
    ///
    /// This method is meant to be used at query time.  You will have precalculated the set of
    /// local nodes for a file at index time; at query time, you will load this information from
    /// your storage layer and use this method to update our internal view of which nodes are
    /// local.
    pub fn mark_local_node(&mut self, node: Handle<Node>) {
        self.local_nodes.add(node);
    }

    /// Returns whether a node is local according to the partial paths in this database.  You must
    /// have already called [`find_local_nodes`][] or [`mark_local_node`][], depending on whether
    /// it is index time or query time.
    pub fn node_is_local(&self, node: Handle<Node>) -> bool {
        self.local_nodes.contains(node)
    }

    /// Returns an iterator over all of the handles of all of the partial paths in this database.
    /// (Note that because we're only returning _handles_, this iterator does not retain a
    /// reference to the `Database`.)
    pub fn iter_partial_paths(&self) -> impl Iterator<Item = Handle<PartialPath>> {
        self.partial_paths.iter_handles()
    }

    pub fn ensure_both_directions(&mut self, partials: &mut PartialPaths) {
        for path in self.partial_paths.iter_handles() {
            self.partial_paths
                .get_mut(path)
                .ensure_both_directions(partials);
        }
    }

    pub fn ensure_forwards(&mut self, partials: &mut PartialPaths) {
        for path in self.partial_paths.iter_handles() {
            self.partial_paths.get_mut(path).ensure_forwards(partials);
        }
    }
}

impl Index<Handle<PartialPath>> for Database {
    type Output = PartialPath;
    #[inline(always)]
    fn index(&self, handle: Handle<PartialPath>) -> &PartialPath {
        self.partial_paths.get(handle)
    }
}

/// The key type that we use to find partial paths that start from the root node and have a
/// particular symbol stack as their precondition.
#[derive(Clone, Copy)]
pub struct SymbolStackKey {
    // Note: the symbols are stored in reverse order, with the "front" of the List being the "back"
    // of the symbol stack.  That lets us easily get a handle to the back of the symbol stack, and
    // also lets us easily pops items off the back of key, which we need to do to search for all
    // prefixes of a particular symbol stack down in `find_candidate_partial_paths_from_root`.
    symbols: List<Handle<Symbol>>,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct SymbolStackCacheKey {
    head: Handle<Symbol>,
    tail: SymbolStackKeyHandle,
}

type SymbolStackKeyCell = ListCell<Handle<Symbol>>;
type SymbolStackKeyHandle = Handle<SymbolStackKeyCell>;

impl SymbolStackKey {
    /// Returns an empty symbol stack key.
    fn empty() -> SymbolStackKey {
        SymbolStackKey {
            symbols: List::empty(),
        }
    }

    fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Pushes a new symbol onto the back of this symbol stack key.
    fn push_back(&mut self, db: &mut Database, symbol: Handle<Symbol>) {
        let cache_key = SymbolStackCacheKey {
            head: symbol,
            tail: self.back_handle(),
        };
        if let Some(handle) = db.symbol_stack_key_cache.get(&cache_key) {
            self.symbols = List::from_handle(*handle);
            return;
        }
        // push_front because we store the key's symbols in reverse order.
        self.symbols.push_front(&mut db.symbol_stack_keys, symbol);
        let handle = self.back_handle();
        db.symbol_stack_key_cache.insert(cache_key, handle);
    }

    /// Pops a symbol from the back of this symbol stack key.
    fn pop_back(&mut self, db: &Database) -> Option<Handle<Symbol>> {
        // pop_front because we store the key's symbols in reverse order.
        self.symbols.pop_front(&db.symbol_stack_keys).copied()
    }

    /// Extracts a new symbol stack key from a symbol stack.
    pub fn from_symbol_stack(
        paths: &mut Paths,
        db: &mut Database,
        mut stack: SymbolStack,
    ) -> SymbolStackKey {
        let mut result = SymbolStackKey::empty();
        while let Some(symbol) = stack.pop_front(paths) {
            result.push_back(db, symbol.symbol);
        }
        result
    }

    /// Extracts a new symbol stack key from a partial symbol stack.
    pub fn from_partial_symbol_stack(
        partials: &mut PartialPaths,
        db: &mut Database,
        mut stack: PartialSymbolStack,
    ) -> SymbolStackKey {
        let mut result = SymbolStackKey::empty();
        while let Some(symbol) = stack.pop_front(partials) {
            result.push_back(db, symbol.symbol);
        }
        result
    }

    /// Returns a handle to the back of the symbol stack key.
    fn back_handle(self) -> SymbolStackKeyHandle {
        // Because the symbols are stored in reverse order, the handle to the "front" of the list
        // is a handle to the "back" of the key.
        self.symbols.handle()
    }

    #[cfg(feature = "copious-debugging")]
    fn display<'a>(self, graph: &'a StackGraph, db: &'a Database) -> impl Display + 'a {
        DisplaySymbolStackKey(self, graph, db)
    }
}

#[cfg(feature = "copious-debugging")]
struct DisplaySymbolStackKey<'a>(SymbolStackKey, &'a StackGraph, &'a Database);

#[cfg(feature = "copious-debugging")]
impl<'a> Display for DisplaySymbolStackKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Use a recursive function to print the contents of the key out in reverse order.
        fn display_one(
            mut key: SymbolStackKey,
            graph: &StackGraph,
            db: &Database,
            f: &mut std::fmt::Formatter,
        ) -> std::fmt::Result {
            let last = match key.pop_back(db) {
                Some(last) => last,
                None => return Ok(()),
            };
            display_one(key, graph, db, f)?;
            last.display(graph).fmt(f)
        }
        display_one(self.0, self.1, self.2, f)
    }
}

//-------------------------------------------------------------------------------------------------
// Stitching partial paths together

/// Implements a phased path-stitching algorithm.
///
/// Our overall goal is to start with a set of _seed_ paths, and to repeatedly extend each path by
/// appending a compatible partial path onto the end of it.  (If there are multiple compatible
/// partial paths, we append each of them separately, resulting in more than one extension for the
/// current path.)
///
/// We perform this processing in _phases_.  At the start of each phase, we have a _current set_ of
/// paths that need to be processed.  As we extend those paths, we add the extensions to the set of
/// paths to process in the _next_ phase.  Phases are processed one at a time, each time you invoke
/// the [`process_next_phase`][] method.
///
/// [`process_next_phase`]: #method.process_next_phase
///
/// After each phase has completed, you can use the [`previous_phase_paths`][] method to retrieve
/// all of the paths that were discovered during that phase.  That gives you a chance to add to the
/// `Database` all of the partial paths that we might need to extend those paths with before
/// invoking the next phase.
///
/// [`previous_phase_paths`]: #method.previous_phase_paths
///
/// If you don't care about this phasing nonsense, you can instead preload your `Database` with all
/// possible partial paths, and run the path-stitching algorithm all the way to completion, using
/// the [`find_all_complete_paths`][] method.
///
/// [`find_all_complete_paths`]: #method.find_all_complete_paths
pub struct PathStitcher {
    candidate_paths: Vec<Handle<PartialPath>>,
    queue: VecDeque<(Path, AppendingCycleDetector<OwnedOrDatabasePath>)>,
    next_iteration: (
        VecDeque<Path>,
        VecDeque<AppendingCycleDetector<OwnedOrDatabasePath>>,
    ),
    appended_paths: Appendages<OwnedOrDatabasePath>,
    max_work_per_phase: usize,
    #[cfg(feature = "copious-debugging")]
    phase_number: usize,
}

impl PathStitcher {
    /// Creates a new path stitcher that is "seeded" with a set of starting stack graph nodes.
    ///
    /// Before calling this method, you must ensure that `db` contains all of the possible partial
    /// paths that start with any of your requested starting nodes.
    ///
    /// Before calling [`process_next_phase`][] for the first time, you must ensure that `db`
    /// contains all possible extensions of any of those initial paths.  You can retrieve a list of
    /// those extensions via [`previous_phase_paths`][].
    ///
    /// [`previous_phase_paths`]: #method.previous_phase_paths
    /// [`process_next_phase`]: #method.process_next_phase
    pub fn new<I>(
        graph: &StackGraph,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        db: &mut Database,
        starting_nodes: I,
    ) -> PathStitcher
    where
        I: IntoIterator<Item = Handle<Node>>,
    {
        copious_debugging!("==> Start phase 0");
        let mut candidate_paths = Vec::new();
        for node in starting_nodes {
            copious_debugging!("    Initial node {}", node.display(graph));
            db.find_candidate_partial_paths_from_node(graph, partials, node, &mut candidate_paths);
        }
        let mut appended_paths = Appendages::new();
        let next_iteration = candidate_paths
            .iter()
            .filter_map(|partial_path| {
                Path::from_partial_path(graph, paths, partials, &db[*partial_path]).map(|p| {
                    (
                        p,
                        AppendingCycleDetector::from(&mut appended_paths, partial_path.into()),
                    )
                })
            })
            .unzip();
        copious_debugging!("==> End phase 0");
        PathStitcher {
            candidate_paths,
            queue: VecDeque::new(),
            next_iteration,
            appended_paths,
            // By default, there's no artificial bound on the amount of work done per phase
            max_work_per_phase: usize::MAX,
            #[cfg(feature = "copious-debugging")]
            phase_number: 1,
        }
    }

    /// Returns an iterator of all of the (possibly incomplete) paths that were encountered during
    /// the most recent phase of the path-stitching algorithm.
    pub fn previous_phase_paths(&self) -> impl Iterator<Item = &Path> + '_ {
        self.next_iteration.0.iter()
    }

    /// Returns a slice of all of the (possibly incomplete) paths that were encountered during the
    /// most recent phase of the path-stitching algorithm.
    pub fn previous_phase_paths_slice(&mut self) -> &[Path] {
        self.next_iteration.0.make_contiguous();
        self.next_iteration.0.as_slices().0
    }

    /// Returns a mutable slice of all of the (possibly incomplete) paths that were encountered
    /// during the most recent phase of the path-stitching algorithm.
    pub fn previous_phase_paths_slice_mut(&mut self) -> &mut [Path] {
        self.next_iteration.0.make_contiguous();
        self.next_iteration.0.as_mut_slices().0
    }

    /// Sets the maximum amount of work that can be performed during each phase of the algorithm.
    /// By bounding our work this way, you can ensure that it's not possible for our CPU-bound
    /// algorithm to starve any worker threads or processes that you might be using.  If you don't
    /// call this method, then we allow ourselves to process all of the extensions of all of the
    /// paths found in the previous phase, with no additional bound.
    pub fn set_max_work_per_phase(&mut self, max_work_per_phase: usize) {
        self.max_work_per_phase = max_work_per_phase;
    }

    /// Attempts to extend one path as part of the path-stitching algorithm.  When calling this
    /// function, you are responsible for ensuring that `db` already contains all of the possible
    /// partial paths that we might want to extend `path` with.
    fn stitch_path(
        &mut self,
        graph: &StackGraph,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        db: &mut Database,
        path: &Path,
        cycle_detector: AppendingCycleDetector<OwnedOrDatabasePath>,
    ) -> usize {
        copious_debugging!("--> Candidate path {}", path.display(graph, paths));
        self.candidate_paths.clear();
        if graph[path.end_node].is_root() {
            let key = SymbolStackKey::from_symbol_stack(paths, db, path.symbol_stack);
            db.find_candidate_partial_paths_from_root(
                graph,
                partials,
                Some(key),
                &mut self.candidate_paths,
            );
        } else {
            db.find_candidate_partial_paths_from_node(
                graph,
                partials,
                path.end_node,
                &mut self.candidate_paths,
            );
        }

        let extension_count = self.candidate_paths.len();
        self.next_iteration.0.reserve(extension_count);
        self.next_iteration.1.reserve(extension_count);
        for extension in &self.candidate_paths {
            let extension_path = &db[*extension];
            copious_debugging!("    Extend {}", path.display(graph, paths),);
            copious_debugging!("      with {}", extension_path.display(graph, partials));
            let mut new_path = path.clone();
            let mut new_cycle_detector = cycle_detector.clone();
            // If there are errors adding this partial path to the path, or resolving the resulting
            // path, just skip the partial path — it's not a fatal error.
            #[cfg_attr(not(feature = "copious-debugging"), allow(unused_variables))]
            if let Err(err) = new_path.append_partial_path(graph, paths, partials, extension_path) {
                copious_debugging!("        is invalid: {:?}", err);
                continue;
            }
            new_cycle_detector.append(&mut self.appended_paths, extension.into());
            let cycles = new_cycle_detector
                .is_cyclic(graph, partials, db, &mut self.appended_paths)
                .unwrap();
            if !cycles
                .into_iter()
                .all(|c| c == Cyclicity::StrengthensPrecondition)
            {
                copious_debugging!("        is invalid: cyclic");
                continue;
            }
            copious_debugging!("        is {}", new_path.display(graph, paths));
            self.next_iteration.0.push_back(new_path);
            self.next_iteration.1.push_back(new_cycle_detector);
        }

        extension_count
    }

    /// Returns whether the path-stitching algorithm has completed.
    pub fn is_complete(&self) -> bool {
        self.queue.is_empty() && self.next_iteration.0.is_empty()
    }

    /// Runs the next phase of the path-stitching algorithm.  We will have built up a set of
    /// incomplete paths during the _previous_ phase.  Before calling this function, you must
    /// ensure that `db` contains all of the possible partial paths that we might want to extend
    /// any of those paths with.
    ///
    /// After this method returns, you can use [`previous_phase_paths`][] to retrieve a list of the
    /// (possibly incomplete) paths that were encountered during this phase.
    ///
    /// [`previous_phase_paths`]: #method.previous_phase_paths
    pub fn process_next_phase(
        &mut self,
        graph: &StackGraph,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        db: &mut Database,
    ) {
        copious_debugging!("==> Start phase {}", self.phase_number);
        self.queue.extend(
            self.next_iteration
                .0
                .drain(..)
                .zip(self.next_iteration.1.drain(..)),
        );
        let mut work_performed = 0;
        while let Some((path, cycle_detector)) = self.queue.pop_front() {
            work_performed += self.stitch_path(graph, paths, partials, db, &path, cycle_detector);
            if work_performed >= self.max_work_per_phase {
                break;
            }
        }

        #[cfg(feature = "copious-debugging")]
        {
            copious_debugging!("==> End phase {}", self.phase_number);
            self.phase_number += 1;
        }
    }

    /// Returns all of the complete paths that are reachable from a set of starting nodes, building
    /// them up by stitching together partial paths from this database.
    ///
    /// This function will not return until all reachable paths have been processed, so your
    /// database must already contain all partial paths that might be needed.  If you have a very
    /// large stack graph stored in some other storage system, and want more control over lazily
    /// loading only the necessary pieces, then you should code up your own loop that calls
    /// [`process_next_phase`][] manually.
    ///
    /// [`process_next_phase`]: #method.process_next_phase
    pub fn find_all_complete_paths<I>(
        graph: &StackGraph,
        paths: &mut Paths,
        partials: &mut PartialPaths,
        db: &mut Database,
        starting_nodes: I,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<Vec<Path>, CancellationError>
    where
        I: IntoIterator<Item = Handle<Node>>,
    {
        let mut result = Vec::new();
        let mut stitcher = PathStitcher::new(graph, paths, partials, db, starting_nodes);
        while !stitcher.is_complete() {
            cancellation_flag.check("finding complete paths")?;
            let complete_paths = stitcher
                .previous_phase_paths()
                .filter(|path| path.is_complete(graph));
            result.extend(complete_paths.cloned());
            stitcher.process_next_phase(graph, paths, partials, db);
        }
        Ok(result)
    }
}

//-------------------------------------------------------------------------------------------------
// Stitching partial paths together

/// Implements a phased forward partial path stitching algorithm.
///
/// Our overall goal is to start with a set of _seed_ partial paths, and to repeatedly extend each
/// partial path by concatenating another, compatible partial path onto the end of it.  (If there
/// are multiple compatible partial paths, we concatenate each of them separately, resulting in
/// more than one extension for the current path.)
///
/// We perform this processing in _phases_.  At the start of each phase, we have a _current set_ of
/// partial paths that need to be processed.  As we extend those partial paths, we add the
/// extensions to the set of partial paths to process in the _next_ phase.  Phases are processed
/// one at a time, each time you invoke the [`process_next_phase`][] method.
///
/// [`process_next_phase`]: #method.process_next_phase
///
/// After each phase has completed, you can use the [`previous_phase_partial_paths`][] method to
/// retrieve all of the partial paths that were discovered during that phase.  That gives you a
/// chance to add to the `Database` all of the other partial paths that we might need to extend
/// those partial paths with before invoking the next phase.
///
/// [`previous_phase_partial_paths`]: #method.previous_phase_partial_paths
///
/// If you don't care about this phasing nonsense, you can instead preload your `Database` with all
/// possible partial paths, and run the forward partial path stitching algorithm all the way to
/// completion, using the [`find_all_complete_partial_paths`][] method.
///
/// [`find_all_complete_partial_paths`]: #method.find_all_complete_partial_paths
pub struct ForwardPartialPathStitcher {
    candidate_partial_paths: Vec<Handle<PartialPath>>,
    queue: VecDeque<(PartialPath, AppendingCycleDetector<OwnedOrDatabasePath>)>,
    next_iteration: (
        VecDeque<PartialPath>,
        VecDeque<AppendingCycleDetector<OwnedOrDatabasePath>>,
    ),
    appended_paths: Appendages<OwnedOrDatabasePath>,
    max_work_per_phase: usize,
    #[cfg(feature = "copious-debugging")]
    phase_number: usize,
    should_extend: fn(&StackGraph, &mut PartialPaths, &PartialPath) -> bool,
}

impl ForwardPartialPathStitcher {
    /// Creates a new forward partial path stitcher that is "seeded" with a set of starting stack
    /// graph nodes.
    ///
    /// Before calling this method, you must ensure that `db` contains all of the possible partial
    /// paths that start with any of your requested starting nodes.
    ///
    /// Before calling [`process_next_phase`][] for the first time, you must ensure that `db`
    /// contains all possible extensions of any of those initial partial paths.  You can retrieve a
    /// list of those extensions via [`previous_phase_partial paths`][].
    ///
    /// [`previous_phase_partial paths`]: #method.previous_phase_partial paths
    /// [`process_next_phase`]: #method.process_next_phase
    pub fn from_nodes<I>(
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        starting_nodes: I,
    ) -> ForwardPartialPathStitcher
    where
        I: IntoIterator<Item = Handle<Node>>,
    {
        copious_debugging!("==> Start phase 0");
        let mut candidate_partial_paths = Vec::new();
        for node in starting_nodes {
            copious_debugging!("    Initial node {}", node.display(graph));
            if graph[node].is_root() {
                db.find_candidate_partial_paths_from_root(
                    graph,
                    partials,
                    None,
                    &mut candidate_partial_paths,
                );
            } else {
                db.find_candidate_partial_paths_from_node(
                    graph,
                    partials,
                    node,
                    &mut candidate_partial_paths,
                );
            }
        }
        let mut appended_paths = Appendages::new();
        let next_iteration = candidate_partial_paths
            .iter()
            .copied()
            .map(|handle| {
                (
                    db[handle].clone(),
                    AppendingCycleDetector::from(&mut appended_paths, handle.into()),
                )
            })
            .unzip();
        copious_debugging!("==> End phase 0");
        ForwardPartialPathStitcher {
            candidate_partial_paths,
            queue: VecDeque::new(),
            next_iteration,
            appended_paths,
            // By default, there's no artificial bound on the amount of work done per phase
            max_work_per_phase: usize::MAX,
            #[cfg(feature = "copious-debugging")]
            phase_number: 1,
            should_extend: |_, _, _| true,
        }
    }

    /// Creates a new forward partial path stitcher that is "seeded" with a set of initial partial
    /// paths.
    ///
    /// Before calling [`process_next_phase`][] for the first time, you must ensure that `db`
    /// contains all possible extensions of any of those initial partial paths.  You can retrieve a
    /// list of those extensions via [`previous_phase_partial paths`][].
    ///
    /// [`previous_phase_partial paths`]: #method.previous_phase_partial paths
    /// [`process_next_phase`]: #method.process_next_phase
    pub fn from_partial_paths(
        _graph: &StackGraph,
        _partials: &mut PartialPaths,
        _db: &mut Database,
        initial_partial_paths: Vec<PartialPath>,
    ) -> ForwardPartialPathStitcher {
        let mut appended_paths = Appendages::new();
        let next_iteration = initial_partial_paths
            .into_iter()
            .map(|p| {
                let c = AppendingCycleDetector::from(&mut appended_paths, p.clone().into());
                (p, c)
            })
            .unzip();
        ForwardPartialPathStitcher {
            candidate_partial_paths: Vec::new(),
            queue: VecDeque::new(),
            next_iteration,
            appended_paths,
            // By default, there's no artificial bound on the amount of work done per phase
            max_work_per_phase: usize::MAX,
            #[cfg(feature = "copious-debugging")]
            phase_number: 1,
            should_extend: |_, _, _| true,
        }
    }

    /// Returns an iterator of all of the (possibly incomplete) partial paths that were encountered
    /// during the most recent phase of the algorithm.
    pub fn previous_phase_partial_paths(&self) -> impl Iterator<Item = &PartialPath> + '_ {
        self.next_iteration.0.iter()
    }

    /// Returns a slice of all of the (possibly incomplete) partial paths that were encountered
    /// during the most recent phase of the algorithm.
    pub fn previous_phase_partial_paths_slice(&mut self) -> &[PartialPath] {
        self.next_iteration.0.make_contiguous();
        self.next_iteration.0.as_slices().0
    }

    /// Returns a mutable slice of all of the (possibly incomplete) partial paths that were
    /// encountered during the most recent phase of the algorithm.
    pub fn previous_phase_partial_paths_slice_mut(&mut self) -> &mut [PartialPath] {
        self.next_iteration.0.make_contiguous();
        self.next_iteration.0.as_mut_slices().0
    }

    /// Sets the maximum amount of work that can be performed during each phase of the algorithm.
    /// By bounding our work this way, you can ensure that it's not possible for our CPU-bound
    /// algorithm to starve any worker threads or processes that you might be using.  If you don't
    /// call this method, then we allow ourselves to process all of the extensions of all of the
    /// paths found in the previous phase, with no additional bound.
    pub fn set_max_work_per_phase(&mut self, max_work_per_phase: usize) {
        self.max_work_per_phase = max_work_per_phase;
    }

    /// Sets a condition that determines if a partial path is a candidate that should be extended.
    pub fn set_should_extend(
        &mut self,
        should_extend: fn(&StackGraph, &mut PartialPaths, &PartialPath) -> bool,
    ) {
        self.should_extend = should_extend;
    }

    /// Attempts to extend one partial path as part of the algorithm.  When calling this function,
    /// you are responsible for ensuring that `db` already contains all of the possible partial
    /// paths that we might want to extend `partial_path` with.
    fn stitch_partial_path(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        partial_path: &PartialPath,
        cycle_detector: AppendingCycleDetector<OwnedOrDatabasePath>,
    ) -> usize {
        self.candidate_partial_paths.clear();
        if graph[partial_path.end_node].is_root() {
            let key = SymbolStackKey::from_partial_symbol_stack(
                partials,
                db,
                partial_path.symbol_stack_postcondition,
            );
            db.find_candidate_partial_paths_from_root(
                graph,
                partials,
                Some(key),
                &mut self.candidate_partial_paths,
            );
        } else {
            db.find_candidate_partial_paths_from_node(
                graph,
                partials,
                partial_path.end_node,
                &mut self.candidate_partial_paths,
            );
        }

        let extension_count = self.candidate_partial_paths.len();
        self.next_iteration.0.reserve(extension_count);
        self.next_iteration.1.reserve(extension_count);
        for extension in &self.candidate_partial_paths {
            let mut extension_path = db[*extension].clone();
            copious_debugging!("    Extend {}", partial_path.display(graph, partials));
            copious_debugging!("      with {}", extension_path.display(graph, partials));
            extension_path.ensure_no_overlapping_variables(partials, partial_path);
            copious_debugging!("        -> {}", extension_path.display(graph, partials));

            let mut new_partial_path = partial_path.clone();
            let mut new_cycle_detector = cycle_detector.clone();
            // If there are errors concatenating these partial paths, or resolving the resulting
            // partial path, just skip the extension — it's not a fatal error.
            #[cfg_attr(not(feature = "copious-debugging"), allow(unused_variables))]
            {
                if let Err(err) = new_partial_path.concatenate(graph, partials, &extension_path) {
                    copious_debugging!("        is invalid: {:?}", err);
                    continue;
                }
                new_cycle_detector.append(&mut self.appended_paths, extension.into());
                let cycles = new_cycle_detector
                    .is_cyclic(graph, partials, db, &mut self.appended_paths)
                    .unwrap();
                if !cycles
                    .into_iter()
                    .all(|c| c == Cyclicity::StrengthensPrecondition)
                {
                    copious_debugging!("        is invalid: cyclic");
                    continue;
                }
            }
            copious_debugging!("        is {}", new_partial_path.display(graph, partials));
            self.next_iteration.0.push_back(new_partial_path);
            self.next_iteration.1.push_back(new_cycle_detector);
        }

        extension_count
    }

    /// Returns whether the algorithm has completed.
    pub fn is_complete(&self) -> bool {
        self.queue.is_empty() && self.next_iteration.0.is_empty()
    }

    /// Runs the next phase of the algorithm.  We will have built up a set of incomplete partial
    /// paths during the _previous_ phase.  Before calling this function, you must ensure that `db`
    /// contains all of the possible other partial paths that we might want to extend any of those
    /// candidate partial paths with.
    ///
    /// After this method returns, you can use [`previous_phase_partial_paths`][] to retrieve a
    /// list of the (possibly incomplete) partial paths that were encountered during this phase.
    ///
    /// [`previous_phase_partial_paths`]: #method.previous_phase_partial_paths
    pub fn process_next_phase(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
    ) {
        copious_debugging!("==> Start phase {}", self.phase_number);
        self.queue.extend(
            self.next_iteration
                .0
                .drain(..)
                .zip(self.next_iteration.1.drain(..)),
        );
        let mut work_performed = 0;
        while let Some((partial_path, cycle_detector)) = self.queue.pop_front() {
            copious_debugging!(
                "--> Candidate partial path {}",
                partial_path.display(graph, partials)
            );
            if !(self.should_extend)(graph, partials, &partial_path) {
                copious_debugging!("    Should not extend");
                continue;
            }
            work_performed +=
                self.stitch_partial_path(graph, partials, db, &partial_path, cycle_detector);
            if work_performed >= self.max_work_per_phase {
                break;
            }
        }

        #[cfg(feature = "copious-debugging")]
        {
            copious_debugging!("==> End phase {}", self.phase_number);
            self.phase_number += 1;
        }
    }

    /// Returns all of the complete partial paths that are reachable from a set of starting nodes,
    /// building them up by stitching together partial paths from this database.
    ///
    /// This function will not return until all reachable partial paths have been processed, so
    /// your database must already contain all partial paths that might be needed.  If you have a
    /// very large stack graph stored in some other storage system, and want more control over
    /// lazily loading only the necessary pieces, then you should code up your own loop that calls
    /// [`process_next_phase`][] manually.
    ///
    /// [`process_next_phase`]: #method.process_next_phase
    pub fn find_all_complete_partial_paths<I, F>(
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        starting_nodes: I,
        cancellation_flag: &dyn CancellationFlag,
        mut visit: F,
    ) -> Result<(), CancellationError>
    where
        I: IntoIterator<Item = Handle<Node>>,
        F: FnMut(&StackGraph, &mut PartialPaths, &PartialPath),
    {
        let mut stitcher =
            ForwardPartialPathStitcher::from_nodes(graph, partials, db, starting_nodes);
        stitcher.set_should_extend(|g, _, p| p.starts_at_reference(g));
        while !stitcher.is_complete() {
            cancellation_flag.check("finding complete partial paths")?;
            let complete_partial_paths = stitcher
                .previous_phase_partial_paths()
                .filter(|partial_path| partial_path.is_complete(graph));
            for path in complete_partial_paths {
                visit(graph, partials, path);
            }
            stitcher.process_next_phase(graph, partials, db);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum OwnedOrDatabasePath {
    Db(Handle<PartialPath>),
    Owned(PartialPath),
}

impl OwnedOrDatabasePath {
    pub(crate) fn get<'a>(&'a self, db: &'a Database) -> &'a PartialPath {
        match self {
            Self::Db(path) => &db[*path],
            Self::Owned(path) => path,
        }
    }
}

impl From<Handle<PartialPath>> for OwnedOrDatabasePath {
    fn from(value: Handle<PartialPath>) -> Self {
        Self::Db(value)
    }
}

impl From<&Handle<PartialPath>> for OwnedOrDatabasePath {
    fn from(value: &Handle<PartialPath>) -> Self {
        Self::Db(*value)
    }
}

impl From<PartialPath> for OwnedOrDatabasePath {
    fn from(value: PartialPath) -> Self {
        Self::Owned(value)
    }
}
