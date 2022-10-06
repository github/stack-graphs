// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use clap::ValueHint;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter::Parser;
use tree_sitter_graph::parse_error::ParseError;
use tree_sitter_stack_graphs::loader::Loader;
use tree_sitter_stack_graphs::LoadError;

use crate::loader::LoaderArgs;
use crate::util::path_exists;

/// Parse file
#[derive(clap::Parser)]
pub struct Command {
    #[clap(flatten)]
    loader: LoaderArgs,

    /// Input file path.
    #[clap(value_name = "FILE_PATH", required = true, value_hint = ValueHint::AnyPath, parse(from_os_str), validator_os = path_exists)]
    file_path: PathBuf,
}

impl Command {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut loader = self.loader.new_loader()?;
        self.parse_file(&self.file_path, &mut loader)
            .with_context(|| format!("Error parsing file {}", self.file_path.display()))?;
        Ok(())
    }

    fn parse_file(&self, file_path: &Path, loader: &mut Loader) -> anyhow::Result<()> {
        let source = std::fs::read_to_string(file_path)?;
        let lang = match loader.load_tree_sitter_language_for_file(file_path, Some(&source))? {
            Some(sgl) => sgl,
            None => return Err(anyhow!("No stack graph language found")),
        };

        let mut parser = Parser::new();
        parser.set_language(lang)?;
        let tree = parser.parse(source, None).ok_or(LoadError::ParseError)?;
        let parse_errors = ParseError::into_all(tree);
        if parse_errors.errors().len() > 0 {
            return Err(anyhow!(LoadError::ParseErrors(parse_errors)));
        }
        let tree = parse_errors.into_tree();
        self.print_tree(tree);

        Ok(())
    }

    // From: https://github.com/tree-sitter/tree-sitter/blob/master/cli/src/parse.rs
    fn print_tree(&self, tree: tree_sitter::Tree) {
        let mut cursor = tree.walk();

        let mut needs_newline = false;
        let mut indent_level = 0;
        let mut did_visit_children = false;
        loop {
            let node = cursor.node();
            let is_named = node.is_named();
            if did_visit_children {
                if is_named {
                    print!(")");
                    needs_newline = true;
                }
                if cursor.goto_next_sibling() {
                    did_visit_children = false;
                } else if cursor.goto_parent() {
                    did_visit_children = true;
                    indent_level -= 1;
                } else {
                    break;
                }
            } else {
                if is_named {
                    if needs_newline {
                        print!("\n");
                    }
                    for _ in 0..indent_level {
                        print!("  ");
                    }
                    let start = node.start_position();
                    let end = node.end_position();
                    if let Some(field_name) = cursor.field_name() {
                        print!("{}: ", field_name);
                    }
                    print!(
                        "({} [{}, {}] - [{}, {}]",
                        node.kind(),
                        start.row,
                        start.column,
                        end.row,
                        end.column
                    );
                    needs_newline = true;
                }
                if cursor.goto_first_child() {
                    did_visit_children = false;
                    indent_level += 1;
                } else {
                    did_visit_children = true;
                }
            }
        }
        cursor.reset(tree.root_node());
        println!("");
    }
}
