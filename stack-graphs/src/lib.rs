// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Stack graphs provide a single framework for performing name resolution for any programming
//! language, while abstracting away the specific name resolution rules for each of those
//! languages. The basic idea is to represent the _definitions_ and _references_ in a program using
//! graph.  A _name binding_ maps a reference to all of the possible definitions that the reference
//! could refer to.  Because we’ve represented definitions and references as a graph, bindings are
//! represented by paths within that graph.
//!
//! While searching for a path in an incremental stack graph, we keep track of two stacks: a
//! _symbol stack_ and a _scope stack_. Broadly speaking, the symbol stack keeps track of what
//! symbols we’re trying to resolve, while the scope stack gives us control over which particular
//! scopes we look for those symbols in.
//!
//! ## Relationship to scope graphs
//!
//! Stack graphs are based in the [scope graphs][] formalism from Eelco Visser's group at TU Delft.
//! Scope graphs also model the name binding structure of a program using a graph, and uses paths
//! within that graph to represent which definitions a reference might refer to.
//!
//! [scope graphs]: https://pl.ewi.tudelft.nl/research/projects/scope-graphs/
//!
//! Stack graphs add _incrementality_ to scope graphs.  An incremental analysis is one where we
//! can reuse analysis results for source code files that haven't changed.  In a non-incremental
//! analysis, we have to reanalyze the entire contents of a repository or package when _any_ file
//! is changed.
//!
//! As one example, the [_Scopes as Types_][] paper presents some rewrite rules that let you handle
//! field access in a record or object type — for instance, being able to resolve the `foo` part of
//! `A.foo`. In _Scopes as Types_, we’d handle this by performing the path-finding algorithm _when
//! constructing the graph_, to find out what `A` resolves to. Once we find its definition, we then
//! attach the reference to `foo` to that result.
//!
//! [_Scopes as Types_]: https://eelcovisser.org/blog/video/paper/2018/10/27/scopes-as-types/
//!
//! This is not incremental because the reference to `foo` “lives” over in some unrelated part of
//! the graph.  If `A` is defined in a different file (or worse, a different package), then we have
//! to update that distant part of the graph with the node representing the reference.  We end up
//! cluttering the graph for a class definition with nodes representing _all_ of its field
//! references, which is especially problematic for popular packages with lots of downstream
//! dependencies.
//!
//! Our key insight is to recognize that when resolving `A.foo`, we have to “pause” the resolution
//! of `foo` to start resolving `A`.  Once we’ve found the binding for `A`, we can resume the
//! original resolution of `foo`.  This describes a stack!  So instead of having our path-finding
//! algorithm look for exactly one symbol, we can keep track of a _stack_ of things to look for.
//! To correctly resolve `foo`, we do still have to eventually make our way over to the part of the
//! graph where `A` is defined.  But by having a stack of path-finding targets, we can resolve the
//! reference to `A.foo` with a _single_ execution of the path-finding algorithm.  And most
//! importantly, each “chunk” of the overall graph only depends on “local” information from the
//! original source file.  (a.k.a., it’s incremental!)

use std::time::{Duration, Instant};

use thiserror::Error;

pub mod arena;
pub mod assert;
pub mod c;
pub mod cycles;
#[macro_use]
mod debugging;
pub mod graph;
pub mod partial;
pub mod paths;
pub mod serde;
pub mod stats;
pub mod stitching;
#[cfg(feature = "storage")]
pub mod storage;
pub(crate) mod utils;
#[cfg(feature = "visualization")]
pub mod visualization;

/// Trait to signal that the execution is cancelled
pub trait CancellationFlag {
    fn check(&self, at: &'static str) -> Result<(), CancellationError>;
}

pub struct NoCancellation;
impl CancellationFlag for NoCancellation {
    fn check(&self, _at: &'static str) -> Result<(), CancellationError> {
        Ok(())
    }
}

pub struct CancelAfterDuration {
    limit: Duration,
    start: Instant,
}

impl CancelAfterDuration {
    pub fn new(limit: Duration) -> Self {
        Self {
            limit,
            start: Instant::now(),
        }
    }
}

impl CancellationFlag for CancelAfterDuration {
    fn check(&self, at: &'static str) -> Result<(), CancellationError> {
        if self.start.elapsed() > self.limit {
            return Err(CancellationError(at));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Error)]
#[error("Cancelled at \"{0}\"")]
pub struct CancellationError(pub &'static str);
