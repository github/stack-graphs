// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use clap::ValueHint;
use stack_graphs::serde::NoFilter;
use stack_graphs::storage::SQLiteReader;
use stack_graphs::NoCancellation;
use std::path::Path;
use std::path::PathBuf;

/// Visualize database
#[derive(Args)]
pub struct VisualizeArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
    )]
    pub source_paths: Vec<PathBuf>,

    #[clap(
        long,
        short = 'o',
        value_name = "OUTPUT_PATH",
        value_hint = ValueHint::AnyPath,
        default_value = "stack-graph.html",
    )]
    pub output: PathBuf,
}

impl VisualizeArgs {
    pub fn run(self, db_path: &Path) -> anyhow::Result<()> {
        let cancellation_flag = &NoCancellation;
        let mut db = SQLiteReader::open(&db_path)?;
        for source_path in &self.source_paths {
            let source_path = source_path.canonicalize()?;
            db.load_graph_for_file_or_directory(&source_path, cancellation_flag)?;
        }
        let (graph, partials, db) = db.get();
        let html = graph.to_html_string("stack-graph", partials, db, &NoFilter)?;
        if let Some(dir) = self.output.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.output, html)?;
        println!("Visualization at {}", self.output.display());
        Ok(())
    }
}
