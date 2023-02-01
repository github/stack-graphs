// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines assertions that can be run against a stack graph.

use itertools::Itertools;
use lsp_positions::Position;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::paths::Path;
use crate::paths::Paths;
use crate::CancellationError;
use crate::CancellationFlag;

/// A stack graph assertion
#[derive(Debug, Clone)]
pub enum Assertion {
    Defined {
        source: AssertionSource,
        targets: Vec<AssertionTarget>,
    },
    Defines {
        source: AssertionSource,
        symbols: Vec<Handle<Symbol>>,
    },
    Refers {
        source: AssertionSource,
        symbols: Vec<Handle<Symbol>>,
    },
}

/// Source position of an assertion
#[derive(Debug, Clone)]
pub struct AssertionSource {
    pub file: Handle<File>,
    pub position: Position,
}

impl AssertionSource {
    fn definitions_iter<'a>(
        &'a self,
        graph: &'a StackGraph,
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        graph.nodes_for_file(self.file).filter(move |n| {
            graph[*n].is_definition()
                && graph
                    .source_info(*n)
                    .map(|s| s.span.contains(&self.position))
                    .unwrap_or(false)
        })
    }

    fn references_iter<'a>(
        &'a self,
        graph: &'a StackGraph,
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        graph.nodes_for_file(self.file).filter(move |n| {
            graph[*n].is_reference()
                && graph
                    .source_info(*n)
                    .map(|s| s.span.contains(&self.position))
                    .unwrap_or(false)
        })
    }
}

/// Target line of an assertion
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssertionTarget {
    pub file: Handle<File>,
    pub line: usize,
}

impl AssertionTarget {
    /// Checks if the target matches the node corresponding to the handle in the given graph.
    pub fn matches_node(&self, node: Handle<Node>, graph: &StackGraph) -> bool {
        let file = graph[node].file().unwrap();
        let si = graph.source_info(node).unwrap();
        let start_line = si.span.start.line;
        let end_line = si.span.end.line;
        file == self.file && start_line <= self.line && self.line <= end_line
    }
}

/// Error describing assertion failures.
#[derive(Clone)]
pub enum AssertionError {
    NoReferences {
        source: AssertionSource,
    },
    IncorrectlyDefined {
        source: AssertionSource,
        references: Vec<Handle<Node>>,
        missing_targets: Vec<AssertionTarget>,
        unexpected_paths: Vec<Path>,
    },
    IncorrectDefinitions {
        source: AssertionSource,
        missing_symbols: Vec<Handle<Symbol>>,
        unexpected_symbols: Vec<Handle<Symbol>>,
    },
    IncorrectReferences {
        source: AssertionSource,
        missing_symbols: Vec<Handle<Symbol>>,
        unexpected_symbols: Vec<Handle<Symbol>>,
    },
    Cancelled(CancellationError),
}

impl From<CancellationError> for AssertionError {
    fn from(value: CancellationError) -> Self {
        Self::Cancelled(value)
    }
}

impl Assertion {
    /// Run this assertion against the given graph, using the given paths object for path search.
    pub fn run(
        &self,
        graph: &StackGraph,
        paths: &mut Paths,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), AssertionError> {
        match self {
            Self::Defined { source, targets } => {
                self.run_defined(graph, paths, source, targets, cancellation_flag)
            }
            Self::Defines { source, symbols } => self.run_defines(graph, source, symbols),
            Self::Refers { source, symbols } => self.run_refers(graph, source, symbols),
        }
    }

    fn run_defined(
        &self,
        graph: &StackGraph,
        paths: &mut Paths,
        source: &AssertionSource,
        expected_targets: &Vec<AssertionTarget>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), AssertionError> {
        let references = source.references_iter(graph).collect::<Vec<_>>();
        if references.is_empty() {
            return Err(AssertionError::NoReferences {
                source: source.clone(),
            });
        }

        let mut actual_paths = Vec::new();
        paths.find_all_paths(graph, references.clone(), cancellation_flag, |g, _ps, p| {
            if p.is_complete(g) {
                actual_paths.push(p);
            }
        })?;
        paths.remove_shadowed_paths(&mut actual_paths, cancellation_flag)?;
        let missing_targets = expected_targets
            .iter()
            .filter(|t| {
                !actual_paths
                    .iter()
                    .any(|p| t.matches_node(p.end_node, graph))
            })
            .cloned()
            .unique()
            .collect::<Vec<_>>();
        let unexpected_paths = actual_paths
            .iter()
            .filter(|p| {
                !expected_targets
                    .iter()
                    .any(|t| t.matches_node(p.end_node, graph))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !missing_targets.is_empty() || !unexpected_paths.is_empty() {
            return Err(AssertionError::IncorrectlyDefined {
                source: source.clone(),
                references,
                missing_targets,
                unexpected_paths,
            });
        }

        Ok(())
    }

    fn run_defines(
        &self,
        graph: &StackGraph,
        source: &AssertionSource,
        expected_symbols: &Vec<Handle<Symbol>>,
    ) -> Result<(), AssertionError> {
        let actual_symbols = source
            .definitions_iter(graph)
            .filter_map(|d| graph[d].symbol())
            .collect::<Vec<_>>();
        let missing_symbols = expected_symbols
            .iter()
            .filter(|x| !actual_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();
        let unexpected_symbols = actual_symbols
            .iter()
            .filter(|x| !expected_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();
        if !missing_symbols.is_empty() || !unexpected_symbols.is_empty() {
            return Err(AssertionError::IncorrectDefinitions {
                source: source.clone(),
                missing_symbols,
                unexpected_symbols,
            });
        }
        Ok(())
    }

    fn run_refers(
        &self,
        graph: &StackGraph,
        source: &AssertionSource,
        expected_symbols: &Vec<Handle<Symbol>>,
    ) -> Result<(), AssertionError> {
        let actual_symbols = source
            .references_iter(graph)
            .filter_map(|d| graph[d].symbol())
            .collect::<Vec<_>>();
        let missing_symbols = expected_symbols
            .iter()
            .filter(|x| !actual_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();
        let unexpected_symbols = actual_symbols
            .iter()
            .filter(|x| !expected_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();
        if !missing_symbols.is_empty() || !unexpected_symbols.is_empty() {
            return Err(AssertionError::IncorrectReferences {
                source: source.clone(),
                missing_symbols,
                unexpected_symbols,
            });
        }
        Ok(())
    }
}
