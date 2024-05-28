// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::path::Path;

use stack_graphs::graph::StackGraph;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::NoCancellation;
use tree_sitter_stack_graphs::StackGraphLanguage;

use crate::edges::check_stack_graph_edges;
use crate::nodes::check_stack_graph_nodes;

#[test]
fn can_support_preexisting_nodes() {
    let tsg = r#"
    (module)@mod {
      node @mod.lexical_scope
    }
    "#;
    let python = "pass";

    let file_name = "test.py";
    let source_path = Path::new(file_name);
    let source_root = Path::new("");

    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file(file_name);
    let node_id = graph.new_node_id(file);
    let _preexisting_node = graph.add_scope_node(node_id, true).unwrap();

    let globals = Variables::new();
    let language = StackGraphLanguage::from_str(tree_sitter_python::language(), tsg).unwrap();
    language
        .build_stack_graph_into(
            &mut graph,
            file,
            python,
            source_path,
            source_root,
            &globals,
            &NoCancellation,
        )
        .expect("Failed to build graph");
}

#[test]
fn can_support_injected_nodes() {
    let tsg = r#"
    global EXT_NODE
    (module)@mod {
      node @mod.lexical_scope
      edge @mod.lexical_scope -> EXT_NODE
    }
    "#;
    let python = "pass";

    let file_name = "test.py";
    let source_path = Path::new(file_name);
    let source_root = Path::new("");

    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file(file_name);
    let node_id = graph.new_node_id(file);
    let _preexisting_node = graph.add_scope_node(node_id, true).unwrap();

    let language = StackGraphLanguage::from_str(tree_sitter_python::language(), tsg).unwrap();
    let mut builder =
        language.builder_into_stack_graph(&mut graph, file, python, source_path, source_root);

    let mut globals = Variables::new();
    globals
        .add("EXT_NODE".into(), builder.inject_node(node_id).into())
        .expect("Failed to add EXT_NODE variable");

    builder
        .build(&globals, &NoCancellation)
        .expect("Failed to build graph");

    check_stack_graph_nodes(
        &graph,
        file,
        &["[test.py(0) exported scope]", "[test.py(1) scope]"],
    );
    check_stack_graph_edges(
        &graph,
        &["[test.py(1) scope] -0-> [test.py(0) exported scope]"],
    );
}
