// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_stack_graphs::loader::FileAnalyzers;
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
pub const STACK_GRAPHS_BUILTINS_PATH: &str = "src/builtins.rb";
/// The stack graphs builtins source for this language.
pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.rb");

/// The name of the file path global variable.
pub const FILE_PATH_VAR: &str = "FILE_PATH";

pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {
    try_language_configuration(cancellation_flag).unwrap_or_else(|err| panic!("{}", err))
}

pub fn try_language_configuration(
    cancellation_flag: &dyn CancellationFlag,
) -> Result<LanguageConfiguration, LoadError> {
    let mut lc = LanguageConfiguration::from_sources(
        tree_sitter_ruby::language(),
        Some(String::from("source.rb")),
        None,
        vec![String::from("rb")],
        STACK_GRAPHS_TSG_PATH.into(),
        STACK_GRAPHS_TSG_SOURCE,
        Some((
            STACK_GRAPHS_BUILTINS_PATH.into(),
            STACK_GRAPHS_BUILTINS_SOURCE,
        )),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        FileAnalyzers::new(),
        cancellation_flag,
    )?;
    lc.sgl.functions_mut().add("uuid".into(), UUID);
    Ok(lc)
}

struct UUID;

impl tree_sitter_graph::functions::Function for UUID {
    fn call(
        &self,
        _graph: &mut tree_sitter_graph::graph::Graph,
        _source: &str,
        parameters: &mut dyn tree_sitter_graph::functions::Parameters,
    ) -> Result<tree_sitter_graph::graph::Value, tree_sitter_graph::ExecutionError> {
        parameters.finish()?;
        Ok(uuid::Uuid::new_v4().to_string().into())
    }
}
