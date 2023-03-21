// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde_json::Value;
use thiserror::Error;

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
