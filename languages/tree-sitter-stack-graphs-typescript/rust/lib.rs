// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::CancellationFlag;

/// The stack graphs tsg source for this language
const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language
const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins source for this language
const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");

pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {
    LanguageConfiguration::from_tsg_str(
        tree_sitter_typescript::language_typescript(),
        Some(String::from("source.ts")),
        None,
        vec![String::from("ts")],
        STACK_GRAPHS_TSG_SOURCE,
        Some(STACK_GRAPHS_BUILTINS_SOURCE),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        cancellation_flag,
    )
    .unwrap()
}
