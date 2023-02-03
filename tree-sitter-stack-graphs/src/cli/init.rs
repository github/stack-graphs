// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use chrono::Datelike;
use clap::Args;
use clap::ValueHint;
use dialoguer::Input;
use dialoguer::Select;
use dialoguer::Validator;
use indoc::printdoc;
use indoc::writedoc;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use self::license::lookup_license;
use self::license::DEFAULT_LICENSES;
use self::license::NO_LICENSE;
use self::license::OTHER_LICENSE;

mod license;

const TSSG_VERSION: &str = env!("CARGO_PKG_VERSION");

static VALID_CRATE_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_-][a-zA-Z0-9_-]*$").unwrap());
static VALID_CRATE_VERSION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap());
static VALID_DEPENDENCY_VERSION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[~^]?[0-9]+(\.[0-9]+(\.[0-9]+)?)?$").unwrap());

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
            println!();
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
    crate_name: String,
    crate_version: String,
    author: String,
    license: String,
    grammar_crate_name: String,
    grammar_crate_version: String,
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

        printdoc! {r#"

            Give an identifier for {}. This identifier will be used for the suggested project
            name and suggested dependencies. May only contain letters, numbers, dashes, and
            underscores.
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
            .validate_with(regex_validator(&VALID_CRATE_NAME))
            .interact_text()?;

        printdoc! {r#"

            Give the file extension for {}. This file extension will be used for stub files in
            the project. May only contain letters, numbers, dashes, and underscores.
            "#,
            self.language_name,
        };
        let default_language_file_extension = if self.language_file_extension.is_empty() {
            &self.language_id
        } else {
            &self.language_file_extension
        };
        self.language_file_extension = Input::new()
            .with_prompt("Language file extension")
            .with_initial_text(default_language_file_extension)
            .validate_with(regex_validator(&VALID_CRATE_NAME))
            .interact_text()?;

        printdoc! {r#"

            Give the crate name for this project. May only contain letters, numbers, dashes,
            and underscores.
            "#
        };
        let default_crate_name = "tree-sitter-stack-graphs-".to_string() + &self.language_id;
        self.crate_name = Input::new()
            .with_prompt("Package name")
            .with_initial_text(if self.crate_name.is_empty() {
                &default_crate_name
            } else {
                &self.crate_name
            })
            .validate_with(regex_validator(&VALID_CRATE_NAME))
            .interact_text()?;

        printdoc! {r#"

            Give the crate version for this project. Must be in MAJOR.MINOR.PATCH format.
            "#
        };
        self.crate_version = Input::new()
            .with_prompt("Package version")
            .with_initial_text(if self.crate_version.is_empty() {
                "0.1.0"
            } else {
                &self.crate_version
            })
            .validate_with(regex_validator(&VALID_CRATE_VERSION))
            .interact_text()?;

        printdoc! {r#"

            Give the project author in the format NAME <EMAIL>. Leave empty to omit.
            "#
        };
        self.author = Input::new()
            .with_prompt("Author")
            .with_initial_text(&self.author)
            .allow_empty(true)
            .interact_text()?;

        printdoc! {r#"

            Give the project license as an SPDX expression. Choose "Other" to input
            manually. Press ESC to deselect. See https://spdx.org/licenses/ for possible
            license identifiers.
            "#
        };
        let selected = lookup_license(&self.license);
        let (other, other_default) = if selected == OTHER_LICENSE {
            (format!("Other ({})", self.license), self.license.as_ref())
        } else {
            ("Other".to_string(), "")
        };
        let selected = Select::new()
            .with_prompt("License")
            .items(&DEFAULT_LICENSES.iter().map(|l| l.0).collect::<Vec<_>>())
            .item(&other)
            .item("None")
            .default(selected)
            .interact()?;
        self.license = if selected == NO_LICENSE {
            "".to_string()
        } else if selected == OTHER_LICENSE {
            Input::new()
                .with_prompt("Other license")
                .with_initial_text(other_default)
                .allow_empty(true)
                .interact_text()?
        } else {
            DEFAULT_LICENSES[selected].0.to_string()
        };

        printdoc! {r#"

            Give the crate name for the Tree-sitter grammar that is to be used for
            parsing. May only contain letters, numbers, dashes, and underscores.
            "#
        };
        let default_grammar_crate_name = "tree-sitter-".to_string() + &self.language_id;
        self.grammar_crate_name = Input::new()
            .with_prompt("Grammar crate name")
            .with_initial_text(if self.grammar_crate_name.is_empty() {
                &default_grammar_crate_name
            } else {
                &self.grammar_crate_name
            })
            .interact_text()?;

        printdoc! {r##"

            Give the crate version the {} dependency. This must be a valid Cargo
            dependency version. For example, 1.2, ^0.4.1, or ~3.2.4.
            See https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html.
            "##,
            self.grammar_crate_name,
        };
        self.grammar_crate_version = Input::new()
            .with_prompt("Grammar crate version")
            .with_initial_text(&self.grammar_crate_version)
            .validate_with(regex_validator(&VALID_DEPENDENCY_VERSION))
            .interact_text()?;

        Ok(())
    }

    fn package_name(&self) -> String {
        self.crate_name.replace("-", "_")
    }

    fn grammar_package_name(&self) -> String {
        self.grammar_crate_name.replace("-", "_")
    }

    fn license_author(&self) -> String {
        if self.author.is_empty() {
            format!("the {} authors", self.crate_name)
        } else {
            self.author.clone()
        }
    }

    fn generate_files_into(&self, project_path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(project_path)?;
        fs::create_dir_all(project_path.join("rust"))?;
        fs::create_dir_all(project_path.join("src"))?;
        fs::create_dir_all(project_path.join("test"))?;
        self.generate_readme(project_path)?;
        self.generate_changelog(project_path)?;
        self.generate_license(project_path)?;
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

            ### Building and Running Tests

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

            ### Writing TSG

            The stack graph rules are written in [tree-sitter-graph][]. Checkout the [examples][],
            which contain self-contained TSG rules for specific language features. A VSCode
            [extension][] is available that provides syntax highlighting for TSG files.

            [tree-sitter-graph]: https://github.com/tree-sitter/tree-sitter-graph
            [examples]: https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/examples/
            [extension]: https://marketplace.visualstudio.com/items?itemName=tree-sitter.tree-sitter-graph

            Parse and test a single file by executing the following commands:

            ``` sh
            $ cargo run --features cli -- parse FILES...
            $ cargo run --features cli -- test TESTFILES...
            ```

            Generate a visualization to debug failing tests by passing the `-V` flag:

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
            self.language_name, self.grammar_crate_name,
            self.grammar_crate_name, self.grammar_crate_name,
            self.crate_name, self.crate_version,
            self.crate_name,
            self.crate_name,
            self.crate_name,
            self.crate_name,
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
            self.crate_name,
        }?;
        Ok(())
    }

    fn generate_license(&self, project_path: &Path) -> std::io::Result<()> {
        match lookup_license(&self.license) {
            NO_LICENSE | OTHER_LICENSE => {}
            selected => {
                let mut file = File::create(project_path.join("LICENSE"))?;
                let year = chrono::Utc::now().year();
                let author = self.license_author();
                (DEFAULT_LICENSES[selected].2)(&mut file, year, &author)?;
            }
        };
        Ok(())
    }

    fn write_license_header(&self, file: &mut File, prefix: &str) -> std::io::Result<()> {
        match lookup_license(&self.license) {
            NO_LICENSE | OTHER_LICENSE => {}
            selected => {
                let year = chrono::Utc::now().year();
                let author = self.license_author();
                (DEFAULT_LICENSES[selected].1)(file, year, &author, prefix)?;
            }
        };
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
            self.crate_name,
            self.crate_version,
            self.language_name, self.grammar_crate_name,
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
            self.crate_name,
            TSSG_VERSION,
            self.grammar_crate_name, self.grammar_crate_version,
        }?;
        Ok(())
    }

    fn generate_rust_bin(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/bin.rs"))?;
        self.write_license_header(&mut file, "// ")?;
        writedoc! {file, r#"
            use clap::Parser;
            use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
            use tree_sitter_stack_graphs::NoCancellation;

            fn main() -> anyhow::Result<()> {{
                let cli = Cli::parse();
                cli.subcommand.run(vec![
                    {}::language_configuration(&NoCancellation),
                ])
            }}

            #[derive(Parser)]
            #[clap(about, version)]
            pub struct Cli {{
                #[clap(subcommand)]
                subcommand: Subcommands,
            }}
            "#,
            self.package_name(),
        }?;
        Ok(())
    }

    fn generate_rust_lib(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/lib.rs"))?;
        self.write_license_header(&mut file, "// ")?;
        writedoc! {file, r#"
            use tree_sitter_stack_graphs::loader::FileAnalyzers;
            use tree_sitter_stack_graphs::loader::LanguageConfiguration;
            use tree_sitter_stack_graphs::CancellationFlag;

            /// The stack graphs tsg source for this language
            pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

            /// The stack graphs builtins configuration for this language
            pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
            /// The stack graphs builtins source for this language
            pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.{}");

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
            self.language_file_extension,
            self.grammar_package_name(),
            self.language_file_extension,
            self.language_file_extension,
        }?;
        Ok(())
    }

    fn generate_rust_test(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/test.rs"))?;
        self.write_license_header(&mut file, "// ")?;
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
            self.package_name(),
        }?;
        Ok(())
    }

    fn generate_stack_graphs_tsg(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("src/stack-graphs.tsg"))?;
        self.write_license_header(&mut file, ";; ")?;
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
            self.crate_name,
            self.crate_version,
            self.author,
            self.license,
            self.grammar_crate_name,
            self.grammar_crate_version,
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
