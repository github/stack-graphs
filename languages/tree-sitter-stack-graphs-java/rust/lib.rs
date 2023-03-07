use tree_sitter_stack_graphs::loader::FileAnalyzers;
use tree_sitter_stack_graphs::loader::LanguageConfiguration;
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
    LanguageConfiguration::from_tsg_str(
        tree_sitter_java::language(),
        Some(String::from("source.java")),
        None,
        vec![String::from("java")],
        STACK_GRAPHS_TSG_SOURCE,
        Some(STACK_GRAPHS_BUILTINS_SOURCE),
        Some(STACK_GRAPHS_BUILTINS_CONFIG),
        FileAnalyzers::new(),
        cancellation_flag,
    )
    .unwrap()
}
