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
use std::ops::Index;

use crate::arena::Arena;
use crate::arena::Handle;
use crate::arena::List;
use crate::arena::ListArena;
use crate::arena::ListCell;
use crate::arena::SupplementalArena;
use crate::cycles::CycleDetector;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::partial::PartialSymbolStack;
use crate::paths::Path;
use crate::paths::Paths;
use crate::paths::SymbolStack;

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
    partial_paths: Arena<PartialPath>,
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
        let symbol_stack_precondition = path.symbol_stack_precondition;
        let handle = self.partial_paths.add(path);

        // If the partial path starts at the root node, index it by its symbol stack precondition.
        if graph[start_node].is_root() {
            let key = SymbolStackKey::from_partial_symbol_stack(
                partials,
                self,
                symbol_stack_precondition,
            );
            let key_handle = key.back_handle();
            self.root_paths_by_precondition[key_handle].push(handle);
        } else {
            // Otherwise index it by its source node.
            self.paths_by_start_node[start_node].push(handle);
        }

        handle
    }

    /// Find all partial paths in this database that start at the root node, and have a symbol
    /// stack precondition that is compatible with a given symbol stack.
    pub fn find_candidate_partial_paths_from_root<R>(
        &mut self,
        paths: &mut Paths,
        symbol_stack: SymbolStack,
        result: &mut R,
    ) where
        R: std::iter::Extend<Handle<PartialPath>>,
    {
        // If the path currently ends at the root node, then we need to look up partial paths whose
        // symbol stack precondition is compatible with the path.
        let mut symbol_stack = SymbolStackKey::from_symbol_stack(paths, self, symbol_stack);
        loop {
            let key_handle = symbol_stack.back_handle();
            if let Some(paths) = self.root_paths_by_precondition.get(key_handle) {
                result.extend(paths.iter().copied());
            }
            if symbol_stack.pop_back(self).is_none() {
                break;
            }
        }
    }

    /// Find all partial paths in the database that start at the given node.  We don't filter the
    /// results any further than that, since we have to check each partial path for compatibility
    /// as we try to append it to the current incomplete path anyway, and non-root nodes will
    /// typically have a small number of outgoing edges.
    pub fn find_candidate_partial_paths_from_node<R>(
        &self,
        start_node: Handle<Node>,
        result: &mut R,
    ) where
        R: std::iter::Extend<Handle<PartialPath>>,
    {
        // Return all of the partial paths that start at the requested node.
        if let Some(paths) = self.paths_by_start_node.get(start_node) {
            result.extend(paths.iter().copied());
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

#[derive(Clone, Copy)]
struct SymbolStackKey {
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
    fn from_symbol_stack(
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
    fn from_partial_symbol_stack(
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
    queue: VecDeque<Path>,
    next_iteration: VecDeque<Path>,
    cycle_detector: CycleDetector<Path>,
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
        let mut candidate_paths = Vec::new();
        for node in starting_nodes {
            db.find_candidate_partial_paths_from_node(node, &mut candidate_paths);
        }
        let next_iteration = candidate_paths
            .iter()
            .filter_map(|partial_path| {
                Path::from_partial_path(graph, paths, partials, &db[*partial_path])
            })
            .collect();
        PathStitcher {
            candidate_paths,
            queue: VecDeque::new(),
            next_iteration,
            cycle_detector: CycleDetector::new(),
        }
    }

    /// Returns an iterator of all of the (possibly incomplete) paths that were encountered during
    /// the most recent phase of the path-stitching algorithm.
    pub fn previous_phase_paths(&self) -> impl Iterator<Item = &Path> + '_ {
        self.next_iteration.iter()
    }

    /// Returns a slice of all of the (possibly incomplete) paths that were encountered during the
    /// most recent phase of the path-stitching algorithm.
    pub fn previous_phase_paths_slice(&mut self) -> &[Path] {
        self.next_iteration.make_contiguous();
        self.next_iteration.as_slices().0
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
    ) {
        self.candidate_paths.clear();
        if graph[path.end_node].is_root() {
            db.find_candidate_partial_paths_from_root(
                paths,
                path.symbol_stack,
                &mut self.candidate_paths,
            );
        } else {
            db.find_candidate_partial_paths_from_node(path.end_node, &mut self.candidate_paths);
        }

        self.next_iteration.reserve(self.candidate_paths.len());
        for extension in &self.candidate_paths {
            let mut new_path = path.clone();
            // If there are errors adding this partial path to the path, or resolving the resulting
            // path, just skip the partial path — it's not a fatal error.
            if new_path
                .append_partial_path(graph, paths, partials, &db[*extension])
                .is_err()
            {
                continue;
            }
            if new_path.resolve(graph, paths).is_err() {
                continue;
            }
            self.next_iteration.push_back(new_path);
        }
    }

    /// Returns whether the path-stitching algorithm has completed.
    pub fn is_complete(&self) -> bool {
        self.next_iteration.is_empty()
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
        std::mem::swap(&mut self.queue, &mut self.next_iteration);
        while let Some(path) = self.queue.pop_front() {
            if !self
                .cycle_detector
                .should_process_path(&path, |probe| probe.cmp(graph, paths, &path))
            {
                continue;
            }
            self.stitch_path(graph, paths, partials, db, &path);
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
    ) -> Vec<Path>
    where
        I: IntoIterator<Item = Handle<Node>>,
    {
        let mut result = Vec::new();
        let mut stitcher = PathStitcher::new(graph, paths, partials, db, starting_nodes);
        while !stitcher.is_complete() {
            let complete_paths = stitcher
                .previous_phase_paths()
                .filter(|path| path.is_complete(graph));
            result.extend(complete_paths.cloned());
            stitcher.process_next_phase(graph, paths, partials, db);
        }
        result
    }
}
