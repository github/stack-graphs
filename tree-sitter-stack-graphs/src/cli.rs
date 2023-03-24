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

pub(self) const MAX_PARSE_ERRORS: usize = 5;

pub mod clean;
pub mod index;
pub mod init;
pub mod load;
pub mod parse;
pub mod query;
pub mod test;
mod util;
pub mod visualize;

pub mod path_loading {
    use clap::Subcommand;

    use crate::cli::clean::CleanArgs;
    use crate::cli::index::IndexArgs;
    use crate::cli::init::InitArgs;
    use crate::cli::load::PathLoaderArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::query::QueryArgs;
    use crate::cli::test::TestArgs;
    use crate::cli::visualize::VisualizeArgs;

    #[derive(Subcommand)]
    pub enum Subcommands {
        Clean(Clean),
        Index(Index),
        Init(Init),
        Parse(Parse),
        Query(Query),
        Test(Test),
        Visualize(Visualize),
    }

    impl Subcommands {
        pub fn run(&self) -> anyhow::Result<()> {
            match self {
                Self::Clean(cmd) => cmd.run(),
                Self::Index(cmd) => cmd.run(),
                Self::Init(cmd) => cmd.run(),
                Self::Parse(cmd) => cmd.run(),
                Self::Query(cmd) => cmd.run(),
                Self::Test(cmd) => cmd.run(),
                Self::Visualize(cmd) => cmd.run(),
            }
        }
    }

    /// Clean command
    #[derive(clap::Parser)]
    pub struct Clean {
        #[clap(flatten)]
        clean_args: CleanArgs,
    }

    impl Clean {
        pub fn run(&self) -> anyhow::Result<()> {
            self.clean_args.run()
        }
    }

    /// Index command
    #[derive(clap::Parser)]
    pub struct Index {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        index_args: IndexArgs,
    }

    impl Index {
        pub fn run(&self) -> anyhow::Result<()> {
            let mut loader = self.load_args.get()?;
            self.index_args.run(&mut loader)
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

    /// Query command
    #[derive(clap::Parser)]
    pub struct Query {
        #[clap(flatten)]
        query_args: QueryArgs,
    }

    impl Query {
        pub fn run(&self) -> anyhow::Result<()> {
            self.query_args.run()
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

    /// Visualize command
    #[derive(clap::Parser)]
    pub struct Visualize {
        #[clap(flatten)]
        visualize_args: VisualizeArgs,
    }

    impl Visualize {
        pub fn run(&self) -> anyhow::Result<()> {
            self.visualize_args.run()
        }
    }
}

pub mod provided_languages {
    use clap::Subcommand;

    use crate::cli::clean::CleanArgs;
    use crate::cli::index::IndexArgs;
    use crate::cli::load::LanguageConfigurationsLoaderArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::query::QueryArgs;
    use crate::cli::test::TestArgs;
    use crate::cli::visualize::VisualizeArgs;
    use crate::loader::LanguageConfiguration;

    #[derive(Subcommand)]
    pub enum Subcommands {
        Clean(Clean),
        Index(Index),
        Parse(Parse),
        Query(Query),
        Test(Test),
        Visualize(Visualize),
    }

    impl Subcommands {
        pub fn run(&self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            match self {
                Self::Clean(cmd) => cmd.run(configurations),
                Self::Index(cmd) => cmd.run(configurations),
                Self::Parse(cmd) => cmd.run(configurations),
                Self::Query(cmd) => cmd.run(configurations),
                Self::Test(cmd) => cmd.run(configurations),
                Self::Visualize(cmd) => cmd.run(configurations),
            }
        }
    }

    /// Clean command
    #[derive(clap::Parser)]
    pub struct Clean {
        #[clap(flatten)]
        clean_args: CleanArgs,
    }

    impl Clean {
        pub fn run(&self, _configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            self.clean_args.run()
        }
    }

    /// Index command
    #[derive(clap::Parser)]
    pub struct Index {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        index_args: IndexArgs,
    }

    impl Index {
        pub fn run(&self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let mut loader = self.load_args.get(configurations)?;
            self.index_args.run(&mut loader)
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

    /// Query command
    #[derive(clap::Parser)]
    pub struct Query {
        #[clap(flatten)]
        query_args: QueryArgs,
    }

    impl Query {
        pub fn run(&self, _configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            self.query_args.run()
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

    /// Visualize command
    #[derive(clap::Parser)]
    pub struct Visualize {
        #[clap(flatten)]
        visualize_args: VisualizeArgs,
    }

    impl Visualize {
        pub fn run(&self, _configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            self.visualize_args.run()
        }
    }
}
