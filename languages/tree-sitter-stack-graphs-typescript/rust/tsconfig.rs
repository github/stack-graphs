// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::path::Path;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::FileAnalyzer;

pub struct TsConfigAnalyzer {}

impl FileAnalyzer for TsConfigAnalyzer {
    fn build_stack_graph_into<'a>(
        &'a self,
        _stack_graph: &'a mut StackGraph,
        _file: Handle<File>,
        _path: &Path,
        _source: &'a str,
        _paths: Vec<&Path>,
        _cancellation_flag: &'a dyn tree_sitter_stack_graphs::CancellationFlag,
    ) -> Result<(), tree_sitter_stack_graphs::LoadError> {
        todo!()
    }
}
