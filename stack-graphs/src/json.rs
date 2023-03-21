// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde::ser::Serialize;
use serde::ser::SerializeSeq;
use serde::ser::SerializeStruct;
use serde::ser::Serializer;
use serde_json::Value;
use std::ops::Index;
use thiserror::Error;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::NodeID;
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
pub use crate::serde::Filter;
use crate::serde::ImplicationFilter;
pub use crate::serde::NoFilter;
use crate::stitching::Database;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct JsonError(#[from] serde_json::error::Error);

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
// Files

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
