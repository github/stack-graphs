// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::builder::StringValueParser;
use clap::builder::TypedValueParser;
use clap::error::ContextKind;
use clap::error::ContextValue;
use clap::error::ErrorKind;
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
use time::OffsetDateTime;

use self::license::*;

mod license;

const TSSG_VERSION: &str = env!("CARGO_PKG_VERSION");

static VALID_CRATE_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_-][a-zA-Z0-9_-]*$").unwrap());
static VALID_CRATE_VERSION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap());
static VALID_DEPENDENCY_VERSION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[~^]?[0-9]+(\.[0-9]+(\.[0-9]+)?)?$").unwrap());

#[derive(Args)]
pub struct InitArgs {
    /// Project directory path. (Or repository directory path when --internal is specified.)
    #[clap(
        value_name = "PROJECT_PATH",
        required = false,
        default_value = ".",
        value_hint = ValueHint::AnyPath,
    )]
    pub project_path: PathBuf,

    /// Disable console interaction. All input values must be provided through the appropriate options.
    #[clap(
        long,
        requires("language_name"),
        requires("language_id"),
        requires("language_file_extension"),
        // crate_name is optional
        // crate_version is optional
        // author is optional
        // license is optional
        // grammar_crate_name
        requires("grammar_crate_version")
    )]
    pub non_interactive: bool,

    /// Name of the target language.
    #[clap(long)]
    pub language_name: Option<String>,

    /// Identifier for the target language.
    #[clap(long, value_parser = RegexValidator(&VALID_CRATE_NAME))]
    pub language_id: Option<String>,

    /// File extension for files written in the target language.
    #[clap(long, value_parser = RegexValidator(&VALID_CRATE_NAME))]
    pub language_file_extension: Option<String>,

    /// Name for the generated crate. Default: tree-sitter-stack-graphs-LANGUAGE_ID
    #[clap(long, value_parser = RegexValidator(&VALID_CRATE_NAME))]
    pub crate_name: Option<String>,

    /// Version for the generated crate. Default: 0.1.0
    #[clap(long, value_parser = RegexValidator(&VALID_CRATE_VERSION))]
    pub crate_version: Option<String>,

    /// Author of the generated crate, in NAME <EMAIL> format.
    #[clap(long)]
    pub author: Option<String>,

    /// SPDX identifier for the license of the generated crate. Examples: MIT, Apache-2.0
    #[clap(long)]
    pub license: Option<String>,

    /// The crate name of the Tree-sitter grammar for the target language.
    #[clap(long, value_parser = RegexValidator(&VALID_CRATE_NAME))]
    pub grammar_crate_name: Option<String>,

    /// The crate version of the Tree-sitter grammar for the target language.
    #[clap(long, value_parser = RegexValidator(&VALID_DEPENDENCY_VERSION))]
    pub grammar_crate_version: Option<String>,

    /// Generate a project that is meant to be part of the official stack-graphs repository.
    /// Instead of the project path, the repository root must be specified. The project path,
    /// license, and dependencies will follow the repository conventions.
    #[clap(long, conflicts_with("crate_name"), conflicts_with("license"))]
    pub internal: bool,
}

impl InitArgs {
    pub fn run(self) -> anyhow::Result<()> {
        if self.internal {
            Self::check_repo_dir(&self.project_path)?;
        } else {
            Self::check_project_dir(&self.project_path)?;
        }
        let license = if self.internal {
            Some(INTERNAL_LICENSE)
        } else {
            self.license.map(|spdx| {
                DEFAULT_LICENSES
                    .iter()
                    .find(|l| l.0 == spdx)
                    .cloned()
                    .unwrap_or_else(|| new_license(spdx.into()))
            })
        };
        let mut config = ProjectSettings {
            language_name: self.language_name.unwrap_or_default(),
            language_id: self.language_id.unwrap_or_default(),
            language_file_extension: self.language_file_extension.unwrap_or_default(),
            crate_name: self.crate_name,
            crate_version: self.crate_version,
            author: self.author,
            license,
            grammar_crate_name: self.grammar_crate_name,
            grammar_crate_version: self.grammar_crate_version.unwrap_or_default(),
            internal: self.internal,
        };
        if !self.non_interactive && !Self::interactive(&self.project_path, &mut config)? {
            return Ok(());
        }
        let project_path = Self::effective_project_path(&self.project_path, &config);
        Self::check_project_dir(&project_path)?;
        config.generate_files_into(&project_path)?;
        Ok(())
    }

    fn check_project_dir(project_path: &Path) -> anyhow::Result<()> {
        if !project_path.exists() {
            return Ok(());
        }
        if !project_path.is_dir() {
            return Err(anyhow!("Project path exists but is not a directory"));
        }
        if fs::read_dir(&project_path)?.next().is_some() {
            return Err(anyhow!("Project directory exists but is not empty"));
        }
        Ok(())
    }

    fn check_repo_dir(project_path: &Path) -> anyhow::Result<()> {
        if !project_path.exists() {
            return Ok(());
        }
        if !project_path.is_dir() {
            return Err(anyhow!("Repository path exists but is not a directory"));
        }
        if !project_path.join("Cargo.toml").exists() {
            return Err(anyhow!(
                "Repository directory exists but is missing Cargo.toml"
            ));
        }
        Ok(())
    }

    fn effective_project_path(project_path: &Path, config: &ProjectSettings) -> PathBuf {
        if config.internal {
            project_path.join("languages").join(config.crate_name())
        } else {
            project_path.to_path_buf()
        }
    }

    fn interactive(project_path: &Path, config: &mut ProjectSettings) -> anyhow::Result<bool> {
        loop {
            Self::read_from_console(config)?;
            let project_path = Self::effective_project_path(project_path, config);
            println!();
            println!("=== Review project settings ===");
            println!("Project directory          : {}", project_path.display());
            println!("{}", config);
            let action = Select::new()
                .items(&["Generate", "Edit", "Cancel"])
                .default(0)
                .interact()?;
            match action {
                0 => {
                    println!(
                        "Project created. See {} to get started!",
                        project_path.join("README.md").display(),
                    );
                    return Ok(true);
                }
                1 => {
                    continue;
                }
                2 => {
                    println!("No project created.");
                    return Ok(false);
                }
                _ => unreachable!(),
            }
        }
    }

    fn read_from_console(config: &mut ProjectSettings) -> anyhow::Result<()> {
        printdoc! {r#"

            Give the name of the programming language the stack graphs definitions in this
            project will target. This name will appear in the project description and comments.
            "#
        };
        config.language_name = Input::new()
            .with_prompt("Language name")
            .with_initial_text(&config.language_name)
            .interact_text()?;

        printdoc! {r#"

            Give an identifier for {}. This identifier will be used for the suggested project
            name and suggested dependencies. May only contain letters, numbers, dashes, and
            underscores.
            "#,
            config.language_name,
        };
        let default_language_id = config.language_name.to_lowercase();
        config.language_id = Input::new()
            .with_prompt("Language identifier")
            .with_initial_text(if config.language_id.is_empty() {
                &default_language_id
            } else {
                &config.language_id
            })
            .validate_with(RegexValidator(&VALID_CRATE_NAME))
            .interact_text()?;

        printdoc! {r#"

            Give the file extension for {}. This file extension will be used for stub files in
            the project. May only contain letters, numbers, dashes, and underscores.
            "#,
            config.language_name,
        };
        let default_language_file_extension = if config.language_file_extension.is_empty() {
            &config.language_id
        } else {
            &config.language_file_extension
        };
        config.language_file_extension = Input::new()
            .with_prompt("Language file extension")
            .with_initial_text(default_language_file_extension)
            .validate_with(RegexValidator(&VALID_CRATE_NAME))
            .interact_text()?;

        printdoc! {r#"

            Give the crate name for this project. May only contain letters, numbers, dashes,
            and underscores.
        "#
        };
        config.crate_name = Some(
            Input::new()
                .with_prompt("Crate name")
                .with_initial_text(config.crate_name())
                .validate_with(RegexValidator(&VALID_CRATE_NAME))
                .interact_text()?,
        );

        printdoc! {r#"

            Give the crate version for this project. Must be in MAJOR.MINOR.PATCH format.
            "#
        };
        config.crate_version = Some(
            Input::new()
                .with_prompt("Crate version")
                .with_initial_text(config.crate_version())
                .validate_with(RegexValidator(&VALID_CRATE_VERSION))
                .interact_text()?,
        );

        printdoc! {r#"

            Give the project author in the format NAME <EMAIL>. Leave empty to omit.
            "#
        };
        let author: String = Input::new()
            .with_prompt("Author")
            .with_initial_text(config.author.clone().unwrap_or_default())
            .allow_empty(true)
            .interact_text()?;
        config.author = if author.is_empty() {
            None
        } else {
            Some(author)
        };

        config.license = if config.internal {
            Some(INTERNAL_LICENSE)
        } else {
            printdoc! {r#"

                Give the project license as an SPDX expression. Choose "Other" to input
                manually. Press ESC to deselect. See https://spdx.org/licenses/ for possible
                license identifiers.
            "#
            };
            let (selected, other, other_default) = if let Some(license) = &config.license {
                if let Some(selected) = DEFAULT_LICENSES.iter().position(|l| l.0 == license.0) {
                    (selected, "Other".to_string(), "")
                } else {
                    (
                        OTHER_LICENSE,
                        format!("Other ({})", license.0),
                        license.0.as_ref(),
                    )
                }
            } else {
                (NO_LICENSE, "Other".to_string(), "")
            };
            let selected = Select::new()
                .with_prompt("License")
                .items(&DEFAULT_LICENSES.iter().map(|l| &l.0).collect::<Vec<_>>())
                .item(&other)
                .item("None")
                .default(selected)
                .interact()?;
            if selected == NO_LICENSE {
                None
            } else if selected == OTHER_LICENSE {
                let spdx: String = Input::new()
                    .with_prompt("Other license")
                    .with_initial_text(other_default)
                    .allow_empty(true)
                    .interact_text()?;
                Some(new_license(spdx.into()))
            } else {
                Some(DEFAULT_LICENSES[selected].clone())
            }
        };

        printdoc! {r#"

            Give the crate name for the Tree-sitter grammar that is to be used for
            parsing. May only contain letters, numbers, dashes, and underscores.
            "#
        };
        config.grammar_crate_name = Some(
            Input::new()
                .with_prompt("Grammar crate name")
                .with_initial_text(config.grammar_crate_name())
                .interact_text()?,
        );

        printdoc! {r##"

            Give the crate version the {} dependency. This must be a valid Cargo
            dependency version. For example, 1.2, ^0.4.1, or ~3.2.4.
            See https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html.
            "##,
            config.grammar_crate_name(),
        };
        config.grammar_crate_version = Input::new()
            .with_prompt("Grammar crate version")
            .with_initial_text(&config.grammar_crate_version)
            .validate_with(RegexValidator(&VALID_DEPENDENCY_VERSION))
            .interact_text()?;

        Ok(())
    }
}

#[derive(Default)]
struct ProjectSettings<'a> {
    language_name: String,
    language_id: String,
    language_file_extension: String,
    crate_name: Option<String>,
    crate_version: Option<String>,
    author: Option<String>,
    license: Option<License<'a>>,
    grammar_crate_name: Option<String>,
    grammar_crate_version: String,
    internal: bool,
}

impl<'a> ProjectSettings<'a> {}

impl ProjectSettings<'_> {
    fn crate_name(&self) -> String {
        self.crate_name
            .clone()
            .unwrap_or_else(|| format!("tree-sitter-stack-graphs-{}", self.language_id))
    }

    fn crate_version(&self) -> String {
        self.crate_version
            .clone()
            .unwrap_or_else(|| "0.1.0".to_string())
    }

    fn package_name(&self) -> String {
        self.crate_name().replace("-", "_")
    }

    fn grammar_crate_name(&self) -> String {
        self.grammar_crate_name
            .clone()
            .unwrap_or_else(|| format!("tree-sitter-{}", self.language_id))
    }

    fn grammar_package_name(&self) -> String {
        self.grammar_crate_name().replace("-", "_")
    }

    fn license_author(&self) -> String {
        self.author
            .clone()
            .unwrap_or_else(|| format!("the {} authors", self.crate_name()))
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
            self.language_name, self.grammar_crate_name(),
            self.grammar_crate_name(), self.grammar_crate_name(),
            self.crate_name(), self.crate_version(),
            self.crate_name(),
            self.crate_name(),
            self.crate_name(),
            self.crate_name(),
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
            self.crate_name(),
        }?;
        Ok(())
    }

    fn generate_license(&self, project_path: &Path) -> std::io::Result<()> {
        if let Some(license) = &self.license {
            let mut file = File::create(project_path.join("LICENSE"))?;
            let year = OffsetDateTime::now_utc().year();
            let author = self.license_author();
            (license.2)(&mut file, year, &author)?;
        }
        Ok(())
    }

    fn write_license_header(&self, file: &mut File, prefix: &str) -> std::io::Result<()> {
        if let Some(license) = &self.license {
            let year = OffsetDateTime::now_utc().year();
            let author = self.license_author();
            (license.1)(file, year, &author, prefix)?;
        }
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
            self.crate_name(),
            self.crate_version(),
            self.language_name, self.grammar_crate_name(),
            self.language_id
        }?;
        if self.internal || self.author.is_some() {
            writeln!(file, r#"authors = ["#)?;
            if self.internal {
                writeln!(
                    file,
                    r#"    "GitHub <opensource+stack-graphs@github.com>","#
                )?;
            }
            if let Some(author) = &self.author {
                writeln!(file, r#"    "{}","#, author)?;
            }
            writeln!(file, r#"]"#)?;
        }
        if let Some(license) = &self.license {
            writeln!(file, r#"license = "{}""#, license.0)?;
        }
        let tssg_dep_fields = if self.internal {
            format!(
                r#"version = "{}", path = "../../tree-sitter-stack-graphs""#,
                TSSG_VERSION
            )
        } else {
            format!(r#"version = "{}""#, TSSG_VERSION)
        };
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

            [features]
            cli = ["anyhow", "clap", "tree-sitter-stack-graphs/cli"]

            [dependencies]
            anyhow = {{ version = "1.0", optional = true }}
            clap = {{ version = "4", optional = true, features = ["derive"] }}
            tree-sitter-stack-graphs = {{ {} }}
            {} = "{}"

            [dev-dependencies]
            anyhow = "1.0"
            tree-sitter-stack-graphs = {{ {}, features = ["cli"] }}
            "#,
            self.crate_name(),
            tssg_dep_fields,
            self.grammar_crate_name(), self.grammar_crate_version,
            tssg_dep_fields,
        }?;
        Ok(())
    }

    fn generate_rust_bin(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("rust/bin.rs"))?;
        self.write_license_header(&mut file, "// ")?;
        writedoc! {file, r#"
            use anyhow::anyhow;
            use clap::Parser;
            use tree_sitter_stack_graphs::cli::database::default_user_database_path_for_crate;
            use tree_sitter_stack_graphs::cli::provided_languages::Subcommands;
            use tree_sitter_stack_graphs::NoCancellation;

            fn main() -> anyhow::Result<()> {{
                let lc = match {}::try_language_configuration(&NoCancellation)
                {{
                    Ok(lc) => lc,
                    Err(err) => {{
                        eprintln!("{{}}", err.display_pretty());
                        return Err(anyhow!("Language configuration error"));
                    }}
                }};
                let cli = Cli::parse();
                let default_db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;
                cli.subcommand.run(default_db_path, vec![lc])
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
            use tree_sitter_stack_graphs::loader::LanguageConfiguration;
            use tree_sitter_stack_graphs::loader::LoadError;
            use tree_sitter_stack_graphs::CancellationFlag;

            /// The stack graphs tsg source for this language.
            pub const STACK_GRAPHS_TSG_PATH: &str = "src/stack-graphs.tsg";
            /// The stack graphs tsg source for this language.
            pub const STACK_GRAPHS_TSG_SOURCE: &str = include_str!("../src/stack-graphs.tsg");

            /// The stack graphs builtins configuration for this language.
            pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../src/builtins.cfg");
            /// The stack graphs builtins path for this language
            pub const STACK_GRAPHS_BUILTINS_PATH: &str = "src/builtins.{}";
            /// The stack graphs builtins source for this language.
            pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../src/builtins.{}");

            /// The name of the file path global variable.
            pub const FILE_PATH_VAR: &str = "FILE_PATH";

            pub fn language_configuration(cancellation_flag: &dyn CancellationFlag) -> LanguageConfiguration {{
                try_language_configuration(cancellation_flag).unwrap_or_else(|err| panic!("{{}}", err))
            }}

            pub fn try_language_configuration(
                cancellation_flag: &dyn CancellationFlag,
            ) -> Result<LanguageConfiguration, LoadError> {{
                LanguageConfiguration::from_sources(
                    {}::language(),
                    Some(String::from("source.{}")),
                    None,
                    vec![String::from("{}")],
                    STACK_GRAPHS_TSG_PATH.into(),
                    STACK_GRAPHS_TSG_SOURCE,
                    Some((
                        STACK_GRAPHS_BUILTINS_PATH.into(),
                        STACK_GRAPHS_BUILTINS_SOURCE,
                    )),
                    Some(STACK_GRAPHS_BUILTINS_CONFIG),
                    cancellation_flag,
                )
            }}
            "#,
            self.language_file_extension,
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
            use anyhow::anyhow;
            use std::path::PathBuf;
            use tree_sitter_stack_graphs::ci::Tester;
            use tree_sitter_stack_graphs::NoCancellation;

            fn main() -> anyhow::Result<()> {{
                let lc = match {}::try_language_configuration(&NoCancellation)
                {{
                    Ok(lc) => lc,
                    Err(err) => {{
                        eprintln!("{{}}", err.display_pretty());
                        return Err(anyhow!("Language configuration error"));
                    }}
                }};
                let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
                Tester::new(vec![lc], vec![test_path]).run()
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

impl std::fmt::Display for ProjectSettings<'_> {
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
            self.crate_name(),
            self.crate_version(),
            self.author.clone().unwrap_or_default(),
            self.license.as_ref().map_or("", |l| &l.0),
            self.grammar_crate_name(),
            self.grammar_crate_version,
        }
    }
}

#[derive(Clone)]
struct RegexValidator<'a>(&'a Regex);

impl TypedValueParser for RegexValidator<'static> {
    type Value = String;
    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner = StringValueParser::new();
        let value = inner.parse_ref(cmd, arg, value)?;

        if self.0.is_match(&value) {
            return Ok(value);
        }

        let mut err = clap::Error::new(ErrorKind::ValueValidation);
        if let Some(arg) = arg {
            err.insert(
                ContextKind::InvalidArg,
                ContextValue::String(arg.to_string()),
            );
        }
        err.insert(ContextKind::InvalidValue, ContextValue::String(value));
        err.insert(
            ContextKind::Custom,
            ContextValue::String(format!("value must match {}", self.0)),
        );

        Err(err)
    }
}

impl Validator<String> for RegexValidator<'_> {
    type Err = String;
    fn validate(&mut self, input: &String) -> Result<(), Self::Err> {
        if !self.0.is_match(input) {
            return Err(format!("Invalid input value. Must match {}", self.0));
        }
        Ok(())
    }
}
