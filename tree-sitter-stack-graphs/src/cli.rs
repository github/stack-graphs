// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines CLI

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

pub(self) const MAX_PARSE_ERRORS: usize = 5;

pub mod init;
pub mod load;
pub mod parse;
pub mod test;
mod util;

/// CLI implementation that loads grammars and stack graph definitions from paths.
#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn main() -> Result<()> {
        let cli = Cli::parse();
        match &cli.command {
            Commands::Init(cmd) => cmd.run(),
            Commands::Parse(cmd) => cmd.run(),
            Commands::Test(cmd) => cmd.run(),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Init(Init),
    Parse(Parse),
    Test(Test),
}

/// Init command
#[derive(clap::Parser)]
pub struct Init {
    #[clap(flatten)]
    init_args: self::init::InitArgs,
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
    load_args: self::load::PathsLoadArgs,

    #[clap(flatten)]
    parse_args: self::parse::ParseArgs,
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
    load_args: self::load::PathsLoadArgs,

    #[clap(flatten)]
    test_args: self::test::TestArgs,
}

impl Test {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut loader = self.load_args.new_loader()?;
        self.test_args.run(&mut loader)
    }
}
