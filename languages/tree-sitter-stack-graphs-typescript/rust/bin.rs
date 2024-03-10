// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::Parser;
use tree_sitter_stack_graphs::cli::database::default_user_database_path_for_crate;
use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
use tree_sitter_stack_graphs::NoCancellation;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let lc = match language_configuration(&cli.dialect)
    {
        Ok(lc) => lc,
        Err(err) => {
            eprintln!("{err}");
            return Err(anyhow!("Language configuration error"));
        }
    };
    let default_db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;
    cli.subcommand.run(default_db_path, vec![lc])
}

fn language_configuration(dialect: &str) -> anyhow::Result<tree_sitter_stack_graphs::loader::LanguageConfiguration> {
    match dialect {
        "typescript" => tree_sitter_stack_graphs_typescript::try_language_configuration(&NoCancellation)
            .map_err(|e| anyhow::anyhow!("{}", e.display_pretty())),

        "tsx" => tree_sitter_stack_graphs_typescript::try_language_configuration_tsx(&NoCancellation)
            .map_err(|e| anyhow::anyhow!("{}", e.display_pretty())),

        _ => anyhow::bail!("Unknown dialect: {}", dialect)
    }
}

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[clap(short, long, default_value = "typescript")]
    dialect: String,

    #[clap(subcommand)]
    subcommand: Subcommands,
}
