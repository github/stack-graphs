// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use itertools::Itertools;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;

pub trait Filter {
    /// Return whether elements for the given file must be included.
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool;

    /// Return whether the given node must be included.
    /// Nodes of excluded files are always excluded.
    fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool;

    /// Return whether the given edge must be included.
    /// Edges via excluded nodes are always excluded.
    fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool;

    /// Return whether the given path must be included.
    /// Paths via excluded nodes or edges are always excluded.
    fn include_partial_path(
        &self,
        graph: &StackGraph,
        paths: &PartialPaths,
        path: &PartialPath,
    ) -> bool;
}

impl<F> Filter for F
where
    F: Fn(&StackGraph, &Handle<File>) -> bool,
{
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
        self(graph, file)
    }

    fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
        true
    }

    fn include_edge(
        &self,
        _graph: &StackGraph,
        _source: &Handle<Node>,
        _sink: &Handle<Node>,
    ) -> bool {
        true
    }

    fn include_partial_path(
        &self,
        _graph: &StackGraph,
        _paths: &PartialPaths,
        _path: &PartialPath,
    ) -> bool {
        true
    }
}

// Filter implementation that includes everything.
pub struct NoFilter;

impl Filter for NoFilter {
    fn include_file(&self, _graph: &StackGraph, _file: &Handle<File>) -> bool {
        true
    }

    fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
        true
    }

    fn include_edge(
        &self,
        _graph: &StackGraph,
        _source: &Handle<Node>,
        _sink: &Handle<Node>,
    ) -> bool {
        true
    }

    fn include_partial_path(
        &self,
        _graph: &StackGraph,
        _paths: &PartialPaths,
        _path: &PartialPath,
    ) -> bool {
        true
    }
}

/// Filter implementation that includes a single file.
pub struct FileFilter(pub Handle<File>);

impl Filter for FileFilter {
    fn include_file(&self, _graph: &StackGraph, file: &Handle<File>) -> bool {
        *file == self.0
    }

    fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
        true
    }

    fn include_edge(
        &self,
        _graph: &StackGraph,
        _source: &Handle<Node>,
        _sink: &Handle<Node>,
    ) -> bool {
        true
    }

    fn include_partial_path(
        &self,
        _graph: &StackGraph,
        _paths: &PartialPaths,
        _path: &PartialPath,
    ) -> bool {
        true
    }
}

/// Filter implementation that enforces all implications of another filter.
/// For example, that nodes frome excluded files are not included, etc.
pub(crate) struct ImplicationFilter<'a>(pub &'a dyn Filter);

impl Filter for ImplicationFilter<'_> {
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
        self.0.include_file(graph, file)
    }

    fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool {
        graph[*node]
            .id()
            .file()
            .map_or(true, |f| self.include_file(graph, &f))
            && self.0.include_node(graph, node)
    }

    fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool {
        self.include_node(graph, source)
            && self.include_node(graph, sink)
            && self.0.include_edge(graph, source, sink)
    }

    fn include_partial_path(
        &self,
        graph: &StackGraph,
        paths: &PartialPaths,
        path: &PartialPath,
    ) -> bool {
        let super_ok = self.0.include_partial_path(graph, paths, path);
        if !super_ok {
            return false;
        }
        let all_included_edges = path
            .edges
            .iter_unordered(paths)
            .map(|e| graph.node_for_id(e.source_node_id).unwrap())
            .chain(std::iter::once(path.end_node))
            .tuple_windows()
            .all(|(source, sink)| self.include_edge(graph, &source, &sink));
        if !all_included_edges {
            return false;
        }
        true
    }
}
