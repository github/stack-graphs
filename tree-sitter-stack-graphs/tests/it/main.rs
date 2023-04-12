// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::BuildError;
use tree_sitter_stack_graphs::NoCancellation;
use tree_sitter_stack_graphs::StackGraphLanguage;

mod builder;
mod edges;
mod loader;
mod nodes;
mod test;

pub(self) fn build_stack_graph(
    python_source: &str,
    tsg_source: &str,
) -> Result<(StackGraph, Handle<File>), BuildError> {
    let language =
        StackGraphLanguage::from_str(tree_sitter_python::language(), tsg_source).unwrap();
    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file("test.py");
    let globals = Variables::new();
    language.build_stack_graph_into(&mut graph, file, python_source, &globals, &NoCancellation)?;
    Ok((graph, file))
}
