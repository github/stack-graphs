// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use clap::Args;
use clap::ValueHint;
use colored::Colorize as _;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tree_sitter_graph::Variables;
use walkdir::WalkDir;

use crate::cli::util::duration_from_seconds_str;
use crate::cli::util::map_parse_errors;
use crate::cli::util::path_exists;
use crate::loader::Loader;
use crate::CancelAfterDuration;
use crate::CancellationFlag;
use crate::LoadError;
use crate::NoCancellation;

/// Analyze sources
#[derive(Args)]
pub struct AnalyzeArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub source_paths: Vec<PathBuf>,

    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Maximum runtime per file in seconds.
    #[clap(
        long,
        value_name = "SECONDS",
        parse(try_from_str = duration_from_seconds_str),
        require_equals = true,
    )]
    pub max_file_time: Option<Duration>,
}

impl AnalyzeArgs {
    pub fn new(source_paths: Vec<PathBuf>) -> Self {
        Self {
            source_paths,
            verbose: false,
            max_file_time: None,
        }
    }

    pub fn run(&self, loader: &mut Loader) -> anyhow::Result<()> {
        for source_path in &self.source_paths {
            if source_path.is_dir() {
                let source_root = source_path;
                for source_entry in WalkDir::new(source_root)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let source_path = source_entry.path();
                    self.run_with_context(source_root, source_path, loader)?;
                }
            } else {
                let source_root = source_path.parent().unwrap();
                self.run_with_context(source_root, source_path, loader)?;
            }
        }
        Ok(())
    }

    /// Run test file and add error context to any failures that are returned.
    fn run_with_context(
        &self,
        source_root: &Path,
        source_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<()> {
        self.analyze_file(source_root, source_path, loader)
            .with_context(|| format!("Error analyzing file {}", source_path.display()))
    }

    fn analyze_file(
        &self,
        source_root: &Path,
        source_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<()> {
        let mut cancellation_flag: Arc<dyn CancellationFlag> = Arc::new(NoCancellation);
        if let Some(max_file_time) = self.max_file_time {
            cancellation_flag = CancelAfterDuration::new(max_file_time);
        }

        let source = std::fs::read_to_string(source_path)?;
        let lc = match loader.load_for_file(source_path, Some(&source), cancellation_flag.as_ref())
        {
            Ok(Some(sgl)) => sgl,
            Ok(None) => return Ok(()),
            Err(crate::loader::LoadError::Cancelled(_)) => {
                eprintln!(
                    "{}: {}",
                    source_path.display(),
                    "language loading timed out".yellow()
                );
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        if self.verbose {
            eprint!("{}: ", source_path.display());
        }

        let mut graph = StackGraph::new();
        let file = graph
            .add_file(&source_path.to_string_lossy())
            .map_err(|_| anyhow!("Duplicate file {}", source_path.display()))?;

        let relative_source_path = source_path.strip_prefix(source_root).unwrap();
        let result = if let Some(fa) = source_path
            .file_name()
            .and_then(|f| lc.special_files.get(&f.to_string_lossy()))
        {
            fa.build_stack_graph_into(
                &mut graph,
                file,
                &relative_source_path,
                &source,
                &mut std::iter::empty(),
                &HashMap::new(),
                cancellation_flag.as_ref(),
            )
        } else {
            let globals = Variables::new();
            lc.sgl.build_stack_graph_into(
                &mut graph,
                file,
                &source,
                &globals,
                cancellation_flag.as_ref(),
            )
        };
        match result {
            Err(LoadError::ParseErrors(parse_errors)) => {
                let parse_error = map_parse_errors(source_path, &parse_errors, &source, "");
                if !self.verbose {
                    eprint!("{}: ", source_path.display());
                }
                eprintln!("{}", "parsing failed".red());
                eprintln!("{}", parse_error);
                return Ok(());
            }
            Err(LoadError::Cancelled(_)) => {
                if !self.verbose {
                    eprint!("{}: ", source_path.display());
                }
                eprintln!("{}", "parsing timed out".yellow());
                return Ok(());
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        };

        let mut partials = PartialPaths::new();
        let mut db = Database::new();
        match partials.find_all_partial_paths_in_file(
            &graph,
            file,
            &cancellation_flag.as_ref(),
            |g, ps, p| {
                if p.is_complete_as_possible(g) {
                    db.add_partial_path(g, ps, p);
                }
            },
        ) {
            Ok(_) => {}
            Err(_) => {
                if !self.verbose {
                    eprint!("{}: ", source_path.display());
                }
                eprintln!("{}", "path computation timed out".yellow());
                return Ok(());
            }
        }

        if self.verbose {
            eprintln!("{}", "success".green());
        }
        Ok(())
    }
}
