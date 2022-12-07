// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::loader::Loader;
use tree_sitter_stack_graphs::CancellationFlag;
use tree_sitter_stack_graphs::StackGraphLanguage;
use tree_sitter_stack_graphs::Variables;

/// The stack graphs tsg source for this language
const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language
const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins source for this language
const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");

pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {
    let language = tree_sitter_typescript::language_typescript();
    let sgl = StackGraphLanguage::from_str(language, STACK_GRAPHS_TSG_SOURCE).unwrap();
    let mut builtins = StackGraph::new();
    let file = builtins.add_file("<builtins>").unwrap();
    let mut builtins_globals = Variables::new();
    Loader::load_globals_from_config_str(STACK_GRAPHS_BUILTINS_CONFIG, &mut builtins_globals)
        .unwrap();
    sgl.build_stack_graph_into(
        &mut builtins,
        file,
        STACK_GRAPHS_BUILTINS_SOURCE,
        &builtins_globals,
        cancellation_flag,
    )
    .unwrap();
    LanguageConfiguration {
        language,
        scope: Some(String::from("source.ts")),
        content_regex: None,
        file_types: vec![String::from("ts")],
        sgl,
        builtins,
    }
}
