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
use std::path::Path;
use std::path::PathBuf;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::test::Test;
use tree_sitter_stack_graphs::LoadError;
use tree_sitter_stack_graphs::NoCancellation;
use tree_sitter_stack_graphs::StackGraphLanguage;

lazy_static! {
    static ref PATH: PathBuf = PathBuf::from("test.py");
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
    let language =
        StackGraphLanguage::from_str(tree_sitter_python::language(), tsg_source).unwrap();
    let globals = Variables::new();
    language.build_stack_graph_into(graph, file, python_source, &globals, &NoCancellation)?;
    Ok(())
}

fn check_test(
    python_path: &Path,
    python_source: &str,
    tsg_source: &str,
    expected_successes: usize,
    expected_failures: usize,
) {
    let mut test =
        Test::from_source(python_path, python_source, python_path).expect("Could not parse test");
    let assertion_count: usize = test.fragments.iter().map(|f| f.assertions.len()).sum();
    assert_eq!(
        expected_successes + expected_failures,
        assertion_count,
        "expected {} assertions, got {}",
        expected_successes + expected_failures,
        assertion_count,
    );
    for fragments in &test.fragments {
        build_stack_graph_into(
            &mut test.graph,
            fragments.file,
            &fragments.source,
            tsg_source,
        )
        .expect("Could not load stack graph");
    }
    let results = test
        .run(&NoCancellation)
        .expect("should never be cancelled");
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
    check_test(&PATH, python, &TSG, 4, 0);
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
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn test_assert_multiple_lines() {
    let python = r#"
      # --- path: a.py ---
      x = 1;

      # --- path: b.py ---
      x = 1;

      # --- path: c.py ---
        x;
      # ^ defined: 3, 6
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn test_fragment_can_have_same_name_as_test() {
    let python = r#"
      # --- path: test.py ---
      x = 1;
        x;
      # ^ defined: 3
    "#;
    check_test(&PathBuf::from("test.py"), python, &TSG, 1, 0);
}

#[test]
fn test_cannot_assert_on_first_line() {
    let python = r#"
      # ^ defined: 3
    "#;
    if let Ok(_) = Test::from_source(&PATH, python, &PATH) {
        panic!("Parsing test unexpectedly succeeded.");
    }
}

#[test]
fn test_cannot_assert_before_first_fragment() {
    let python = r#"
      # this is ignored
      # --- path: a.py ---
        x;
      # ^ defined: 1
    "#;
    if let Ok(_) = Test::from_source(&PATH, python, &PATH) {
        panic!("Parsing test unexpectedly succeeded.");
    }
}
