// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lazy_static::lazy_static;
use pretty_assertions::assert_eq;
use stack_graphs::graph::StackGraph;
use std::path::PathBuf;
use tree_sitter_stack_graphs::loader::FileAnalyzers;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::loader::Loader;
use tree_sitter_stack_graphs::NoCancellation;
use tree_sitter_stack_graphs::StackGraphLanguage;

lazy_static! {
    static ref PATH: PathBuf = PathBuf::from("test.py");
    static ref TSG: String = r#"
      (module) {}
    "#
    .to_string();
}

#[test]
fn can_load_from_provided_language_configuration() {
    let language = tree_sitter_python::language();
    let sgl = StackGraphLanguage::from_str(language, &TSG).unwrap();
    let lc = LanguageConfiguration {
        language: language,
        scope: Some("source.py".into()),
        content_regex: None,
        file_types: vec!["py".into()],
        sgl,
        builtins: StackGraph::new(),
        special_files: FileAnalyzers::new(),
    };
    let mut loader =
        Loader::from_language_configurations(vec![lc], None).expect("Expected loader to succeed");

    let tsl = loader
        .load_tree_sitter_language_for_file(&PATH, &mut None)
        .expect("Expected loading tree-sitter language to succeed");
    assert_eq!(tsl, Some(language));

    let lc = loader
        .load_for_file(&PATH, &mut None, &NoCancellation)
        .expect("Expected loading stack graph language to succeed");
    assert_eq!(lc.map(|lc| lc.language), Some(language));
}
