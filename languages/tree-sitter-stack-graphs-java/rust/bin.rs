use anyhow::anyhow;
use clap::Parser;
use tree_sitter_stack_graphs::cli::database::default_user_database_path_for_crate;
use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
use tree_sitter_stack_graphs::NoCancellation;

fn main() -> anyhow::Result<()> {
    let lc = match tree_sitter_stack_graphs_java::try_language_configuration(&NoCancellation) {
        Ok(lc) => lc,
        Err(err) => {
            eprintln!("{}", err.display_pretty());
            return Err(anyhow!("Language configuration error"));
        }
    };
    let cli = Cli::parse();
    let default_db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;
    cli.subcommand.run(default_db_path, vec![lc])
}

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[clap(subcommand)]
    subcommand: Subcommands,
}
