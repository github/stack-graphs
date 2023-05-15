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
use tree_sitter::Tree;

use crate::cli::parse::parse;
use crate::cli::parse::print_node;
use crate::cli::util::ExistingPathBufValueParser;
use crate::loader::FileReader;
use crate::loader::Loader;
use crate::NoCancellation;

/// Match file
#[derive(Args)]
pub struct MatchArgs {
    /// Input file path.
    #[clap(
        value_name = "FILE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser = ExistingPathBufValueParser,
    )]
    pub file_path: PathBuf,
}

impl MatchArgs {
    pub fn run(self, mut loader: Loader) -> anyhow::Result<()> {
        let mut file_reader = FileReader::new();
        let lc = match loader.load_for_file(&self.file_path, &mut file_reader, &NoCancellation)? {
            Some(lc) => lc,
            None => return Err(anyhow!("No stack graph language found")),
        };
        let source = file_reader.get(&self.file_path)?;
        let tree = parse(lc.language, &self.file_path, source)?;
        print_matches(lc.sgl.tsg_path(), &lc.sgl.tsg, &tree, source)?;
        Ok(())
    }
}

fn print_matches(
    tsg_path: &Path,
    tsg: &tree_sitter_graph::ast::File,
    tree: &Tree,
    source: &str,
) -> anyhow::Result<()> {
    tsg.try_visit_matches(tree, source, true, |mat| {
        println!(
            "{}:{}:{}: stanza query",
            tsg_path.display(),
            mat.query_location().row + 1,
            mat.query_location().column + 1,
        );
        {
            let full_capture = mat.full_capture();
            print!("  matched ");
            print_node(full_capture, true);
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
        println!("  captured");
        for (name, _, nodes) in mat.named_captures() {
            let mut first = true;
            for node in nodes {
                if first {
                    first = false;
                    print!("    @{}{} = ", name, " ".repeat(width - name.len()));
                } else {
                    print!("     {} | ", " ".repeat(width));
                }
                print_node(node, true);
                println!();
            }
        }
        Ok(())
    })
}
