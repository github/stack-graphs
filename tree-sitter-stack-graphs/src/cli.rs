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
//! use tree_sitter_stack_graphs::cli::database::default_user_database_path_for_crate;
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
//!     let default_db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;
//!     cli.subcommand.run(default_db_path)
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
//! use tree_sitter_stack_graphs::cli::database::default_user_database_path_for_crate;
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
//!     let cli = Cli::parse();
//!     let language_configurations = vec![/* add your language configurations here */];
//!     let default_db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;
//!     cli.subcommand.run(default_db_path, language_configurations)
//! }
//! ```

pub mod clean;
pub mod database;
pub mod index;
pub mod init;
pub mod load;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod r#match;
pub mod parse;
pub mod query;
pub mod status;
pub mod test;
mod util;

pub mod path_loading {
    use std::path::PathBuf;

    use clap::Subcommand;

    use crate::cli::clean::CleanArgs;
    use crate::cli::index::IndexArgs;
    use crate::cli::init::InitArgs;
    use crate::cli::load::PathLoaderArgs;
    #[cfg(feature = "lsp")]
    use crate::cli::lsp::LspArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::query::QueryArgs;
    use crate::cli::r#match::MatchArgs;
    use crate::cli::status::StatusArgs;
    use crate::cli::test::TestArgs;

    use super::database::DatabaseArgs;

    #[derive(Subcommand)]
    pub enum Subcommands {
        Clean(Clean),
        Index(Index),
        Init(Init),
        #[cfg(feature = "lsp")]
        Lsp(Lsp),
        Match(Match),
        Parse(Parse),
        Query(Query),
        Status(Status),
        Test(Test),
    }

    impl Subcommands {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            match self {
                Self::Clean(cmd) => cmd.run(default_db_path),
                Self::Index(cmd) => cmd.run(default_db_path),
                Self::Init(cmd) => cmd.run(),
                #[cfg(feature = "lsp")]
                Self::Lsp(cmd) => cmd.run(default_db_path),
                Self::Match(cmd) => cmd.run(),
                Self::Parse(cmd) => cmd.run(),
                Self::Query(cmd) => cmd.run(default_db_path),
                Self::Status(cmd) => cmd.run(default_db_path),
                Self::Test(cmd) => cmd.run(),
            }
        }
    }

    /// Clean the indexing database.
    #[derive(clap::Parser)]
    pub struct Clean {
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        clean_args: CleanArgs,
    }

    impl Clean {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let db_path = self.db_args.get_or(default_db_path);
            self.clean_args.run(&db_path)
        }
    }

    /// Index source files into the database.
    #[derive(clap::Parser)]
    pub struct Index {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        index_args: IndexArgs,
    }

    impl Index {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let loader = self.load_args.get()?;
            let db_path = self.db_args.get_or(default_db_path);
            self.index_args.run(&db_path, loader)
        }
    }

    /// Inititialize a new stack graphs project for a tree-sitter language.
    #[derive(clap::Parser)]
    pub struct Init {
        #[clap(flatten)]
        init_args: InitArgs,
    }

    impl Init {
        pub fn run(self) -> anyhow::Result<()> {
            self.init_args.run()
        }
    }

    /// Run an LSP server.
    #[cfg(feature = "lsp")]
    #[derive(clap::Parser)]
    pub struct Lsp {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        lsp_args: LspArgs,
    }

    #[cfg(feature = "lsp")]
    impl Lsp {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let loader = self.load_args.get()?;
            let db_path = self.db_args.get_or(default_db_path);
            self.lsp_args.run(db_path, loader)
        }
    }

    /// Match stanza queries against a source file.
    #[derive(clap::Parser)]
    pub struct Match {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        match_args: MatchArgs,
    }

    impl Match {
        pub fn run(self) -> anyhow::Result<()> {
            let loader = self.load_args.get()?;
            self.match_args.run(loader)
        }
    }

    /// Parse a source file and show the parse tree.
    #[derive(clap::Parser)]
    pub struct Parse {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        parse_args: ParseArgs,
    }

    impl Parse {
        pub fn run(self) -> anyhow::Result<()> {
            let loader = self.load_args.get()?;
            self.parse_args.run(loader)
        }
    }

    /// Query the database to resolve references.
    #[derive(clap::Parser)]
    pub struct Query {
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        query_args: QueryArgs,
    }

    impl Query {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let db_path = self.db_args.get_or(default_db_path);
            self.query_args.run(&db_path)
        }
    }

    /// Show indexing status for source files.
    #[derive(clap::Parser)]
    pub struct Status {
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        status_args: StatusArgs,
    }

    impl Status {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let db_path = self.db_args.get_or(default_db_path);
            self.status_args.run(&db_path)
        }
    }

    /// Run test files and show results.
    #[derive(clap::Parser)]
    pub struct Test {
        #[clap(flatten)]
        load_args: PathLoaderArgs,
        #[clap(flatten)]
        test_args: TestArgs,
    }

    impl Test {
        pub fn run(self) -> anyhow::Result<()> {
            let loader = self.load_args.get()?;
            self.test_args.run(loader)
        }
    }
}

pub mod provided_languages {
    use std::path::PathBuf;

    use clap::Subcommand;

    use crate::cli::clean::CleanArgs;
    use crate::cli::index::IndexArgs;
    use crate::cli::init::InitArgs;
    use crate::cli::load::LanguageConfigurationsLoaderArgs;
    #[cfg(feature = "lsp")]
    use crate::cli::lsp::LspArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::query::QueryArgs;
    use crate::cli::r#match::MatchArgs;
    use crate::cli::status::StatusArgs;
    use crate::cli::test::TestArgs;
    use crate::loader::LanguageConfiguration;

    use super::database::DatabaseArgs;

    #[derive(Subcommand)]
    pub enum Subcommands {
        Clean(Clean),
        Index(Index),
        Init(Init),
        #[cfg(feature = "lsp")]
        Lsp(Lsp),
        Match(Match),
        Parse(Parse),
        Query(Query),
        Status(Status),
        Test(Test),
    }

    impl Subcommands {
        pub fn run(
            self,
            default_db_path: PathBuf,
            configurations: Vec<LanguageConfiguration>,
        ) -> anyhow::Result<()> {
            match self {
                Self::Clean(cmd) => cmd.run(default_db_path),
                Self::Index(cmd) => cmd.run(default_db_path, configurations),
                Self::Init(cmd) => cmd.run(),
                #[cfg(feature = "lsp")]
                Self::Lsp(cmd) => cmd.run(default_db_path, configurations),
                Self::Match(cmd) => cmd.run(configurations),
                Self::Parse(cmd) => cmd.run(configurations),
                Self::Query(cmd) => cmd.run(default_db_path),
                Self::Status(cmd) => cmd.run(default_db_path),
                Self::Test(cmd) => cmd.run(configurations),
            }
        }
    }

    /// Clean the indexing database.
    #[derive(clap::Parser)]
    pub struct Clean {
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        clean_args: CleanArgs,
    }

    impl Clean {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let db_path = self.db_args.get_or(default_db_path);
            self.clean_args.run(&db_path)
        }
    }

    /// Index source files into the database.
    #[derive(clap::Parser)]
    pub struct Index {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        index_args: IndexArgs,
    }

    impl Index {
        pub fn run(
            self,
            default_db_path: PathBuf,
            configurations: Vec<LanguageConfiguration>,
        ) -> anyhow::Result<()> {
            let loader = self.load_args.get(configurations)?;
            let db_path = self.db_args.get_or(default_db_path);
            self.index_args.run(&db_path, loader)
        }
    }

    /// Inititialize a new stack graphs project for a tree-sitter language.
    #[derive(clap::Parser)]
    pub struct Init {
        #[clap(flatten)]
        init_args: InitArgs,
    }

    impl Init {
        pub fn run(self) -> anyhow::Result<()> {
            self.init_args.run()
        }
    }

    /// Run an LSP server.
    #[cfg(feature = "lsp")]
    #[derive(clap::Parser)]
    pub struct Lsp {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        lsp_args: LspArgs,
    }

    #[cfg(feature = "lsp")]
    impl Lsp {
        pub fn run(
            self,
            default_db_path: PathBuf,
            configurations: Vec<LanguageConfiguration>,
        ) -> anyhow::Result<()> {
            let loader = self.load_args.get(configurations)?;
            let db_path = self.db_args.get_or(default_db_path);
            self.lsp_args.run(db_path, loader)
        }
    }

    /// Match stanza queries against a source file.
    #[derive(clap::Parser)]
    pub struct Match {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        match_args: MatchArgs,
    }

    impl Match {
        pub fn run(self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let loader = self.load_args.get(configurations)?;
            self.match_args.run(loader)
        }
    }

    /// Parse a source file and show the parse tree.
    #[derive(clap::Parser)]
    pub struct Parse {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        parse_args: ParseArgs,
    }

    impl Parse {
        pub fn run(self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let loader = self.load_args.get(configurations)?;
            self.parse_args.run(loader)
        }
    }

    /// Query the database to resolve references.
    #[derive(clap::Parser)]
    pub struct Query {
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        query_args: QueryArgs,
    }

    impl Query {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let db_path = self.db_args.get_or(default_db_path);
            self.query_args.run(&db_path)
        }
    }

    /// Show indexing status for source files.
    #[derive(clap::Parser)]
    pub struct Status {
        #[clap(flatten)]
        db_args: DatabaseArgs,
        #[clap(flatten)]
        status_args: StatusArgs,
    }

    impl Status {
        pub fn run(self, default_db_path: PathBuf) -> anyhow::Result<()> {
            let db_path = self.db_args.get_or(default_db_path);
            self.status_args.run(&db_path)
        }
    }

    /// Run test files and show results.
    #[derive(clap::Parser)]
    pub struct Test {
        #[clap(flatten)]
        load_args: LanguageConfigurationsLoaderArgs,
        #[clap(flatten)]
        test_args: TestArgs,
    }

    impl Test {
        pub fn run(self, configurations: Vec<LanguageConfiguration>) -> anyhow::Result<()> {
            let loader = self.load_args.get(configurations)?;
            self.test_args.run(loader)
        }
    }
}
