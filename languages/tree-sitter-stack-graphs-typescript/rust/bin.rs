// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use tree_sitter_stack_graphs::cli::database::default_user_database_path_for_crate;
use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
use tree_sitter_stack_graphs::loader::{LanguageConfiguration, LoadError};
use tree_sitter_stack_graphs::NoCancellation;

/// Flag to select the dialect of the language
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Dialect {
    Typescript,
    TSX,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let lc = match language_configuration(cli.dialect) {
        Ok(lc) => lc,
        Err(err) => {
            eprintln!("{}", err.display_pretty());
            return Err(anyhow!("Language configuration error"));
        }
    };
    let default_db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;
    cli.subcommand.run(default_db_path, vec![lc])
}

fn language_configuration<'a>(dialect: Dialect) -> Result<LanguageConfiguration, LoadError<'a>> {
    match dialect {
        Dialect::Typescript => {
            tree_sitter_stack_graphs_typescript::try_language_configuration_typescript(
                &NoCancellation,
            )
        }
        Dialect::TSX => {
            tree_sitter_stack_graphs_typescript::try_language_configuration_tsx(&NoCancellation)
        }
    }
}

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[clap(
        short,
        long,
        value_enum,
        default_value_t = Dialect::Typescript,
    )]
    dialect: Dialect,

    #[clap(subcommand)]
    subcommand: Subcommands,
}
