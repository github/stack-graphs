// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::arena::Handle;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::serde;
use crate::stitching::Database;
use crate::stitching::ForwardPartialPathStitcher;
use crate::CancellationError;
use crate::CancellationFlag;

pub struct SQLiteWriter {
    graph: StackGraph,
    partials: PartialPaths,
    db: Database,
}

impl SQLiteWriter {
    pub fn new_in_memory() -> Self {
        Self {
            graph: StackGraph::new(),
            partials: PartialPaths::new(),
            db: Database::new(),
        }
    }

    pub fn add_graph(&mut self, graph: &StackGraph) {
        let graph = serde::StackGraph::from_graph(graph);
        graph.load_into(&mut self.graph).expect("TODO");
    }

    pub fn add_partial_path(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        path: &PartialPath,
    ) {
        let path = serde::PartialPath::from_partial_path(graph, partials, path);
        let path = path
            .to_partial_path(&mut self.graph, &mut self.partials)
            .expect("TODO");
        self.db
            .add_partial_path(&self.graph, &mut self.partials, path);
    }

    pub fn into_reader(self) -> SQLiteReader {
        SQLiteReader {
            graph: self.graph,
            partials: self.partials,
            db: self.db,
        }
    }
}

pub struct SQLiteReader {
    graph: StackGraph,
    partials: PartialPaths,
    db: Database,
}

impl SQLiteReader {
    /// Ensure all data for the given file is loaded. If it was already loaded, nothing is done.
    pub fn load_for_file(&mut self, name: &str) {
        self.graph.get_file(name).expect("missing file data");
    }

    pub fn load_for_partial_path(&mut self, path: &PartialPath) {
        let file = match self.graph[path.end_node].file() {
            Some(file) => self.graph[file].name().to_string(),
            None => return,
        };
        self.load_for_file(&file);
    }

    /// Get the stack graph, partial paths arena, and path database for the currently loaded data.
    pub fn get(&mut self) -> (&StackGraph, &mut PartialPaths, &mut Database) {
        (&self.graph, &mut self.partials, &mut self.db)
    }

    /// Find all paths using the given path stitcher. Data is lazily loaded if necessary.
    pub fn find_all_complete_partial_paths<I, F>(
        &mut self,
        starting_nodes: I,
        cancellation_flag: &dyn CancellationFlag,
        mut visit: F,
    ) -> Result<(), CancellationError>
    where
        I: IntoIterator<Item = Handle<Node>>,
        F: FnMut(&StackGraph, &mut PartialPaths, &PartialPath),
    {
        let mut stitcher = ForwardPartialPathStitcher::from_nodes(
            &self.graph,
            &mut self.partials,
            &mut self.db,
            starting_nodes,
        );
        while !stitcher.is_complete() {
            cancellation_flag.check("find_all_complete_partial_paths")?;
            for path in stitcher.previous_phase_partial_paths() {
                self.load_for_partial_path(path);
            }
            stitcher.process_next_phase(&self.graph, &mut self.partials, &mut self.db);
            for path in stitcher.previous_phase_partial_paths() {
                if path.is_complete(&self.graph) {
                    visit(&self.graph, &mut self.partials, path);
                }
            }
        }
        Ok(())
    }
}
