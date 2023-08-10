// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use clap::ValueHint;
use stack_graphs::serde::NoFilter;
use stack_graphs::stitching::Database;
use stack_graphs::storage::SQLiteReader;
use stack_graphs::NoCancellation;
use std::path::Path;
use std::path::PathBuf;

/// Visualize database
#[derive(Args)]
#[clap(after_help = r#"LIMITATIONS:
    Visualizations will only work for very small stack graphs. This command is
    useful for debugging minimal examples, but running it on any real-world code
    will most likely result in HTML files that will not load in any browser.
"#)]
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
            db.load_graphs_for_file_or_directory(&source_path, cancellation_flag)?;
        }
        let (graph, _, _) = db.get();
        let starting_nodes = graph
            .iter_nodes()
            .filter(|n| graph[*n].is_reference())
            .collect::<Vec<_>>();
        let mut complete_paths_db = Database::new();
        db.find_all_complete_partial_paths(starting_nodes, cancellation_flag, |g, ps, p| {
            complete_paths_db.add_partial_path(g, ps, p.clone());
        })?;
        let (graph, partials, _) = db.get();
        let html =
            graph.to_html_string("stack-graph", partials, &mut complete_paths_db, &NoFilter)?;
        if let Some(dir) = self.output.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.output, html)?;
        println!("Visualization at {}", self.output.display());
        Ok(())
    }
}
