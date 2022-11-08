use tree_sitter_stack_graphs::loader::LanguageConfiguration;

/// The stack graphs tsg source for this language
const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

pub fn language_configuration() -> LanguageConfiguration {
    LanguageConfiguration {
        language: tree_sitter_java::language(),
        scope: Some(String::from("source.java")),
        content_regex: None,
        file_types: vec![String::from("java")],
        tsg_source: STACK_GRAPHS_TSG_SOURCE.to_string(),
        builtins: None,
    }
}
