// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::graph::StackGraph;
use crate::partial::PartialPaths;

use super::Error;
use super::Filter;
use super::ImplicationFilter;
use super::NoFilter;
use super::PartialPath;

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
pub struct Database {
    paths: Vec<PartialPath>,
}

impl Database {
    pub fn from_database(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::stitching::Database,
    ) -> Self {
        Self::from_database_filter(graph, partials, value, &NoFilter)
    }

    pub fn from_database_filter(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::stitching::Database,
        filter: &dyn Filter,
    ) -> Self {
        let filter = ImplicationFilter(filter);
        let mut paths = Vec::new();
        for path in value.iter_partial_paths() {
            let path = &value[path];
            if !filter.include_partial_path(graph, partials, path) {
                continue;
            }
            let path = PartialPath::from_partial_path(graph, partials, &path);
            paths.push(path);
        }
        Self { paths }
    }

    pub fn load_into(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &mut crate::stitching::Database,
    ) -> Result<(), Error> {
        for path in &self.paths {
            let path = path.to_partial_path(graph, partials)?;
            value.add_partial_path(graph, partials, path);
        }
        Ok(())
    }
}

impl crate::stitching::Database {
    pub fn to_serializable(&self, graph: &StackGraph, partials: &mut PartialPaths) -> Database {
        Database::from_database(graph, partials, self)
    }

    pub fn to_serializable_filter(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        filter: &dyn Filter,
    ) -> Database {
        Database::from_database_filter(graph, partials, self, filter)
    }
}
