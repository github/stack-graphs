// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_stack_graphs::loader::BuiltinsConfiguration;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;

/// The stack graphs tsg source for this language
const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language
const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins source for this language
const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");

pub fn language_configuration() -> LanguageConfiguration {
    LanguageConfiguration {
        language: tree_sitter_typescript::language_typescript(),
        scope: Some(String::from("source.ts")),
        content_regex: None,
        file_types: vec![String::from("ts")],
        tsg_source: STACK_GRAPHS_TSG_SOURCE.to_string(),
        builtins: Some(BuiltinsConfiguration {
            source: STACK_GRAPHS_BUILTINS_SOURCE.to_string(),
            config: STACK_GRAPHS_BUILTINS_CONFIG.to_string(),
        }),
    }
}
