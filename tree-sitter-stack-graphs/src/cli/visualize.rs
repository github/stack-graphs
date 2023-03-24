// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context;
use clap::Args;
use clap::ValueHint;
use stack_graphs::serde::NoFilter;
use stack_graphs::stitching::Database;
use stack_graphs::stitching::ForwardPartialPathStitcher;
use stack_graphs::storage::SQLiteReader;
use stack_graphs::NoCancellation;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::util::path_exists;
use super::util::wait_for_input;
use super::util::FileStatusLogger;

/// Analyze sources
#[derive(Args)]
pub struct VisualizeArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub source_paths: Vec<PathBuf>,

    #[clap(
        long,
        short = 'D',
        value_name = "DATABASE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub database: PathBuf,

    #[clap(
        long,
        short = 'o',
        value_name = "OUTPUT_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        default_value = "stack-graph.html",
    )]
    pub output: PathBuf,

    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Wait for user input before starting analysis. Useful for profiling.
    #[clap(long)]
    pub wait_at_start: bool,
}

impl VisualizeArgs {
    pub fn run(&self) -> anyhow::Result<()> {
        if self.wait_at_start {
            wait_for_input()?;
        }

        let mut db = SQLiteReader::open(&self.database)?;
        for source_path in &self.source_paths {
            let source_path = source_path.canonicalize()?;
            if source_path.is_dir() {
                let source_root = &source_path;
                for source_entry in WalkDir::new(source_root)
                    .follow_links(true)
                    .sort_by_file_name()
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let source_path = source_entry.path().canonicalize()?;
                    self.load_file_data(&source_path, &mut db)?;
                }
            } else {
                self.load_file_data(&source_path, &mut db)?;
            }
        }

        self.create_html(&mut db)?;

        Ok(())
    }

    fn load_file_data(&self, source_path: &Path, db: &mut SQLiteReader) -> anyhow::Result<()> {
        let mut file_status = FileStatusLogger::new(source_path, self.verbose);
        let source_path = source_path.to_string_lossy();

        if !db.file_exists(&source_path, None)? {
            file_status.info("not indexed")?;
            return Ok(());
        }

        file_status.processing()?;

        db.load_graph_for_file(&source_path)?;
        let file = db
            .get()
            .0
            .get_file(&source_path)
            .ok_or_else(|| anyhow!("could not load {}", source_path))?;
        db.load_all_paths_for_file(file)?;

        file_status.ok("loaded")?;

        Ok(())
    }

    fn create_html(&self, db: &mut SQLiteReader) -> anyhow::Result<()> {
        let (graph, partials, db) = db.get();
        let mut db = {
            let mut complete_paths_db = Database::new();
            ForwardPartialPathStitcher::find_all_complete_partial_paths(
                graph,
                partials,
                db,
                graph.iter_nodes(),
                &NoCancellation,
                |g, ps, p| {
                    complete_paths_db.add_partial_path(g, ps, p.clone());
                },
            )?;
            complete_paths_db
        };

        let html = graph.to_html_string("stack-graph", partials, &mut db, &NoFilter)?;
        if let Some(dir) = self.output.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.output, html)
            .with_context(|| format!("Unable to write graph {}", self.output.display()))?;

        Ok(())
    }
}
