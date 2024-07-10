// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::loader::LoadError;
use tree_sitter_stack_graphs::CancellationFlag;

use crate::npm_package::NpmPackageAnalyzer;
use crate::tsconfig::TsConfigAnalyzer;

pub mod npm_package;
pub mod tsconfig;
pub mod util;

/// The stacks graphs tsg path for this language.
pub const STACK_GRAPHS_TSG_PATH: &str = "src/stack-graphs.tsg";
/// The stack graphs tsg source for this language
pub const STACK_GRAPHS_TSG_TS_SOURCE: &str =
    include_str!(concat!(env!("OUT_DIR"), "/stack-graphs-typescript.tsg"));
pub const STACK_GRAPHS_TSG_TSX_SOURCE: &str =
    include_str!(concat!(env!("OUT_DIR"), "/stack-graphs-tsx.tsg"));

/// The stack graphs builtins configuration for this language
pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins path for this language
pub const STACK_GRAPHS_BUILTINS_PATH: &str = "src/builtins.ts";
/// The stack graphs builtins source for this language
pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");

/// The name of the project name global variable
pub const PROJECT_NAME_VAR: &str = "PROJECT_NAME";

pub fn language_configuration_typescript(
    cancellation_flag: &dyn CancellationFlag,
) -> LanguageConfiguration {
    try_language_configuration_typescript(cancellation_flag).unwrap_or_else(|err| panic!("{}", err))
}

pub fn try_language_configuration_typescript(
    cancellation_flag: &dyn CancellationFlag,
) -> Result<LanguageConfiguration, LoadError> {
    let mut lc = LanguageConfiguration::from_sources(
        tree_sitter_typescript::language_typescript(),
        Some(String::from("source.ts")),
        None,
        vec![String::from("ts")],
        STACK_GRAPHS_TSG_PATH.into(),
        STACK_GRAPHS_TSG_TS_SOURCE,
        Some((
            STACK_GRAPHS_BUILTINS_PATH.into(),
            STACK_GRAPHS_BUILTINS_SOURCE,
        )),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        cancellation_flag,
    )?;
    lc.special_files
        .add("tsconfig.json".to_string(), TsConfigAnalyzer {})
        .add("package.json".to_string(), NpmPackageAnalyzer {});
    lc.no_similar_paths_in_file = true;
    Ok(lc)
}

pub fn language_configuration_tsx(
    cancellation_flag: &dyn CancellationFlag,
) -> LanguageConfiguration {
    try_language_configuration_tsx(cancellation_flag).unwrap_or_else(|err| panic!("{}", err))
}

pub fn try_language_configuration_tsx(
    cancellation_flag: &dyn CancellationFlag,
) -> Result<LanguageConfiguration, LoadError> {
    let mut lc = LanguageConfiguration::from_sources(
        tree_sitter_typescript::language_tsx(),
        Some(String::from("source.tsx")),
        None,
        vec![String::from("tsx")],
        STACK_GRAPHS_TSG_PATH.into(),
        STACK_GRAPHS_TSG_TSX_SOURCE,
        Some((
            STACK_GRAPHS_BUILTINS_PATH.into(),
            STACK_GRAPHS_BUILTINS_SOURCE,
        )),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        cancellation_flag,
    )?;
    lc.special_files
        .add("tsconfig.json".to_string(), TsConfigAnalyzer {})
        .add("package.json".to_string(), NpmPackageAnalyzer {});
    lc.no_similar_paths_in_file = true;
    Ok(lc)
}
