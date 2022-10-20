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

#[derive(Subcommand)]
enum Commands {
    Init(Init),
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

/// Init command
#[derive(clap::Parser)]
pub struct Init {
    #[clap(flatten)]
    init_args: cli::init::InitArgs,
}

impl Init {
    pub fn run(&self) -> anyhow::Result<()> {
        self.init_args.run()
    }
}

/// Parse command
#[derive(clap::Parser)]
pub struct Parse {
    #[clap(flatten)]
    load_args: cli::load::LoadArgs,

    #[clap(flatten)]
    parse_args: cli::parse::ParseArgs,
}

impl Parse {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut loader = self.load_args.new_loader()?;
        self.parse_args.run(&mut loader)
    }
}

/// Test command
#[derive(clap::Parser)]
pub struct Test {
    #[clap(flatten)]
    load_args: cli::load::LoadArgs,

    #[clap(flatten)]
    test_args: cli::test::TestArgs,
}

impl Test {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut loader = self.load_args.new_loader()?;
        self.test_args.run(&mut loader)
    }
}
