// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::path::Path;

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::FileAnalyzer;

pub struct TsConfigAnalyzer {}

impl FileAnalyzer for TsConfigAnalyzer {
    fn build_stack_graph_into<'a>(
        &self,
        _graph: &mut StackGraph,
        _file: Handle<File>,
        _path: &Path,
        _source: &str,
        _all_paths: &mut dyn Iterator<Item = &'a Path>,
        _globals: &HashMap<String, String>,
        _cancellation_flag: &dyn tree_sitter_stack_graphs::CancellationFlag,
    ) -> Result<(), tree_sitter_stack_graphs::LoadError> {
        todo!()
    }
}
