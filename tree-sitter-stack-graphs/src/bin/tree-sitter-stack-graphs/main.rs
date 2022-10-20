// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use tree_sitter_stack_graphs::cli;

/// The CLI
#[derive(Parser)]
#[clap(about, version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

mod init;

#[derive(Subcommand)]
enum Commands {
    Init(init::Command),
    Parse(Parse),
    Test(Test),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Init(cmd) => cmd.run(),
        Commands::Parse(cmd) => cmd.run(),
        Commands::Test(cmd) => cmd.run(),
    }
}

/// Parse command
#[derive(clap::Parser)]
pub struct Parse {
    #[clap(flatten)]
    loader: cli::load::LoadArgs,

    #[clap(flatten)]
    parser: cli::parse::ParseArgs,
}

impl Parse {
    pub fn run(&self) -> anyhow::Result<()> {
        self.parser.run(&self.loader)
    }
}

/// Test command
#[derive(clap::Parser)]
pub struct Test {
    #[clap(flatten)]
    loader: cli::load::LoadArgs,

    #[clap(flatten)]
    tester: cli::test::TestArgs,
}

impl Test {
    pub fn run(&self) -> anyhow::Result<()> {
        self.tester.run(&self.loader)
    }
}
