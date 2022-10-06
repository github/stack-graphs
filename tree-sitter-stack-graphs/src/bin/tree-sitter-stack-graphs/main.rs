// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

pub(crate) const MAX_PARSE_ERRORS: usize = 5;

#[derive(Parser)]
#[clap(about, version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

mod loader;
mod parse;
mod test;
mod util;

#[derive(Subcommand)]
enum Commands {
    Parse(parse::Command),
    Test(test::Command),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Parse(cmd) => cmd.run(),
        Commands::Test(cmd) => cmd.run(),
    }
}
