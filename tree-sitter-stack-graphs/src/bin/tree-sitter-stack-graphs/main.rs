// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[clap(about)]
#[clap(version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

mod test;

#[derive(Subcommand)]
enum Commands {
    Test(test::Command),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Test(cmd) => cmd.run(),
    }
}
