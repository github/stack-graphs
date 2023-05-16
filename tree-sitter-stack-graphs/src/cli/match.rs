// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::Args;
use clap::ValueHint;
use colored::Colorize;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter::CaptureQuantifier;
use tree_sitter::Node;

use crate::cli::parse::parse;
use crate::cli::parse::print_node;
use crate::cli::util::ExistingPathBufValueParser;
use crate::loader::FileReader;
use crate::loader::Loader;
use crate::NoCancellation;

const MAX_TEXT_LENGTH: usize = 16;

/// Match file
#[derive(Args)]
pub struct MatchArgs {
    /// Input file path.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser = ExistingPathBufValueParser,
    )]
    pub source_path: PathBuf,

    /// Only match stanza on the given line.
    #[clap(long, value_name = "LINE_NUMBER", short = 'S')]
    pub stanza: Vec<usize>,
}

impl MatchArgs {
    pub fn run(self, mut loader: Loader) -> anyhow::Result<()> {
        let mut file_reader = FileReader::new();
        let lc = match loader.load_for_file(&self.source_path, &mut file_reader, &NoCancellation)? {
            Some(lc) => lc,
            None => return Err(anyhow!("No stack graph language found")),
        };
        let source = file_reader.get(&self.source_path)?;
        let tree = parse(lc.language, &self.source_path, source)?;
        if self.stanza.is_empty() {
            lc.sgl.tsg.try_visit_matches(&tree, source, true, |mat| {
                print_matches(lc.sgl.tsg_path(), &self.source_path, source, mat)
            })?;
        } else {
            for line in &self.stanza {
                let stanza = lc
                    .sgl
                    .tsg
                    .stanzas
                    .iter()
                    .find(|s| s.range.start.row <= line - 1 && line - 1 <= s.range.end.row)
                    .ok_or_else(|| {
                        anyhow!("No stanza on {}:{}", lc.sgl.tsg_path().display(), line)
                    })?;
                stanza.try_visit_matches(&tree, source, |mat| {
                    print_matches(lc.sgl.tsg_path(), &self.source_path, source, mat)
                })?;
            }
        }
        Ok(())
    }
}

fn print_matches(
    tsg_path: &Path,
    source_path: &Path,
    source: &str,
    mat: tree_sitter_graph::Match,
) -> anyhow::Result<()> {
    println!(
        "{}: stanza query",
        format!(
            "{}:{}:{}",
            tsg_path.display(),
            mat.query_location().row + 1,
            mat.query_location().column + 1
        )
        .bold(),
    );
    {
        let full_capture = mat.full_capture();
        print!("  matched ");
        print_node(full_capture, true);
        print_node_text(full_capture, source_path, source)?;
        println!();
    }
    let width = mat
        .capture_names()
        .map(|n| n.len())
        .max()
        .unwrap_or_default();
    if width == 0 {
        return Ok(());
    }
    println!("  and captured");
    for (name, quantifier, nodes) in mat.named_captures() {
        for (idx, node) in nodes.enumerate() {
            if idx == 0 {
                print!(
                    "    @{}{}{} = ",
                    name,
                    quantifier_ch(quantifier),
                    " ".repeat(width - name.len())
                );
            } else {
                print!("     {}  | ", " ".repeat(width));
            }
            print_node(node, true);
            print_node_text(node, source_path, source)?;
            println!();
        }
    }
    Ok(())
}

fn print_node_text(node: Node, source_path: &Path, source: &str) -> anyhow::Result<()> {
    print!(", text: \"");
    let text = node.utf8_text(source.as_bytes())?;
    let summary: String = text
        .chars()
        .take(MAX_TEXT_LENGTH)
        .take_while(|c| *c != '\n')
        .collect();
    print!("{}", summary.blue());
    if summary.len() < text.len() {
        print!("{}", "...".dimmed());
    }
    print!("\"");
    print!(
        ", path: {}:{}:{}",
        source_path.display(),
        node.start_position().row + 1,
        node.start_position().column + 1
    );
    Ok(())
}

fn quantifier_ch(quantifier: CaptureQuantifier) -> char {
    match quantifier {
        CaptureQuantifier::Zero => '-',
        CaptureQuantifier::ZeroOrOne => '?',
        CaptureQuantifier::ZeroOrMore => '*',
        CaptureQuantifier::One => ' ',
        CaptureQuantifier::OneOrMore => '+',
    }
}
