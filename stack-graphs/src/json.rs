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
use crate::partial::PartialPaths;
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
        Ok(serde_json::to_value(&paths)?)
    }

    pub fn to_string(&mut self) -> Result<String, JsonError> {
        let paths = self.to_partial_path_vec();
        Ok(serde_json::to_string(&paths)?)
    }

    pub fn to_string_pretty(&mut self) -> Result<String, JsonError> {
        let paths = self.to_partial_path_vec();
        Ok(serde_json::to_string_pretty(&paths)?)
    }

    fn to_partial_path_vec(&mut self) -> Vec<crate::serde::PartialPath> {
        let graph = &self.1;
        let partials = &mut self.2;
        let filter = ImplicationFilter(self.3);

        let mut path_vec = Vec::new();
        for h in self.0.iter_partial_paths() {
            let path = &self.0[h];
            if filter.include_partial_path(graph, partials, path) {
                let path = crate::serde::PartialPath::from(graph, partials, path);
                path_vec.push(path);
            }
        }
        path_vec
    }
}
