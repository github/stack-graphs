// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::ValueHint;
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
    static ref VALID_NPM_PKG: Regex = Regex::new(r"^(@[a-zA-Z0-9_.-]+/)?[a-zA-Z0-9_.-]+$").unwrap();
    static ref VALID_NPM_VERSION: Regex = Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap();
}

/// Initialize project
#[derive(clap::Parser)]
pub struct Command {
    /// Project directory path.
    #[clap(value_name = "PROJECT_PATH", required = false, default_value = ".", value_hint = ValueHint::AnyPath, parse(from_os_str))]
    project_path: PathBuf,
}

impl Command {
    pub fn run(&self) -> anyhow::Result<()> {
        self.check_project_dir()?;
        let config = ProjectSettings::read_from_console()?;
        config.generate_files_into(&self.project_path)?;
        printdoc! {r#"

            Project created in {}.
            See {} to get started!
            "#,
            self.project_path.display(),
            self.project_path.join("README.md").display(),
        };
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

struct ProjectSettings {
    language_name: String,
    language_id: String,
    language_file_extension: String,
    project_npm_name: String,
    project_npm_version: String,
    project_author: String,
    project_license: String,
    grammar_npm_name: String,
    grammar_npm_version: String,
}

impl ProjectSettings {
    fn read_from_console() -> anyhow::Result<Self> {
        printdoc! {r#"

            Give the name of the programming language the stack graphs definitions in this
            project will target. This name will appear in the project description and comments.
            "#
        };
        let language_name: String = Input::new().with_prompt("Language name").interact_text()?;
        println!();

        printdoc! {r#"

            Give an identifier for {}. This identifier will be used for the suggested project
            name and suggested dependencies. May only contain letters, numbers, dashes, dots,
            and underscores.
            "#,
            language_name,
        };
        let language_id: String = Input::new()
            .with_prompt("Language identifier")
            .with_initial_text(language_name.to_lowercase())
            .validate_with(regex_validator(&VALID_NAME))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the file extension for {}. This file extension will be used for stub files in
            the project. May only contain letters, numbers, dashes, dots, and underscores.
            "#,
            language_name,
        };
        let language_file_extension: String = Input::new()
            .with_prompt("Language file extension")
            .validate_with(regex_validator(&VALID_NAME))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the NPM package name for this project. Must be a valid scoped or unscoped
            package name.
            "#
        };
        let project_npm_name: String = Input::new()
            .with_prompt("Project NPM package name")
            .with_initial_text("tree-sitter-stack-graphs-".to_string() + &language_id)
            .validate_with(regex_validator(&VALID_NPM_PKG))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the NPM package version for this project.
            "#
        };
        let project_npm_version: String = Input::new()
            .with_prompt("Project NPM package version")
            .with_initial_text("0.1.0")
            .validate_with(regex_validator(&VALID_NPM_VERSION))
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the project author in the format NAME <EMAIL>. Leave empty to omit.
            "#
        };
        let project_author: String = Input::new()
            .with_prompt("Author")
            .allow_empty(true)
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the project license as an SPDX expression. Leave empty to omit.
            "#
        };
        let project_license: String = Input::new()
            .with_prompt("License")
            .allow_empty(true)
            .interact_text()?;
        println!();

        printdoc! {r#"

            Give the NPM package name for the Tree-sitter grammar that is to be used for
            parsing.
            "#
        };
        let grammar_npm_name: String = Input::new()
            .with_prompt("Grammar NPM package name")
            .with_initial_text("tree-sitter-".to_string() + &language_id)
            .interact_text()?;
        println!();

        printdoc! {r##"

            Give the NPM package version or dependency string for the {} dependency. The
            format can be any of:
             - MAJOR.MINOR.PATCH                    An NPM release version
             - github:OWNER/REPOSITORY#COMMITISH    A GitHub dependency, tagged to a branch, tag, or commit SHA
            "##,
            grammar_npm_name,
        };
        let grammar_npm_version: String = Input::new()
            .with_prompt("Grammar NPM package version")
            .interact_text()?;
        println!();

        Ok(ProjectSettings {
            language_name,
            language_id,
            language_file_extension,
            project_npm_name,
            project_npm_version,
            project_author,
            project_license,
            grammar_npm_name,
            grammar_npm_version,
        })
    }

    fn generate_files_into(&self, project_path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(project_path)?;
        fs::create_dir_all(project_path.join("src"))?;
        fs::create_dir_all(project_path.join("test"))?;
        fs::create_dir_all(project_path.join("bindings/rust"))?;
        self.generate_readme(project_path)?;
        self.generate_package_json(project_path)?;
        self.generate_cargo_toml(project_path)?;
        self.generate_rust_lib(project_path)?;
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

            This project defines tree-sitter-stack-graphs rules for {} using the [{}](https://www.npmjs.com/package/{}) grammar.

            ## Development

            The project is organized as follows:

            - The stack graph rules are defined in `src/stack-graphs.tsg`.
            - Builtins sources and configuration are defined in `src/builtins.{}` and `builtins.cfg` respectively.
            - Tests are put into the `test` directory.

            Make sure all development dependencies are installed by running:

                npm install

            Run all tests in the project by executing the following:

                npm test

            Parse and test a single file by executing the following commands:

                npm run parse-file test/test.{}
                npm run test-file test/test.{}

            Additional flags can be passed to these commands as well. For example, to generate a visualization for the test, execute:

                npm run test-file -- -V test/test.{}

            To generate the visualization regardless of test outcome, execute:

                npm run test-file -- -V --output-mode=always test/test.{}

            These commands should be enough for regular development. If necessary, the `tree-sitter-stack-graphs` command can be invoked directly as well, by executing:

                npx tree-sitter-stack-graphs

            Go to https://crates.io/crates/tree-sitter-stack-graphs for links to examples and documentation.
            "####,
            self.language_name,
            self.language_name, self.grammar_npm_name, self.grammar_npm_name,
            self.language_file_extension,
            self.language_file_extension,
            self.language_file_extension,
            self.language_file_extension,
            self.language_file_extension,
        }?;
        Ok(())
    }

    fn generate_package_json(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("package.json"))?;
        writedoc! {file, r##"
            {{
                "name": "{}",
                "version": "{}",
                "description": "Stack graphs definition for {} using {}",
            "##,
            self.project_npm_name,
            self.project_npm_version,
            self.language_name, self.grammar_npm_name,
        }?;
        if !self.project_author.is_empty() {
            writeln!(file, r#"    "author": "{}","#, self.project_author)?;
        }
        if !self.project_license.is_empty() {
            writeln!(file, r#"    "license": "{}","#, self.project_license)?;
        }
        writedoc! {file, r##"
                "keywords": [
                    "tree-sitter",
                    "stack-graphs",
                    "{}"
                ],
                "devDependencies": {{
                    "tree-sitter-stack-graphs": "{}",
                    "{}": "{}"
                }},
                "scripts": {{
                    "test": "npx tree-sitter-stack-graphs test --grammar node_modules/{} --tsg src/stack-graphs --builtins src/builtins test",
                    "parse-file": "npx tree-sitter-stack-graphs parse --grammar node_modules/{}",
                    "test-file": "npx tree-sitter-stack-graphs test --grammar node_modules/{} --tsg src/stack-graphs --builtins src/builtins"
                }}
            }}
            "##,
            self.language_id,
            TSSG_VERSION,
            self.grammar_npm_name, self.grammar_npm_version,
            self.grammar_npm_name,
            self.grammar_npm_name,
            self.grammar_npm_name,
        }?;
        Ok(())
    }

    fn generate_cargo_toml(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("bindings/rust/Cargo.toml"))?;
        writedoc! {file, r#"
            [package]
            name = "{}"
            version = "{}"
            description = "Stack graphs definition for {} using {}"
            readme = "README.md"
            keywords = ["tree-sitter", "stack-graphs", "{}"]
            "#,
            self.project_npm_name,
            self.project_npm_version,
            self.language_name, self.grammar_npm_name,
            self.language_id
        }?;
        if !self.project_author.is_empty() {
            writeln!(file, r#"authors = ["{}"]"#, self.project_author)?;
        }
        if !self.project_license.is_empty() {
            writeln!(file, r#"license = "{}""#, self.project_license)?;
        }
        writedoc! {file, r#"
            edition = "2018"

            include = [
                "bindings/rust",
                "src"
            ]

            [lib]
            path = "bindings/rust/lib.rs"
            "#,
        }?;
        Ok(())
    }

    fn generate_rust_lib(&self, project_path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(project_path.join("bindings/rust/lib.rs"))?;
        writedoc! {file, r#"
            /// The stack graphs query for this language
            pub const STACK_GRAPHS_QUERY: &str = include_str!("../../src/stack-graphs.tsg");

            /// The stack graphs builtins configuration for this language
            pub const STACK_GRAPHS_BUILTINS_CONFIG: &str = include_str!("../../src/builtins.cfg");
            /// The stack graphs builtins source for this language
            pub const STACK_GRAPHS_BUILTINS_SOURCE: &str = include_str!("../../src/builtins.ts");
            "#
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
            /Cargo.lock
            /node_modules
            /package-lock.json
            /target
            "#,
        }?;
        Ok(())
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
