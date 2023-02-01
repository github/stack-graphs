// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::Args;
use clap::ValueHint;
use dialoguer::Select;
use dialoguer::{Input, Validator};
use indoc::printdoc;
use indoc::writedoc;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

const TSSG_VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref VALID_NAME: Regex = Regex::new(r"^[a-zA-Z0-9_.-]+$").unwrap();
    static ref VALID_CRATE_NAME: Regex = Regex::new(r"^[a-zA-Z0-9_.-]+$").unwrap();
    static ref VALID_CRATE_VERSION: Regex = Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap();
}

/// Initialize project
#[derive(Args)]
pub struct InitArgs {
    /// Project directory path.
    #[clap(value_name = "PROJECT_PATH", required = false, default_value = ".", value_hint = ValueHint::AnyPath, parse(from_os_str))]
    pub project_path: PathBuf,
}

impl InitArgs {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        self.check_project_dir()?;
        let mut config = ProjectSettings::default();
        loop {
            config.read_from_console()?;
            println!("=== Review project settings ===");
            println!(
                "Project directory          : {}",
                self.project_path.display()
            );
            println!("{}", config);
            let action = Select::new()
                .items(&["Generate", "Edit", "Cancel"])
                .default(0)
                .interact()?;
            match action {
                0 => {
                    config.generate_files_into(&self.project_path)?;
                    println!(
                        "Project created. See {} to get started!",
                        self.project_path.join("README.md").display(),
                    );
                    break;
                }
                1 => {
                    continue;
                }
                2 => {
                    println!("No project created.");
                    break;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    fn check_project_dir(&self) -> anyhow::Result<()> {
        if !self.project_path.exists() {
            return Ok(());
        }
        if !self.project_path.is_dir() {
            return Err(anyhow!("Project path exists but is not a directory"));
        }
        if fs::read_dir(&self.project_path)?.next().is_some() {
            return Err(anyhow!("Project directory exists but is not empty"));
        }
        Ok(())
    }
}

#[derive(Default)]
struct ProjectSettings {
    language_name: String,
    language_id: String,
    language_file_extension: String,
    package_name: String,
    package_version: String,
    author: String,
    license: String,
    grammar_package_name: String,
    grammar_package_version: String,
}

impl ProjectSettings {
    fn read_from_console(&mut self) -> anyhow::Result<()> {
        printdoc! {r#"

            Give the name of the programming language the stack graphs definitions in this
            project will target. This name will appear in the project description and comments.
            "#
        };
        self.language_name = Input::new()
            .with_prompt("Language name")
            .with_initial_text(&self.language_name)
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give an identifier for {}. This identifier will be used for the suggested project
            name and suggested dependencies. May only contain letters, numbers, dashes, dots,
            and underscores.
            "#,
            self.language_name,
        };
        let default_language_id = self.language_name.to_lowercase();
        self.language_id = Input::new()
            .with_prompt("Language identifier")
            .with_initial_text(if self.language_id.is_empty() {
                &default_language_id
            } else {
                &self.language_id
            })
            .validate_with(regex_validator(&VALID_NAME))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the file extension for {}. This file extension will be used for stub files in
            the project. May only contain letters, numbers, dashes, dots, and underscores.
            "#,
            self.language_name,
        };
        self.language_file_extension = Input::new()
            .with_prompt("Language file extension")
            .with_initial_text(&self.language_file_extension)
            .validate_with(regex_validator(&VALID_NAME))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the package name for this project. Must be a valid Rust crate name.
            "#
        };
        let default_package_name = "tree-sitter-stack-graphs-".to_string() + &self.language_id;
        self.package_name = Input::new()
            .with_prompt("Package name")
            .with_initial_text(if self.package_name.is_empty() {
                &default_package_name
            } else {
                &self.package_name
            })
            .validate_with(regex_validator(&VALID_CRATE_NAME))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the package version for this project.
            "#
        };
        self.package_version = Input::new()
            .with_prompt("Package version")
            .with_initial_text(if self.package_version.is_empty() {
                "0.1.0"
            } else {
                &self.package_version
            })
            .validate_with(regex_validator(&VALID_CRATE_VERSION))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the project author in the format NAME <EMAIL>. Leave empty to omit.
            "#
        };
        self.author = Input::new()
            .with_prompt("Author")
            .with_initial_text(&self.author)
            .allow_empty(true)
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the project license as an SPDX expression. Leave empty to omit.
            "#
        };
        self.license = Input::new()
            .with_prompt("License")
            .with_initial_text(&self.license)
            .allow_empty(true)
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the crate name for the Tree-sitter grammar that is to be used for
            parsing.
            "#
        };
        let default_grammar_package_name = "tree-sitter-".to_string() + &self.language_id;
        self.grammar_package_name = Input::new()
            .with_prompt("Grammar package name")
            .with_initial_text(if self.grammar_package_name.is_empty() {
                &default_grammar_package_name
            } else {
                &self.grammar_package_name
            })
            .interact_text()?;
        println!();

        printdoc! {r##"

            Give the crate version the {} dependency. The format must be MAJOR.MINOR.PATCH.
            Prefix with ~ to allow any patch version, for example: ~0.4.1
            Prefix with ^ to allow any minor version, for example: ^1.2.7
            "##,
            self.grammar_package_name,
        };
        self.grammar_package_version = Input::new()
            .with_prompt("Grammar package version")
            .with_initial_text(&self.grammar_package_version)
            .interact_text()?;
        println!();

        Ok(())
    }

    fn generate_files_into(&self, project_path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(project_path)?;
        fs::create_dir_all(project_path.join("rust"))?;
        fs::create_dir_all(project_path.join("src"))?;
        fs::create_dir_all(project_path.join("test"))?;
        self.generate_readme(project_path)?;
        self.generate_changelog(project_path)?;
        self.generate_cargo_toml(project_path)?;
        self.generate_rust_bin(project_path)?;
        self.generate_rust_lib(project_path)?;
        self.generate_rust_test(project_path)?;
        self.generate_stack_graphs_tsg(project_path)?;
        self.generate_builtins_src(project_path)?;
        self.generate_builtins_cfg(project_path)?;
        self.generate_test(project_path)?;
        self.generate_gitignore(project_path)?;
        Ok(())
    }

    fn generate_readme(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("README.md"))?;
        writedoc! {file, r####"
            # tree-sitter-stack-graphs definition for {}

            This project defines tree-sitter-stack-graphs rules for {} using the [{}][] grammar.

            [{}]: https://crates.io/crates/{}

            ## Usage

            To use this library, add the following to your `Cargo.toml`:

            ``` toml
            [dependencies]
            {} = "{}"
            ```

            Check out our [documentation](https://docs.rs/{}/*/) for more details on how to use this library.

            ## Command-line Program

            The command-line program for `{}` lets you do stack graph based analysis and lookup from the command line.

            Install the program using `cargo install` as follows:

            ``` sh
            $ cargo install --features cli {}
            $ {} --help
            ```

            ## Development

            The project is written in Rust, and requires a recent version installed.  Rust can be installed and updated using [rustup][].

            [rustup]: https://rustup.rs/


            The project is organized as follows:

            - The stack graph rules are defined in `src/stack-graphs.tsg`.
            - Builtins sources and configuration are defined in `src/builtins.{}` and `builtins.cfg` respectively.
            - Tests are put into the `test` directory.

            Build the project by running:

            ``` sh
            $ cargo build
            ```

            Run the tests as follows:

            ``` sh
            $ cargo test
            ```

            The project consists of a library and a CLI. By default, running `cargo` only applies to the library. To run `cargo` commands on the CLI as well, add `--features cli` or `--all-features`.

            Run the CLI from source as follows:

            ``` sh
            $ cargo run --features cli -- ARGS
            ```

            Sources are formatted using the standard Rust formatted, which is applied by running:

            ``` sh
            $ cargo fmt
            ```

            The stack graph rules are written in [tree-sitter-graph][], which provides a VSCode extension for syntax highlighting.

            [tree-sitter-graph]: https://github.com/tree-sitter/tree-sitter-graph

            Parse and test a single file by executing the following commands:

            ``` sh
            $ cargo run --features cli -- parse FILES...
            $ cargo run --features cli -- test TESTFILES...
            ```

            Additional flags can be passed to these commands as well. For example, to generate a visualization for the test, execute:

            ``` sh
            $ cargo run --features cli -- test -V TESTFILES...
            ```

            To generate the visualization regardless of test outcome, execute:

            ``` sh
            $ cargo run --features cli -- test -V --output-mode=always TESTFILES...
            ```

            Go to https://crates.io/crates/tree-sitter-stack-graphs for links to examples and documentation.
            "####,
            self.language_name,
            self.language_name, self.grammar_package_name,
            self.grammar_package_name, self.grammar_package_name,
            self.package_name, self.package_version,
            self.package_name,
            self.package_name,
            self.package_name,
            self.package_name,
            self.language_file_extension,
        }?;
        Ok(())
    }

    fn generate_changelog(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("CHANGELOG.md"))?;
        writedoc! {file, r####"
            # Changelog for {}

            All notable changes to this project will be documented in this file.

            The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
            and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
            "####,
            self.package_name,
        }?;
        Ok(())
    }

    fn generate_cargo_toml(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("Cargo.toml"))?;
        writedoc! {file, r#"
            [package]
            name = "{}"
            version = "{}"
            description = "Stack graphs definition for {} using {}"
            readme = "README.md"
            keywords = ["tree-sitter", "stack-graphs", "{}"]
            "#,
            self.package_name,
            self.package_version,
            self.language_name, self.grammar_package_name,
            self.language_id
        }?;
        if !self.author.is_empty() {
            writeln!(file, r#"authors = ["{}"]"#, self.author)?;
        }
        if !self.license.is_empty() {
            writeln!(file, r#"license = "{}""#, self.license)?;
        }
        writedoc! {file, r#"
            edition = "2018"

            [[bin]]
            name = "{}"
            path = "rust/bin.rs"
            required-features = ["cli"]

            [lib]
            path = "rust/lib.rs"
            test = false

            [[test]]
            name = "test"
            path = "rust/test.rs"
            harness = false
            required-features = ["test"] # should be a forced feature, but Cargo does not support those

            [features]
            default = ["test"] # test is enabled by default because we cannot specify it as a forced featured for [[test]] above
            cli = ["anyhow", "clap", "tree-sitter-stack-graphs/cli"]
            test = ["anyhow", "tree-sitter-stack-graphs/cli"]

            [dependencies]
            anyhow = {{ version = "1.0", optional = true }}
            clap = {{ version = "3", optional = true }}
            tree-sitter-stack-graphs = "{}"
            {} = "{}"
            "#,
            self.package_name,
            TSSG_VERSION,
            self.grammar_package_name, self.grammar_package_version,
        }?;
        Ok(())
    }

    fn generate_rust_bin(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/bin.rs"))?;
        writedoc! {file, r#"
            use clap::Parser;
            use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
            use tree_sitter_stack_graphs::NoCancellation;

            fn main() -> anyhow::Result<()> {{
                let cli = Cli::parse();
                cli.subcommand.run(vec![
                    tree_sitter_stack_graphs_typescript::language_configuration(&NoCancellation),
                ])
            }}

            #[derive(Parser)]
            #[clap(about, version)]
            pub struct Cli {{
                #[clap(subcommand)]
                subcommand: Subcommands,
            }}
            "#
        }?;
        Ok(())
    }

    fn generate_rust_lib(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/lib.rs"))?;
        writedoc! {file, r#"
            use tree_sitter_stack_graphs::loader::FileAnalyzers;
            use tree_sitter_stack_graphs::loader::LanguageConfiguration;
            use tree_sitter_stack_graphs::CancellationFlag;

            /// The stack graphs tsg source for this language
            pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

            /// The stack graphs builtins configuration for this language
            pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
            /// The stack graphs builtins source for this language
            pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.ts");

            /// The name of the file path global variable
            pub const FILE_PATH_VAR: &str = "FILE_PATH";

            pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {{
                LanguageConfiguration::from_tsg_str(
                    {}::language(),
                    Some(String::from("source.{}")),
                    None,
                    vec![String::from("{}")],
                    STACK_GRAPHS_TSG_SOURCE,
                    Some(STACK_GRAPHS_BUILTINS_SOURCE),
                    Some(STACK_GRAPHS_BUILTINS_CONFIG),
                    FileAnalyzers::new(),
                    cancellation_flag,
                )
                .unwrap()
            }}
            "#,
            self.grammar_package_name.replace("-", "_"),
            self.language_file_extension,
            self.language_file_extension,
        }?;
        Ok(())
    }

    fn generate_rust_test(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/lib.rs"))?;
        writedoc! {file, r#"
            use std::path::PathBuf;
            use tree_sitter_stack_graphs::ci::Tester;
            use tree_sitter_stack_graphs::NoCancellation;

            fn main() -> anyhow::Result<()> {{
                let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
                Tester::new(
                    vec![{}::language_configuration(
                        &NoCancellation,
                    )],
                    vec![test_path],
                )
                .run()
            }}
            "#,
            self.package_name,
        }?;
        Ok(())
    }

    fn generate_stack_graphs_tsg(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("src/stack-graphs.tsg"))?;
        writedoc! {file, r#"
            ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
            ;; Stack graphs definition for {}
            ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

            ;; Global Variables
            ;; ^^^^^^^^^^^^^^^^

            global FILE_PATH
            global ROOT_NODE
            global JUMP_TO_SCOPE_NODE

            ;; Attribute Shorthands
            ;; ^^^^^^^^^^^^^^^^^^^^

            attribute node_definition = node        => type = "pop_symbol", node_symbol = node, is_definition
            attribute node_reference = node         => type = "push_symbol", node_symbol = node, is_reference
            attribute pop_node = node               => type = "pop_symbol", node_symbol = node
            attribute pop_scoped_node = node        => type = "pop_scoped_symbol", node_symbol = node
            attribute pop_scoped_symbol = symbol    => type = "pop_scoped_symbol", symbol = symbol
            attribute pop_symbol = symbol           => type = "pop_symbol", symbol = symbol
            attribute push_node = node              => type = "push_symbol", node_symbol = node
            attribute push_scoped_node = node       => type = "push_scoped_symbol", node_symbol = node
            attribute push_scoped_symbol = symbol   => type = "push_scoped_symbol", symbol = symbol
            attribute push_symbol = symbol          => type = "push_symbol", symbol = symbol
            attribute scoped_node_definition = node => type = "pop_scoped_symbol", node_symbol = node, is_definition
            attribute scoped_node_reference = node  => type = "push_scoped_symbol", node_symbol = node, is_reference
            attribute symbol_definition = symbol    => type = "pop_symbol", symbol = symbol, is_definition
            attribute symbol_reference = symbol     => type = "push_symbol", symbol = symbol, is_reference

            attribute node_symbol = node            => symbol = (source-text node), source_node = node

            ;; Stack Graph Rules
            ;; ^^^^^^^^^^^^^^^^^

            ; Have fun!
            "#,
            self.language_name,
        }?;
        Ok(())
    }

    fn generate_builtins_src(&self, project_path: &Path) -> anyhow::Result<()> {
        File::create(
            project_path.join("src/builtins.".to_string() + &self.language_file_extension),
        )?;
        Ok(())
    }

    fn generate_builtins_cfg(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("src/builtins.cfg"))?;
        writedoc! {file, r#"
            [globals]
            "#,
        }?;
        Ok(())
    }

    fn generate_test(&self, project_path: &Path) -> anyhow::Result<()> {
        File::create(project_path.join("test/test.".to_string() + &self.language_file_extension))?;
        Ok(())
    }

    fn generate_gitignore(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join(".gitignore"))?;
        writedoc! {file, r#"
            *.html
            /Cargo.lock
            /target
            "#,
        }?;
        Ok(())
    }
}

impl std::fmt::Display for ProjectSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writedoc! {f, r##"
            Language name              : {}
            Language identifier        : {}
            Language file extension    : {}
            Project package name       : {}
            Project package version    : {}
            Project author             : {}
            Project license            : {}
            Grammar dependency name    : {}
            Grammar dependency version : {}

            "##,
            self.language_name,
            self.language_id,
            self.language_file_extension,
            self.package_name,
            self.package_version,
            self.author,
            self.license,
            self.grammar_package_name,
            self.grammar_package_version,
        }
    }
}

fn regex_validator<'a>(regex: &'a Regex) -> impl Validator<String, Err = String> + 'a {
    struct RegexValidator<'a>(&'a Regex);
    impl Validator<String> for RegexValidator<'_> {
        type Err = String;
        fn validate(&mut self, input: &String) -> Result<(), Self::Err> {
            if !self.0.is_match(input) {
                return Err(format!("Invalid input value. Must match {}", self.0));
            }
            Ok(())
        }
    }
    RegexValidator(regex)
}
