use tree_sitter_stack_graphs::cli::LanguageConfigurationsCli as Cli;

fn main() -> anyhow::Result<()> {
    Cli::main(vec![tree_sitter_stack_graphs_java::language_configuration()])
}
