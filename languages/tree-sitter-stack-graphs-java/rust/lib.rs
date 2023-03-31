use std::path::PathBuf;

use tree_sitter_stack_graphs::loader::FileAnalyzers;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;
use tree_sitter_stack_graphs::loader::LoadError;
use tree_sitter_stack_graphs::CancellationFlag;

/// The stack graphs tsg source for this language
pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language
pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins source for this language
pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.java");

/// The name of the file path global variable
pub const FILE_PATH_VAR: &str = "FILE_PATH";
/// The name of the project name global variable
pub const PROJECT_NAME_VAR: &str = "PROJECT_NAME";

pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {
    match try_language_configuration(cancellation_flag) {
        Ok(lc) => lc,
        Err(err) => panic!("{}", err),
    }
}

pub fn try_language_configuration(
    cancellation_flag: &dyn CancellationFlag,
) -> Result<LanguageConfiguration, LoadError> {
    LanguageConfiguration::from_tsg_file(
        tree_sitter_java::language(),
        Some(String::from("source.java")),
        None,
        vec![String::from("java")],
        PathBuf::from("src/stack-graphs.tsg"),
        STACK_GRAPHS_TSG_SOURCE,
        Some(STACK_GRAPHS_BUILTINS_SOURCE),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        FileAnalyzers::new(),
        cancellation_flag,
    )
}
