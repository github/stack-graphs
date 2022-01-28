// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use pretty_assertions::assert_eq;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;
use tree_sitter_graph::functions::Functions;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::assert::Assertions;
use tree_sitter_stack_graphs::LoadError;
use tree_sitter_stack_graphs::StackGraphLanguage;

fn build_stack_graph_into(
    graph: &mut StackGraph,
    file: Handle<File>,
    python_source: &str,
    tsg_source: &str,
) -> Result<(), LoadError> {
    let functions = Functions::stdlib();
    let mut language =
        StackGraphLanguage::from_str(tree_sitter_python::language(), tsg_source, functions)
            .unwrap();
    let mut globals = Variables::new();
    language.build_stack_graph_into(graph, file, python_source, &mut globals)?;
    Ok(())
}

fn check_assertions(
    python_source: &str,
    tsg_source: &str,
    expected_successes: usize,
    expected_failures: usize,
) {
    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file("test.py");
    let assertions =
        Assertions::from_source(file, python_source).expect("Could not parse assertions");
    assert_eq!(
        expected_successes + expected_failures,
        assertions.count(),
        "expected {} assertions, got {}",
        expected_successes + expected_failures,
        assertions.count()
    );
    build_stack_graph_into(&mut graph, file, python_source, tsg_source)
        .expect("Could not load stack graph");
    let mut paths = Paths::new();
    let results = assertions.run(&graph, &mut paths);
    assert_eq!(
        expected_successes,
        results.success_count(),
        "expected {} successes, got {}",
        expected_successes,
        results.success_count()
    );
    assert_eq!(
        expected_failures,
        results.failure_count(),
        "expected {} failures, got {}",
        expected_failures,
        results.failure_count()
    );
}

#[test]
fn aligns_correctly_with_unicode() {
    let python = r#"
      x = 1;
      m = {};

      # multi code unit character in assertion line
      m[" "] = x;
      #  §   # ^ defined: 2

      # multi code point character in source line
      m["§"] = x;
      #      # ^ defined: 2

      # multi code point character in assertion line
      m[" "] = x;
      #  g̈   # ^ defined: 2

      # multi code point character in source line
      m["g̈"] = x;
      #      # ^ defined: 2
    "#;
    let tsg = r#"
      (module (_)@stmt) {
          node @stmt.lexical_in
          node @stmt.lexical_out
          edge @stmt.lexical_out -> @stmt.lexical_in
      }
      (module (_)@left . (_)@right) {
          edge @right.lexical_in -> @left.lexical_out
      }
      (expression_statement (assignment left:(identifier)@name))@stmt {
          node @name.def
          attr (@name.def) type = "pop_symbol", symbol = (source-text @name), source_node = @name, is_definition
          edge @stmt.lexical_out -> @name.def
      }
      (expression_statement (assignment right:(identifier)@name))@stmt {
          node @name.ref
          attr (@name.ref) type = "push_symbol", symbol = (source-text @name), source_node = @name, is_reference
          edge @name.ref -> @stmt.lexical_in
      }
    "#;
    check_assertions(python, tsg, 4, 0);
}
