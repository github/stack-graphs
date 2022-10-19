use tree_sitter_stack_graphs::loader::LanguageConfiguration;

pub fn language_configuration() -> LanguageConfiguration {
    LanguageConfiguration {
        language: tree_sitter_typescript::language_typescript(),
        scope: Some(String::from("source.ts")),
        content_regex: None,
        file_types: vec![String::from("ts")],
        tsg_source: STACK_GRAPHS_TSG_SOURCE.to_string(),
        builtins_source: STACK_GRAPHS_BUILTINS_SOURCE.to_string(),
        builtins_config: STACK_GRAPHS_BUILTINS_CONFIG.to_string(),
    }
}

/// The stack graphs tsg source for this language
pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

/// The stack graphs builtins configuration for this language
pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
/// The stack graphs builtins source for this language
pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");
