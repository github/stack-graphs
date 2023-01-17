use clap::Parser;
use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
use tree_sitter_stack_graphs::NoCancellation;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.subcommand.run(vec![
                       tree_sitter_stack_graphs_java::language_configuration(&NoCancellation),
    ])
}

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[clap(subcommand)]
    subcommand: Subcommands,
}
