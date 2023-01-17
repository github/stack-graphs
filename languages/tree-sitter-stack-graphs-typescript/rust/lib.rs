// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_stack_graphs::loader::FileAnalyzers;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::CancellationFlag;

use crate::npm_package::NpmPackageAnalyzer;
use crate::tsconfig::TsConfigAnalyzer;

pub mod npm_package;
pub mod tsconfig;
pub mod util;

/// The stack graphs tsg source for this language
pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language
pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins source for this language
pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");

/// The name of the file path global variable
pub const FILE_PATH_VAR: &str = "FILE_PATH";
/// The name of the project name global variable
pub const PROJECT_NAME_VAR: &str = "PROJECT_NAME";

pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {
    LanguageConfiguration::from_tsg_str(
        tree_sitter_typescript::language_typescript(),
        Some(String::from("source.ts")),
        None,
        vec![String::from("ts")],
        STACK_GRAPHS_TSG_SOURCE,
        Some(STACK_GRAPHS_BUILTINS_SOURCE),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        FileAnalyzers::new()
            .add("tsconfig.json".to_string(), TsConfigAnalyzer {})
            .add("package.json".to_string(), NpmPackageAnalyzer {}),
        cancellation_flag,
    )
    .unwrap()
}
