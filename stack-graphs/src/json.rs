// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

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
use crate::graph::Edge;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::NodeID;
use crate::graph::SourceInfo;
use crate::graph::StackGraph;
use crate::paths::Path;
use crate::paths::PathEdge;
use crate::paths::PathEdgeList;
use crate::paths::Paths;
use crate::paths::ScopeStack;
use crate::paths::ScopedSymbol;
use crate::paths::SymbolStack;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct JsonError(#[from] serde_json::error::Error);

//-----------------------------------------------------------------------------
// InStackGraph

struct InStackGraph<'a, T>(T, &'a StackGraph);

impl<'a, T> InStackGraph<'a, T> {
    fn with<U>(&'a self, u: U) -> InStackGraph<'a, U> {
        InStackGraph(u, self.1)
    }

    fn with_idx<Idx: Copy, U: ?Sized>(&'a self, idx: Idx) -> InStackGraph<'a, (Idx, &U)>
    where
        StackGraph: Index<Idx, Output = U>,
    {
        InStackGraph((idx, &self.1[idx]), self.1)
    }
}

//-----------------------------------------------------------------------------
// StackGraph

impl StackGraph {
    pub fn to_json(&self) -> JsonStackGraph {
        JsonStackGraph(self)
    }
}

pub struct JsonStackGraph<'a>(&'a StackGraph);

impl<'a> JsonStackGraph<'a> {
    pub fn to_value(&self) -> Result<Value, JsonError> {
        Ok(serde_json::to_value(self.0)?)
    }

    pub fn to_string(&self) -> Result<String, JsonError> {
        Ok(serde_json::to_string(self.0)?)
    }

    pub fn to_string_pretty(&self) -> Result<String, JsonError> {
        Ok(serde_json::to_string_pretty(self.0)?)
    }
}

impl Serialize for StackGraph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("stack_graph", 2)?;
        ser.serialize_field("files", &InStackGraph(&Files, self))?;
        ser.serialize_field("nodes", &InStackGraph(&Nodes, self))?;
        ser.serialize_field("edges", &InStackGraph(&Edges, self))?;
        ser.end()
    }
}

//-----------------------------------------------------------------------------
// Files

struct Files;

impl Serialize for InStackGraph<'_, &Files> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let graph = self.1;

        let mut ser = serializer.serialize_seq(None)?;
        for file in graph.iter_files() {
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
        let file = self.0 .1;
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
        let mut nodes = serializer.serialize_seq(None)?;
        for node in self.1.iter_nodes() {
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
        let graph = self.1;
        let handle = self.0 .0;
        let node = self.0 .1;
        let source_info = graph.source_info(handle);

        let mut len = 2;
        if source_info.is_some() {
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

        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &NodeID> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let node_id = self.0;

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
        let graph = self.1;
        let source_info = self.0;

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

//-----------------------------------------------------------------------------
// Edges

struct Edges;

impl Serialize for InStackGraph<'_, &Edges> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_seq(None)?;
        for source in self.1.iter_nodes() {
            for edge in self.1.outgoing_edges(source) {
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
        let graph = self.1;
        let edge = self.0;

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
        let mut ser = serializer.serialize_struct("span", 2)?;
        ser.serialize_field("start", &self.with(&self.0.start))?;
        ser.serialize_field("end", &self.with(&self.0.end))?;
        ser.end()
    }
}

impl Serialize for InStackGraph<'_, &Position> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let position = self.0;

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
        let offset = self.0;

        let mut ser = serializer.serialize_struct("offset", 3)?;
        ser.serialize_field("utf8_offset", &offset.utf8_offset)?;
        ser.serialize_field("utf16_offset", &offset.utf16_offset)?;
        ser.serialize_field("grapheme_offset", &offset.grapheme_offset)?;
        ser.end()
    }
}

//-----------------------------------------------------------------------------
// InPaths

struct InPaths<'a, T>(T, &'a Paths, &'a StackGraph);

impl<'a, T> InPaths<'a, T> {
    fn with<U>(&'a self, u: U) -> InPaths<'a, U> {
        InPaths(u, self.1, self.2)
    }

    fn in_stack_graph(&'a self) -> InStackGraph<'a, T>
    where
        T: Copy,
    {
        InStackGraph(self.0, self.2)
    }
}

//-----------------------------------------------------------------------------
// Paths

impl<'a> Paths {
    pub fn to_json<F>(&'a mut self, graph: &'a StackGraph, f: F) -> JsonPaths<'_, F>
    where
        F: Fn(&StackGraph, &Paths, &Path) -> bool,
    {
        JsonPaths(self, graph, f)
    }
}

pub struct JsonPaths<'a, F>(&'a mut Paths, &'a StackGraph, F)
where
    F: Fn(&StackGraph, &Paths, &Path) -> bool;

impl<'a, F> JsonPaths<'a, F>
where
    F: Fn(&StackGraph, &Paths, &Path) -> bool,
{
    pub fn to_value(&mut self) -> Result<Value, JsonError> {
        let paths = self.to_path_vec();
        Ok(serde_json::to_value(&InPaths(&paths, self.0, self.1))?)
    }

    pub fn to_string(&mut self) -> Result<String, JsonError> {
        let paths = self.to_path_vec();
        Ok(serde_json::to_string(&InPaths(&paths, self.0, self.1))?)
    }

    pub fn to_string_pretty(&mut self) -> Result<String, JsonError> {
        let paths = self.to_path_vec();
        Ok(serde_json::to_string_pretty(&InPaths(
            &paths, self.0, self.1,
        ))?)
    }

    fn to_path_vec(&mut self) -> Vec<Path> {
        let mut paths = Vec::new();
        let f = &self.2;
        self.0
            .find_all_paths(self.1, self.1.iter_nodes(), |g, ps, p| {
                if f(g, ps, &p) {
                    let mut p = p;
                    p.edges.ensure_forwards(ps);
                    paths.push(p);
                }
            });
        paths
    }
}

impl Serialize for InPaths<'_, &Vec<Path>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let paths = self.0;

        let mut ser = serializer.serialize_seq(paths.len().into())?;
        for path in paths {
            ser.serialize_element(&self.with(path))?;
        }
        ser.end()
    }
}

impl Serialize for InPaths<'_, &Path> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let path = self.0;
        let graph = self.2;

        let mut ser = serializer.serialize_struct("path", 5)?;
        ser.serialize_field(
            "start_node",
            &self.in_stack_graph().with(&graph[path.start_node].id()),
        )?;
        ser.serialize_field(
            "end_node",
            &self.in_stack_graph().with(&graph[path.end_node].id()),
        )?;
        ser.serialize_field("symbol_stack", &self.with(&path.symbol_stack))?;
        ser.serialize_field("scope_stack", &self.with(&path.scope_stack))?;
        ser.serialize_field("edges", &self.with(&path.edges))?;
        ser.end()
    }
}

impl Serialize for InPaths<'_, &SymbolStack> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let symbol_stack = self.0;
        let paths = self.1;

        let mut ser = serializer.serialize_seq(symbol_stack.len().into())?;
        for scoped_symbol in symbol_stack.iter(paths) {
            ser.serialize_element(&self.with(&scoped_symbol))?;
        }
        ser.end()
    }
}

impl Serialize for InPaths<'_, &ScopedSymbol> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let scoped_symbol = self.0;
        let graph = self.2;

        let mut len = 1;
        if scoped_symbol.scopes.is_some() {
            len += 1;
        }

        let mut ser = serializer.serialize_struct("scoped_symbol", len)?;
        ser.serialize_field("symbol", &graph[scoped_symbol.symbol])?;
        if let Some(scopes) = scoped_symbol.scopes.into_option() {
            ser.serialize_field("scopes", &self.with(&scopes))?;
        }
        ser.end()
    }
}

impl Serialize for InPaths<'_, &ScopeStack> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let scope_stack = self.0;
        let paths = self.1;
        let graph = self.2;

        let mut ser = serializer.serialize_seq(scope_stack.len().into())?;
        for node in scope_stack.iter(paths) {
            ser.serialize_element(&self.in_stack_graph().with(&graph[node].id()))?;
        }
        ser.end()
    }
}

impl Serialize for InPaths<'_, &PathEdgeList> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let edge_list = self.0;
        let paths = self.1;

        let mut ser = serializer.serialize_seq(edge_list.len().into())?;
        for edge in edge_list.iter_unordered(paths) {
            ser.serialize_element(&self.with(&edge))?;
        }
        ser.end()
    }
}

impl Serialize for InPaths<'_, &PathEdge> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let edge = self.0;

        let mut ser = serializer.serialize_struct("path_edge", 2)?;
        ser.serialize_field("source", &self.in_stack_graph().with(&edge.source_node_id))?;
        ser.serialize_field("precedence", &edge.precedence)?;
        ser.end()
    }
}
