// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines CLI

pub(self) const MAX_PARSE_ERRORS: usize = 5;

pub mod init;
pub mod load;
pub mod parse;
pub mod test;
mod util;

pub use ci::Tester as CiTester;
pub use path_loading::Cli as PathLoadingCli;
pub use provided_languages::Cli as LanguageConfigurationsCli;

mod path_loading {
    use anyhow::Result;
    use clap::Parser;
    use clap::Subcommand;

    use crate::cli::init::InitArgs;
    use crate::cli::load::PathLoaderArgs;
    use crate::cli::parse::ParseArgs;
    use crate::cli::test::TestArgs;

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

mod provided_languages {
    use anyhow::Result;
    use clap::Parser;
    use clap::Subcommand;

    use crate::cli::parse::ParseArgs;
    use crate::cli::test::TestArgs;
    use crate::loader::LanguageConfiguration;

    use super::load::LanguageConfigurationsLoaderArgs;

    /// CLI implementation that loads from provided grammars and stack graph definitions.
    #[derive(Parser)]
    #[clap(about, version)]
    pub struct Cli {
        #[clap(subcommand)]
        command: Commands,
    }

    impl Cli {
        pub fn main(configurations: Vec<LanguageConfiguration>) -> Result<()> {
            let cli = Cli::parse();
            match &cli.command {
                Commands::Parse(cmd) => cmd.run(configurations),
                Commands::Test(cmd) => cmd.run(configurations),
            }
        }
    }

    #[derive(Subcommand)]
    enum Commands {
        Parse(Parse),
        Test(Test),
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

mod ci {
    use std::path::PathBuf;

    use crate::cli::test::TestArgs;
    use crate::loader::{LanguageConfiguration, Loader};

    /// Run tests for the given languages. Test locations are reported relative to the current directory, which
    /// results in better readable output when build tools only provides absolute test paths.
    pub struct Tester {
        configurations: Vec<LanguageConfiguration>,
        test_paths: Vec<PathBuf>,
    }

    impl Tester {
        pub fn new(configurations: Vec<LanguageConfiguration>, test_paths: Vec<PathBuf>) -> Self {
            Self {
                configurations,
                test_paths,
            }
        }

        pub fn run(self) -> anyhow::Result<()> {
            let test_paths = self
                .test_paths
                .into_iter()
                .map(|test_path| {
                    std::env::current_dir()
                        .ok()
                        .and_then(|cwd| pathdiff::diff_paths(&test_path, &cwd))
                        .unwrap_or(test_path)
                })
                .collect::<Vec<_>>();
            for test_path in &test_paths {
                if !test_path.exists() {
                    panic!("Test path {} does not exist", test_path.display());
                }
            }
            let mut loader = Loader::from_language_configurations(self.configurations, None)
                .expect("Expected loader");
            TestArgs::new(test_paths).run(&mut loader)
        }
    }
}
