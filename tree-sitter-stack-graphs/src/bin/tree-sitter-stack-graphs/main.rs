// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Parser;
use tree_sitter_stack_graphs::cli::database::DatabaseArgs;
use tree_sitter_stack_graphs::cli::path_loading::Subcommands;

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[clap(subcommand)]
    subcommand: Subcommands,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let default_db_path = DatabaseArgs::default_for_crate(env!("CARGO_PKG_NAME"))?;
    cli.subcommand.run(&default_db_path)
}
