// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use pretty_assertions::assert_eq;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::LoadError;
use tree_sitter_stack_graphs::StackGraphLanguage;

fn load_stack_graph(python_source: &str, tsg_source: &str) -> Result<StackGraph, LoadError> {
    let mut language = StackGraphLanguage::new(tree_sitter_python::language(), tsg_source).unwrap();
    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file("test.py");
    language.load_stack_graph(&mut graph, file, python_source)?;
    Ok(graph)
}

fn check_stack_graph_node(python_source: &str, tsg_source: &str, expected_nodes: &[&str]) {
    let graph = load_stack_graph(python_source, tsg_source).expect("Could not load stack graph");
    let actual_nodes = graph
        .iter_nodes()
        .skip(2) // skip root and jump-to-scope nodes
        .map(|handle| graph[handle].display(&graph).to_string())
        .collect::<Vec<_>>();
    assert_eq!(expected_nodes, actual_nodes);
}

#[test]
fn can_create_definition_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "definition", symbol = (source-text @id)
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) definition a]"]);
}

#[test]
fn cannot_create_definition_node_without_symbol() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "definition"
      }
    "#;
    let python = "a";
    let result = load_stack_graph(python, tsg);
    assert!(matches!(result, Err(LoadError::MissingSymbol(_))));
}

#[test]
fn can_create_drop_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "drop"
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) drop scopes]"]);
}

#[test]
fn can_create_exported_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "exported"
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) exported scope]"]);
}

#[test]
fn can_create_endpoint_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "endpoint"
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) exported scope]"]);
}

#[test]
fn can_create_implicit_internal_node() {
    let tsg = r#"
      (identifier) @id {
         node result
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) scope]"]);
}

#[test]
fn can_create_explicit_internal_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "internal"
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) scope]"]);
}

#[test]
fn can_create_pop_symbol_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "pop", symbol = (source-text @id)
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) pop a]"]);
}

#[test]
fn cannot_create_pop_symbol_node_without_symbol() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "pop"
      }
    "#;
    let python = "a";
    let result = load_stack_graph(python, tsg);
    assert!(matches!(result, Err(LoadError::MissingSymbol(_))));
}

#[test]
fn can_create_pop_scoped_symbol_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "pop", symbol = (source-text @id), scoped = #true
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) pop scoped a]"]);
}

#[test]
fn cannot_create_pop_scoped_symbol_node_without_symbol() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "pop", scoped = #true
      }
    "#;
    let python = "a";
    let result = load_stack_graph(python, tsg);
    assert!(matches!(result, Err(LoadError::MissingSymbol(_))));
}

#[test]
fn can_create_push_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "push", symbol = (source-text @id)
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) push a]"]);
}

#[test]
fn cannot_create_push_symbol_node_without_symbol() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "push"
      }
    "#;
    let python = "a";
    let result = load_stack_graph(python, tsg);
    assert!(matches!(result, Err(LoadError::MissingSymbol(_))));
}

#[test]
fn can_create_push_scoped_node() {
    let tsg = r#"
      (identifier) @id {
         node scope
         attr (scope) type = "exported"
         node result
         attr (result) type = "push", symbol = (source-text @id), scope = scope
      }
    "#;
    let python = "a";
    check_stack_graph_node(
        python,
        tsg,
        &[
            "[test.py(0) exported scope]", //
            "[test.py(1) push scoped a test.py(0)]",
        ],
    );
}

#[test]
fn cannot_create_push_scoped_symbol_node_without_symbol() {
    let tsg = r#"
      (identifier) @id {
         node scope
         attr (scope) type = "exported"
         node result
         attr (result) type = "push", scope = scope
      }
    "#;
    let python = "a";
    let result = load_stack_graph(python, tsg);
    assert!(matches!(result, Err(LoadError::MissingSymbol(_))));
}

#[test]
fn can_create_reference_node() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "reference", symbol = (source-text @id)
      }
    "#;
    let python = "a";
    check_stack_graph_node(python, tsg, &["[test.py(0) reference a]"]);
}

#[test]
fn cannot_create_reference_node_without_symbol() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "reference"
      }
    "#;
    let python = "a";
    let result = load_stack_graph(python, tsg);
    assert!(matches!(result, Err(LoadError::MissingSymbol(_))));
}

#[test]
fn can_calculate_spans() {
    let tsg = r#"
      (identifier) @id {
         node result
         attr (result) type = "definition", symbol = "test", source_node = @id
      }
    "#;
    let python = "  a  ";
    let graph = load_stack_graph(python, tsg).unwrap();
    let node_handle = graph.iter_nodes().nth(2).unwrap();
    let source_info = graph.source_info(node_handle).unwrap();

    let span = format!(
        "{}:{}-{}:{}",
        source_info.span.start.line,
        source_info.span.start.column.utf8_offset,
        source_info.span.end.line,
        source_info.span.end.column.utf8_offset,
    );
    assert_eq!("0:2-0:3", span);

    let containing_line = source_info.containing_line.into_option().unwrap();
    let containing_line = &graph[containing_line];
    assert_eq!(containing_line, "  a  ");

    let trimmed_line = &python[source_info.span.start.trimmed_line.clone()];
    assert_eq!(trimmed_line, "a");
}
