// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! This crate defines reusable subcommands for clap-based CLI implementations.
//!
//! ## Path loading CLIs
//!
//! Path loading CLIs load language configurations from the file system, based on
//! tree-sitter configuration files and provided arguments. Derive a path loading
//! CLI from as follows:
//!
//! ``` no_run
//! use clap::Parser;
//! use tree_sitter_stack_graphs::cli::path_loading::Subcommands;
//!
//! #[derive(Parser)]
//! #[clap(about, version)]
//! pub struct Cli {
//!     #[clap(subcommand)]
//!     subcommand: Subcommands,
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     let cli = Cli::parse();
//!     cli.subcommand.run()
//! }
//! ```
//!
//! ## Provided languages CLIs
//!
//! Provided languages CLIs use directly provided language configuration instances.
//! Derive a language configuration CLI as follows:
//!
//! ``` no_run
//! use clap::Parser;
//! use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
//!
//! #[derive(Parser)]
//! #[clap(about, version)]
//! pub struct Cli {
//!     #[clap(subcommand)]
//!     subcommand: Subcommands,
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     let language_configurations = vec![/* add your language configurations here */];
//!     let cli = Cli::parse();
//!     cli.subcommand.run(language_configurations)
//! }
//! ```

pub mod analyze;
pub mod init;
pub mod load;
pub mod parse;
pub mod test;
mod util;

pub mod path_loading {
    use clap::Subcommand;

    use crate::cli::analyze::AnalyzeArgs;
    use crate::cli::init::InitArgs;
    use crate::cli::load::PathLoaderArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::test::TestArgs;

    #[derive(Subcommand)]
    pub enum Subcommands {
        Analyze(Analyze),
        Init(Init),
        Parse(Parse),
        Test(Test),
    }

    impl Subcommands {
        pub fn run(&self) -> anyhow::Result<()> {
            match self {
                Self::Analyze(cmd) => cmd.run(),
                Self::Init(cmd) => cmd.run(),
                Self::Parse(cmd) => cmd.run(),
                Self::Test(cmd) => cmd.run(),
            }
        }
    }

    /// Analyze command
    #[derive(clap::Parser)]
    pub struct Analyze {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        analyze_args: AnalyzeArgs,
    }

    impl Analyze {
        pub fn run(&self) -> anyhow::Result<()> {
            let mut loader = self.load_args.get()?;
            self.analyze_args.run(&mut loader)
        }
    }

    /// Init command
    #[derive(clap::Parser)]
    pub struct Init {
        #[clap(flatten)]
        init_args: InitArgs,
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
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        parse_args: ParseArgs,
    }

    impl Parse {
        pub fn run(&self) -> anyhow::Result<()> {
            let mut loader = self.load_args.get()?;
            self.parse_args.run(&mut loader)
        }
    }

    /// Test command
    #[derive(clap::Parser)]
    pub struct Test {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        test_args: TestArgs,
    }

    impl Test {
        pub fn run(&self) -> anyhow::Result<()> {
            let mut loader = self.load_args.get()?;
            self.test_args.run(&mut loader)
        }
    }
}

pub mod provided_languages {
    use clap::Subcommand;

    use crate::cli::analyze::AnalyzeArgs;
    use crate::cli::load::LanguageConfigurationsLoaderArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::test::TestArgs;
    use crate::loader::LanguageConfiguration;

    #[derive(Subcommand)]
    pub enum Subcommands {
        Analyze(Analyze),
        Parse(Parse),
        Test(Test),
    }

    impl Subcommands {
        pub fn run(&self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            match self {
                Self::Analyze(cmd) => cmd.run(configurations),
                Self::Parse(cmd) => cmd.run(configurations),
                Self::Test(cmd) => cmd.run(configurations),
            }
        }
    }

    /// Analyze command
    #[derive(clap::Parser)]
    pub struct Analyze {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        analyze_args: AnalyzeArgs,
    }

    impl Analyze {
        pub fn run(&self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let mut loader = self.load_args.get(configurations)?;
            self.analyze_args.run(&mut loader)
        }
    }

    /// Parse command
    #[derive(clap::Parser)]
    pub struct Parse {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        parse_args: ParseArgs,
    }

    impl Parse {
        pub fn run(&self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let mut loader = self.load_args.get(configurations)?;
            self.parse_args.run(&mut loader)
        }
    }

    /// Test command
    #[derive(clap::Parser)]
    pub struct Test {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        test_args: TestArgs,
    }

    impl Test {
        pub fn run(&self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let mut loader = self.load_args.get(configurations)?;
            self.test_args.run(&mut loader)
        }
    }
}
