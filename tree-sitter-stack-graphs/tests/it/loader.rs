// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lazy_static::lazy_static;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::loader::Loader;
use tree_sitter_stack_graphs::NoCancellation;

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
    let mut loader = Loader::from_language_configurations(
        vec![LanguageConfiguration {
            language: language,
            scope: Some("source.py".into()),
            content_regex: None,
            file_types: vec!["py".into()],
            tsg_source: TSG.to_string(),
            builtins: None,
        }],
        None,
    )
    .expect("Expected loader to succeed");

    let tsl = loader
        .load_tree_sitter_language_for_file(&PATH, None)
        .expect("Expected loading tree-sitter language to succeed");
    assert_eq!(tsl, Some(language));

    let sgl = loader
        .load_for_file(&PATH, None, &NoCancellation)
        .expect("Expected loading stack graph language to succeed");
    assert_eq!(sgl.map(|sgl| sgl.language()), Some(language));
}
