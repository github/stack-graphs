// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lazy_static::lazy_static;
use pretty_assertions::assert_eq;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use tree_sitter_graph::functions::Functions;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::test::Test;
use tree_sitter_stack_graphs::LoadError;
use tree_sitter_stack_graphs::StackGraphLanguage;

lazy_static! {
    static ref TSG: &'static str = r#"
      (module) @mod {
          node @mod.lexical_in
          node @mod.lexical_out
          edge @mod.lexical_in -> ROOT_NODE
          edge ROOT_NODE -> @mod.lexical_out
      }
      (module (_)@stmt) @mod {
          node @stmt.lexical_in
          node @stmt.lexical_out
          edge @stmt.lexical_in -> @mod.lexical_in
          edge @mod.lexical_out -> @stmt.lexical_out
      }
      (module (_)@left . (_)@right) {
          edge @right.lexical_in -> @left.lexical_out
      }
      (expression_statement (assignment left:(identifier)@name))@stmt {
          node @name.def
          attr (@name.def) type = "pop_symbol", symbol = (source-text @name), source_node = @name, is_definition
          edge @stmt.lexical_out -> @name.def
      }
      [
        (expression_statement (assignment right:(identifier)@name))@stmt
        (expression_statement (identifier)@name)@stmt
      ] {
          node @name.ref
          attr (@name.ref) type = "push_symbol", symbol = (source-text @name), source_node = @name, is_reference
          edge @name.ref -> @stmt.lexical_in
      }
    "#;
}

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

fn check_test(
    python_source: &str,
    tsg_source: &str,
    expected_successes: usize,
    expected_failures: usize,
) {
    let mut test = Test::from_source("test.py", python_source).expect("Could not parse test");
    let assertion_count: usize = test.files.iter().map(|f| f.assertions.len()).sum();
    assert_eq!(
        expected_successes + expected_failures,
        assertion_count,
        "expected {} assertions, got {}",
        expected_successes + expected_failures,
        assertion_count,
    );
    for test_file in &test.files {
        build_stack_graph_into(
            &mut test.graph,
            test_file.file,
            &test_file.source,
            tsg_source,
        )
        .expect("Could not load stack graph");
    }
    let results = test.run();
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
    check_test(python, &TSG, 4, 0);
}

#[test]
fn test_can_be_multi_file() {
    let python = r#"
      # --- path: a.py ---
      x = 1;

      # --- path: b.py ---
        x;
      # ^ defined: 3
    "#;
    check_test(python, &TSG, 1, 0);
}
