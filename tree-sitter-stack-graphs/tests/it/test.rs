// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use once_cell::sync::Lazy;
use pretty_assertions::assert_eq;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::test::Test;
use tree_sitter_stack_graphs::LoadError;
use tree_sitter_stack_graphs::NoCancellation;
use tree_sitter_stack_graphs::StackGraphLanguage;

static PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("test.py"));
static TSG: Lazy<String> = Lazy::new(|| {
    r#"
      global ROOT_NODE
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
    "#.to_string()
});
static TSG_WITH_PKG: Lazy<String> = Lazy::new(|| {
    r#"
      global PKG
    "#
    .to_string()
        + &TSG
});

fn build_stack_graph_into(
    graph: &mut StackGraph,
    file: Handle<File>,
    python_source: &str,
    tsg_source: &str,
    globals: &Variables,
) -> Result<(), LoadError> {
    let language =
        StackGraphLanguage::from_str(tree_sitter_python::language(), tsg_source).unwrap();
    language.build_stack_graph_into(graph, file, python_source, globals, &NoCancellation)?;
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

    let mut globals = Variables::new();
    for fragment in &test.fragments {
        globals.clear();
        fragment.add_globals_to(&mut globals);
        build_stack_graph_into(
            &mut test.graph,
            fragment.file,
            &fragment.source,
            tsg_source,
            &globals,
        )
        .expect("Could not load stack graph");
    }

    let mut partials = PartialPaths::new();
    let mut db = Database::new();
    for fragment in &test.fragments {
        partials
            .find_minimal_partial_path_set_in_file(
                &test.graph,
                fragment.file,
                &stack_graphs::NoCancellation,
                |graph, partials, path| {
                    db.add_partial_path(graph, partials, path);
                },
            )
            .expect("should nopt be cancelled");
    }

    let results = test
        .run(&mut partials, &mut db, &NoCancellation)
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
fn can_assert_defined_on_one_line() {
    let python = r#"
      x = 1;
        x;
      # ^ defined: 2
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn can_assert_defined_on_multiple_lines() {
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
fn can_assert_defined_on_no_lines() {
    let python = r#"
      y = 1;
        x;
      # ^ defined:
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn can_assert_defines_one_symbol() {
    let python = r#"
        x = 1;
      # ^ defines: x
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn can_assert_defines_no_symbols() {
    let python = r#"
        x;
      # ^ defines:
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn can_assert_refers_one_symbol() {
    let python = r#"
        x;
      # ^ refers: x
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn can_assert_refers_no_symbols() {
    let python = r#"
        x = 1;
      # ^ refers:
    "#;
    check_test(&PATH, python, &TSG, 1, 0);
}

#[test]
fn test_cannot_use_unknown_assertion() {
    let python = r#"
      foo = 42
      # ^ supercalifragilisticexpialidocious:
    "#;
    if let Ok(_) = Test::from_source(&PATH, python, &PATH) {
        panic!("Parsing test unexpectedly succeeded.");
    }
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

#[test]
fn test_can_set_global() {
    let python = r#"
      # --- global: PKG=test ---
      pass
    "#;
    check_test(&PathBuf::from("test.py"), python, &TSG_WITH_PKG, 0, 0);
}

#[test]
fn test_can_set_global_in_fragments() {
    let python = r#"
      # --- path: a.py ---
      # --- global: PKG=test ---
      pass
      # --- path: b.py ---
      # --- global: PKG=test ---
      pass
    "#;
    check_test(&PathBuf::from("test.py"), python, &TSG_WITH_PKG, 0, 0);
}

#[test]
fn test_cannot_set_global_before_first_fragment() {
    let python = r#"
      # --- global: PKG=test ---
      # --- path: a.py ---
      pass
    "#;
    if let Ok(_) = Test::from_source(&PATH, python, &PATH) {
        panic!("Parsing test unexpectedly succeeded.");
    }
}
