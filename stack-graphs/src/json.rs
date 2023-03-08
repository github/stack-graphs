// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use itertools::Itertools;
use lsp_positions::Offset;
use lsp_positions::Position;
use lsp_positions::Span;
use serde::ser::Serialize;
use serde::ser::SerializeSeq;
use serde::ser::SerializeStruct;
use serde::ser::Serializer;
use serde_json::Value;
use std::ops::Index;
use thiserror::Error;

use crate::arena::Handle;
use crate::graph::DebugEntry;
use crate::graph::DebugInfo;
use crate::graph::Edge;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::NodeID;
use crate::graph::SourceInfo;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPathEdge;
use crate::partial::PartialPathEdgeList;
use crate::partial::PartialPaths;
use crate::partial::PartialScopeStack;
use crate::partial::PartialScopedSymbol;
use crate::partial::PartialSymbolStack;
use crate::partial::ScopeStackVariable;
use crate::partial::SymbolStackVariable;
use crate::stitching::Database;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct JsonError(#[from] serde_json::error::Error);

//-----------------------------------------------------------------------------
// Filter

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

/// Filter implementation that enforces all implications of another filter.
/// For example, that nodes frome excluded files are not included, etc.
struct ImplicationFilter<'a>(&'a dyn Filter);

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

//-----------------------------------------------------------------------------
// InStackGraph

struct InStackGraph<'a, T>(&'a StackGraph, T, &'a dyn Filter);

impl<'a, T> InStackGraph<'a, T> {
    fn with<U>(&'a self, u: U) -> InStackGraph<'a, U> {
        InStackGraph(self.0, u, self.2)
    }

    fn with_idx<Idx: Copy, U: ?Sized>(&'a self, idx: Idx) -> InStackGraph<'a, (Idx, &U)>
    where
        StackGraph: Index<Idx, Output = U>,
    {
        InStackGraph(self.0, (idx, &self.0[idx]), self.2)
    }
}

//-----------------------------------------------------------------------------
// StackGraph

impl<'a> StackGraph {
    pub fn to_json(&'a self, f: &'a dyn Filter) -> JsonStackGraph {
        JsonStackGraph(self, f)
    }

    pub fn to_serializable(&self) -> crate::serde::StackGraph {
        self.to_serializable_filter(&NoFilter)
    }

    pub fn to_serializable_filter(&self, f: &'a dyn Filter) -> crate::serde::StackGraph {
        crate::serde::StackGraph::from_graph_filter(self, f)
    }
}

pub struct JsonStackGraph<'a>(&'a StackGraph, &'a dyn Filter);

impl<'a> JsonStackGraph<'a> {
    pub fn to_value(&self) -> Result<Value, JsonError> {
        Ok(serde_json::to_value(self)?)
    }

    pub fn to_string(&self) -> Result<String, JsonError> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn to_string_pretty(&self) -> Result<String, JsonError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

impl Serialize for JsonStackGraph<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("stack_graph", 2)?;
        ser.serialize_field(
            "files",
            &InStackGraph(self.0, &Files, &ImplicationFilter(self.1)),
        )?;
        ser.serialize_field(
            "nodes",
            &InStackGraph(self.0, &Nodes, &ImplicationFilter(self.1)),
        )?;
        ser.serialize_field(
            "edges",
            &InStackGraph(self.0, &Edges, &ImplicationFilter(self.1)),
        )?;
        ser.end()
    }
}

//-----------------------------------------------------------------------------
// Files

struct Files;

impl<'a> Serialize for InStackGraph<'a, &Files> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let filter = self.2;

        let mut ser = serializer.serialize_seq(None)?;
        for file in graph.iter_files().filter(|f| filter.include_file(graph, f)) {
            ser.serialize_element(&self.with_idx(file))?;
        }
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, (Handle<File>, &File)> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let file = self.1 .1;
        serializer.serialize_str(file.name())
    }
}

//-----------------------------------------------------------------------------
// Nodes

struct Nodes;

impl Serialize for InStackGraph<'_, &Nodes> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let filter = self.2;

        let mut nodes = serializer.serialize_seq(None)?;
        for node in graph
            .iter_nodes()
            .filter(|n| filter.include_node(graph, &n))
        {
            nodes.serialize_element(&self.with_idx(node))?;
        }
        nodes.end()
    }
}

impl Serialize for InStackGraph<'_, (Handle<Node>, &Node)> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let handle = self.1 .0;
        let node = self.1 .1;
        let source_info = graph.source_info(handle);
        let debug_info = graph.debug_info(handle);

        let mut len = 2;
        if source_info.is_some() {
            len += 1;
        }
        if debug_info.is_some() {
            len += 1;
        }

        let mut ser = match node {
            Node::DropScopes(_node) => {
                let mut ser = serializer.serialize_struct("node", len + 1)?;
                ser.serialize_field("type", "drop_scopes")?;
                ser
            }
            Node::JumpTo(_node) => {
                let mut ser = serializer.serialize_struct("node", len + 1)?;
                ser.serialize_field("type", "jump_to_scope")?;
                ser
            }
            Node::PopScopedSymbol(node) => {
                let mut ser = serializer.serialize_struct("node", len + 3)?;
                ser.serialize_field("type", "pop_scoped_symbol")?;
                ser.serialize_field("symbol", &graph[node.symbol])?;
                ser.serialize_field("is_definition", &node.is_definition)?;
                ser
            }
            Node::PopSymbol(node) => {
                let mut ser = serializer.serialize_struct("node", len + 3)?;
                ser.serialize_field("type", "pop_symbol")?;
                ser.serialize_field("symbol", &graph[node.symbol])?;
                ser.serialize_field("is_definition", &node.is_definition)?;
                ser
            }
            Node::PushScopedSymbol(node) => {
                let mut ser = serializer.serialize_struct("node", len + 4)?;
                ser.serialize_field("type", "push_scoped_symbol")?;
                ser.serialize_field("symbol", &graph[node.symbol])?;
                ser.serialize_field("scope", &self.with(&node.scope))?;
                ser.serialize_field("is_reference", &node.is_reference)?;
                ser
            }
            Node::PushSymbol(node) => {
                let mut ser = serializer.serialize_struct("node", len + 3)?;
                ser.serialize_field("type", "push_symbol")?;
                ser.serialize_field("symbol", &graph[node.symbol])?;
                ser.serialize_field("is_reference", &node.is_reference)?;
                ser
            }
            Node::Root(_node) => {
                let mut ser = serializer.serialize_struct("node", len + 1)?;
                ser.serialize_field("type", "root")?;
                ser
            }
            Node::Scope(node) => {
                let mut ser = serializer.serialize_struct("node", len + 2)?;
                ser.serialize_field("type", "scope")?;
                ser.serialize_field("is_exported", &node.is_exported)?;
                ser
            }
        };

        ser.serialize_field("id", &self.with(&node.id()))?;
        if let Some(source_info) = source_info {
            ser.serialize_field("source_info", &self.with(source_info))?;
        }
        if let Some(debug_info) = debug_info {
            ser.serialize_field("debug_info", &self.with(debug_info))?;
        }

        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &Vec<NodeID>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let nodes = self.1;

        let mut ser = serializer.serialize_seq(nodes.len().into())?;
        for node in nodes {
            ser.serialize_element(&self.with(node))?;
        }
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &NodeID> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let node_id = self.1;

        let len = 1 + node_id.file().map(|_| 1).unwrap_or(0);
        let mut ser = serializer.serialize_struct("node_id", len)?;
        if let Some(file) = node_id.file() {
            ser.serialize_field("file", &self.with_idx(file))?;
        }
        ser.serialize_field("local_id", &node_id.local_id())?;
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &SourceInfo> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let source_info = self.1;

        let mut len = 1;
        if source_info.syntax_type.is_some() {
            len += 1;
        }

        let mut ser = serializer.serialize_struct("source_info", len)?;
        ser.serialize_field("span", &self.with(&source_info.span))?;
        if let Some(syntax_type) = source_info.syntax_type {
            ser.serialize_field("syntax_type", &graph[syntax_type])?;
        }
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &DebugInfo> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let debug_info = self.1;

        let mut ser = serializer.serialize_seq(None)?;
        for entry in debug_info.iter() {
            ser.serialize_element(&self.with(entry))?;
        }
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &DebugEntry> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let debug_entry = self.1;

        let mut ser = serializer.serialize_struct("debug_entry", 2)?;
        ser.serialize_field("key", &graph[debug_entry.key])?;
        ser.serialize_field("value", &graph[debug_entry.value])?;
        ser.end()
    }
}

//-----------------------------------------------------------------------------
// Edges

struct Edges;

impl Serialize for InStackGraph<'_, &Edges> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let filter = self.2;

        let mut ser = serializer.serialize_seq(None)?;
        for source in graph.iter_nodes() {
            for edge in graph
                .outgoing_edges(source)
                .filter(|e| filter.include_edge(graph, &e.source, &e.sink))
            {
                ser.serialize_element(&self.with(&edge))?;
            }
        }
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &Edge> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let edge = self.1;

        let mut ser = serializer.serialize_struct("edge", 3)?;
        ser.serialize_field("source", &self.with(&graph[edge.source].id()))?;
        ser.serialize_field("sink", &self.with(&graph[edge.sink].id()))?;
        ser.serialize_field("precedence", &edge.precedence)?;
        ser.end()
    }
}

//-----------------------------------------------------------------------------
// Span, Position, Offset

impl Serialize for InStackGraph<'_, &Span> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let span = self.1;

        let mut ser = serializer.serialize_struct("span", 2)?;
        ser.serialize_field("start", &self.with(&span.start))?;
        ser.serialize_field("end", &self.with(&span.end))?;
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &Position> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let position = self.1;

        let mut ser = serializer.serialize_struct("position", 2)?;
        ser.serialize_field("line", &position.line)?;
        ser.serialize_field("column", &self.with(&position.column))?;
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &Offset> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let offset = self.1;

        let mut ser = serializer.serialize_struct("offset", 3)?;
        ser.serialize_field("utf8_offset", &offset.utf8_offset)?;
        ser.serialize_field("utf16_offset", &offset.utf16_offset)?;
        ser.serialize_field("grapheme_offset", &offset.grapheme_offset)?;
        ser.end()
    }
}

//-----------------------------------------------------------------------------
// Database

impl<'a> Database {
    pub fn to_json(
        &'a mut self,
        graph: &'a StackGraph,
        partials: &'a mut PartialPaths,
        f: &'a dyn Filter,
    ) -> JsonDatabase<'_> {
        JsonDatabase(self, graph, partials, f)
    }
}

pub struct JsonDatabase<'a>(
    &'a mut Database,
    &'a StackGraph,
    &'a mut PartialPaths,
    &'a dyn Filter,
);

impl<'a> JsonDatabase<'a> {
    pub fn to_value(&mut self) -> Result<Value, JsonError> {
        let paths = self.to_partial_path_vec();
        Ok(serde_json::to_value(&InPartialPaths(
            self.1, self.2, &paths,
        ))?)
    }

    pub fn to_string(&mut self) -> Result<String, JsonError> {
        let paths = self.to_partial_path_vec();
        Ok(serde_json::to_string(&InPartialPaths(
            self.1, self.2, &paths,
        ))?)
    }

    pub fn to_string_pretty(&mut self) -> Result<String, JsonError> {
        let paths = self.to_partial_path_vec();
        Ok(serde_json::to_string_pretty(&InPartialPaths(
            self.1, self.2, &paths,
        ))?)
    }

    fn to_partial_path_vec(&mut self) -> Vec<PartialPath> {
        let graph = self.1;
        let filter = ImplicationFilter(self.3);

        let mut path_vec = Vec::new();
        for h in self.0.iter_partial_paths() {
            let path = &self.0[h];
            if filter.include_partial_path(graph, self.2, path) {
                let mut path = path.clone();
                path.ensure_forwards(self.2);
                path_vec.push(path);
            }
        }
        path_vec
    }
}

//-----------------------------------------------------------------------------
// PartialPaths

struct InPartialPaths<'a, T>(&'a StackGraph, &'a PartialPaths, T);

impl<'a, T> InPartialPaths<'a, T> {
    fn with<U>(&'a self, u: U) -> InPartialPaths<'a, U> {
        InPartialPaths(self.0, self.1, u)
    }

    fn in_stack_graph(&'a self) -> InStackGraph<'a, T>
    where
        T: Copy,
    {
        InStackGraph(self.0, self.2, &NoFilter)
    }
}

impl Serialize for InPartialPaths<'_, &Vec<PartialPath>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let paths = self.2;

        let mut ser = serializer.serialize_seq(paths.len().into())?;
        for path in paths {
            ser.serialize_element(&self.with(path))?;
        }
        ser.end()
    }
}

impl Serialize for InPartialPaths<'_, &PartialPath> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let path = self.2;

        let mut ser = serializer.serialize_struct("partial_path", 7)?;
        ser.serialize_field(
            "start_node",
            &self.in_stack_graph().with(&graph[path.start_node].id()),
        )?;
        ser.serialize_field(
            "end_node",
            &self.in_stack_graph().with(&graph[path.end_node].id()),
        )?;
        ser.serialize_field(
            "symbol_stack_precondition",
            &self.with(&path.symbol_stack_precondition),
        )?;
        ser.serialize_field(
            "scope_stack_precondition",
            &self.with(&path.scope_stack_precondition),
        )?;
        ser.serialize_field(
            "symbol_stack_postcondition",
            &self.with(&path.symbol_stack_postcondition),
        )?;
        ser.serialize_field(
            "scope_stack_postcondition",
            &self.with(&path.scope_stack_postcondition),
        )?;
        ser.serialize_field("edges", &self.with(&path.edges))?;
        ser.end()
    }
}

impl Serialize for InPartialPaths<'_, &PartialSymbolStack> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let paths = self.1;
        let symbol_stack = self.2;

        let mut len = 1;
        if symbol_stack.has_variable() {
            len += 1;
        }
        let symbols = symbol_stack.iter_unordered(paths).collect::<Vec<_>>();

        let mut ser = serializer.serialize_struct("partial_symbol_stack", len)?;
        ser.serialize_field("symbols", &self.with(&symbols))?;
        if let Some(variable) = symbol_stack.variable() {
            ser.serialize_field("variable", &variable)?;
        }
        ser.end()
    }
}

impl Serialize for InPartialPaths<'_, &Vec<PartialScopedSymbol>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let symbols = self.2;

        let mut ser = serializer.serialize_seq(symbols.len().into())?;
        for scoped_symbol in symbols {
            ser.serialize_element(&self.with(scoped_symbol))?;
        }
        ser.end()
    }
}

impl Serialize for InPartialPaths<'_, &PartialScopedSymbol> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let scoped_symbol = self.2;

        let mut len = 1;
        if scoped_symbol.scopes.is_some() {
            len += 1;
        }

        let mut ser = serializer.serialize_struct("partial_scoped_symbol", len)?;
        ser.serialize_field("symbol", &graph[scoped_symbol.symbol])?;
        if let Some(scopes) = scoped_symbol.scopes.into_option() {
            ser.serialize_field("scopes", &self.with(&scopes))?;
        }
        ser.end()
    }
}

impl Serialize for SymbolStackVariable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.as_u32())
    }
}

impl Serialize for InPartialPaths<'_, &PartialScopeStack> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.0;
        let paths = self.1;
        let scope_stack = self.2;

        let mut len = 1;
        if scope_stack.has_variable() {
            len += 1;
        }
        let scopes = scope_stack
            .iter_unordered(paths)
            .map(|n| graph[n].id())
            .collect::<Vec<_>>();

        let mut ser = serializer.serialize_struct("partial_scope_stack", len)?;
        ser.serialize_field("scopes", &self.in_stack_graph().with(&scopes))?;
        if let Some(variable) = scope_stack.variable() {
            ser.serialize_field("variable", &variable)?;
        }
        ser.end()
    }
}

impl Serialize for ScopeStackVariable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.as_u32())
    }
}

impl Serialize for InPartialPaths<'_, &PartialPathEdgeList> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let paths = self.1;
        let edge_list = self.2;

        let mut ser = serializer.serialize_seq(edge_list.len().into())?;
        for edge in edge_list.iter_unordered(paths) {
            ser.serialize_element(&self.with(&edge))?;
        }
        ser.end()
    }
}

impl Serialize for InPartialPaths<'_, &PartialPathEdge> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let edge = self.2;

        let mut ser = serializer.serialize_struct("partial_path_edge", 2)?;
        ser.serialize_field("source", &self.in_stack_graph().with(&edge.source_node_id))?;
        ser.serialize_field("precedence", &edge.precedence)?;
        ser.end()
    }
}
