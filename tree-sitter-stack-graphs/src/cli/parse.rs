// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::Args;
use clap::ValueHint;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter::Parser;
use tree_sitter_graph::parse_error::ParseError;

use crate::cli::util::ExistingPathBufValueParser;
use crate::loader::FileReader;
use crate::loader::Loader;
use crate::util::DisplayParseErrorsPretty;
use crate::BuildError;

#[derive(Args)]
pub struct ParseArgs {
    /// Source file path to parse.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser = ExistingPathBufValueParser,
    )]
    pub source_path: PathBuf,
}

impl ParseArgs {
    pub fn run(self, mut loader: Loader) -> anyhow::Result<()> {
        let mut file_reader = FileReader::new();
        let lang =
            match loader.load_tree_sitter_language_for_file(&self.source_path, &mut file_reader)? {
                Some(sgl) => sgl,
                None => return Err(anyhow!("No stack graph language found")),
            };
        let source = file_reader.get(&self.source_path)?;
        let tree = parse(lang, &self.source_path, source)?;
        print_tree(tree);
        Ok(())
    }
}

pub(super) fn parse(
    language: tree_sitter::Language,
    path: &Path,
    source: &str,
) -> anyhow::Result<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser.set_language(language)?;
    let tree = parser.parse(source, None).ok_or(BuildError::ParseError)?;
    let parse_errors = ParseError::into_all(tree);
    if parse_errors.errors().len() > 0 {
        eprintln!(
            "{}",
            DisplayParseErrorsPretty {
                parse_errors: &parse_errors,
                path: path,
                source: &source,
                max_errors: crate::MAX_PARSE_ERRORS,
            }
        );
        return Err(anyhow!("Failed to parse file {}", path.display()));
    }
    Ok(parse_errors.into_tree())
}

// From: https://github.com/tree-sitter/tree-sitter/blob/master/cli/src/parse.rs
pub(super) fn print_tree(tree: tree_sitter::Tree) {
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
                if let Some(field_name) = cursor.field_name() {
                    print!("{}: ", field_name);
                }
                print_node(node, false);
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

pub(super) fn print_node(node: tree_sitter::Node, close: bool) {
    let start = node.start_position();
    let end = node.end_position();
    print!(
        "({} [{}:{} - {}:{}]",
        node.kind(),
        start.row + 1,
        start.column + 1,
        end.row + 1,
        end.column + 1
    );
    if close {
        print!(")");
    }
}
