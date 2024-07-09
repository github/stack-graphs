// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::loader::LoadError;
use tree_sitter_stack_graphs::CancellationFlag;

/// The stack graphs tsg source for this language.
pub const STACK_GRAPHS_TSG_PATH: &str = "src/stack-graphs.tsg";
/// The stack graphs tsg source for this language.
pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language.
pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins path for this language
pub const STACK_GRAPHS_BUILTINS_PATH: &str = "src/builtins.py";
/// The stack graphs builtins source for this language.
pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.py");

pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {
    try_language_configuration(cancellation_flag).unwrap_or_else(|err| panic!("{}", err))
}

pub fn try_language_configuration(
    cancellation_flag: &dyn CancellationFlag,
) -> Result<LanguageConfiguration, LoadError> {
    LanguageConfiguration::from_sources(
        tree_sitter_python::language(),
        Some(String::from("source.py")),
        None,
        vec![String::from("py")],
        STACK_GRAPHS_TSG_PATH.into(),
        STACK_GRAPHS_TSG_SOURCE,
        Some((
            STACK_GRAPHS_BUILTINS_PATH.into(),
            STACK_GRAPHS_BUILTINS_SOURCE,
        )),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        cancellation_flag,
    )
}
