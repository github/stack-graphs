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
use lsp_positions::PositionedSubstring;
use lsp_positions::SpanCalculator;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::storage::SQLiteReader;
use stack_graphs::storage::SQLiteWriter;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tree_sitter_graph::Variables;
use walkdir::WalkDir;

use crate::cli::util::duration_from_seconds_str;
use crate::cli::util::map_parse_errors;
use crate::cli::util::path_exists;
use crate::loader::FileReader;
use crate::loader::Loader;
use crate::CancelAfterDuration;
use crate::CancellationFlag;
use crate::LoadError;
use crate::NoCancellation;

use super::util::FileStatusLogger;
use super::util::SourcePosition;

/// Analyze sources
#[derive(Args)]
pub struct IndexArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub source_paths: Vec<PathBuf>,

    /// Continue analysis from the given file
    #[clap(
        long,
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub continue_from: Option<PathBuf>,

    #[clap(
        long,
        short = 'D',
        value_name = "DATABASE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
    )]
    pub database: PathBuf,

    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Hide failure error details.
    #[clap(long)]
    pub hide_error_details: bool,

    /// Maximum runtime per file in seconds.
    #[clap(
        long,
        value_name = "SECONDS",
        parse(try_from_str = duration_from_seconds_str),
    )]
    pub max_file_time: Option<Duration>,

    /// Wait for user input before starting analysis. Useful for profiling.
    #[clap(long)]
    pub wait_at_start: bool,

    /// Query definitions for the references at the given position. The position
    /// must be given in the format SOURCE_PATH:LINE:COLUMN.
    #[clap(
        long = "definition",
        short = 'd',
        value_name = "POSITION",
        parse(try_from_str)
    )]
    pub query_definitions: Vec<SourcePosition>,
}

impl IndexArgs {
    pub fn new(database: PathBuf, source_paths: Vec<PathBuf>) -> Self {
        Self {
            source_paths,
            database,
            continue_from: None,
            verbose: false,
            hide_error_details: false,
            max_file_time: None,
            wait_at_start: false,
            query_definitions: Vec::new(),
        }
    }

    pub fn run(&self, loader: &mut Loader) -> anyhow::Result<()> {
        if self.wait_at_start {
            self.wait_for_input()?;
        }
        let mut seen_mark = false;
        let mut db = SQLiteWriter::open(&self.database)?;
        for source_path in &self.source_paths {
            if source_path.is_dir() {
                let source_root = source_path;
                for source_entry in WalkDir::new(source_root)
                    .follow_links(true)
                    .sort_by_file_name()
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let source_path = source_entry.path();
                    self.analyze_file_with_context(
                        source_root,
                        source_path,
                        loader,
                        &mut seen_mark,
                        &mut db,
                    )?;
                }
            } else {
                let source_root = source_path.parent().unwrap();
                if self.should_skip(source_path, &mut seen_mark) {
                    continue;
                }
                self.analyze_file_with_context(
                    source_root,
                    source_path,
                    loader,
                    &mut seen_mark,
                    &mut db,
                )?;
            }
        }

        self.run_queries(&mut db.into_reader())?;

        Ok(())
    }

    fn wait_for_input(&self) -> anyhow::Result<()> {
        print!("<press ENTER to continue>");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        Ok(())
    }

    /// Analyze file and add error context to any failures that are returned.
    fn analyze_file_with_context(
        &self,
        source_root: &Path,
        source_path: &Path,
        loader: &mut Loader,
        seen_mark: &mut bool,
        db: &mut SQLiteWriter,
    ) -> anyhow::Result<()> {
        self.analyze_file(source_root, source_path, loader, seen_mark, db)
            .with_context(|| format!("Error analyzing file {}. To continue analysis from this file later, add: --continue-from {}", source_path.display(), source_path.display()))
    }

    fn analyze_file(
        &self,
        source_root: &Path,
        source_path: &Path,
        loader: &mut Loader,
        seen_mark: &mut bool,
        db: &mut SQLiteWriter,
    ) -> anyhow::Result<()> {
        let mut file_status = FileStatusLogger::new(source_path, self.verbose);

        if self.should_skip(source_path, seen_mark) {
            file_status.info("skipped")?;
            return Ok(());
        }

        if db.file_exists(&source_path.to_string_lossy())? {
            file_status.info("cached")?;
            return Ok(());
        }

        let mut file_reader = FileReader::new();
        let lc = match loader.load_for_file(source_path, &mut file_reader, &NoCancellation) {
            Ok(Some(sgl)) => sgl,
            Ok(None) => return Ok(()),
            Err(crate::loader::LoadError::Cancelled(_)) => {
                file_status.warn("language loading timed out")?;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        let source = file_reader.get(source_path)?;

        let mut cancellation_flag: Arc<dyn CancellationFlag> = Arc::new(NoCancellation);
        if let Some(max_file_time) = self.max_file_time {
            cancellation_flag = CancelAfterDuration::new(max_file_time);
        }

        file_status.processing()?;

        let mut graph = StackGraph::new();
        let file = match graph.add_file(&source_path.to_string_lossy()) {
            Ok(file) => file,
            Err(_) => return Err(anyhow!("Duplicate file {}", source_path.display())),
        };

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
                file_status.error("parsing failed")?;
                if !self.hide_error_details {
                    println!("{}", parse_error);
                }
                return Ok(());
            }
            Err(LoadError::Cancelled(_)) => {
                file_status.warn("parsing timed out")?;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        };
        db.add_graph_for_file(&graph, file)?;

        let mut partials = PartialPaths::new();
        match partials.find_minimal_partial_path_set_in_file(
            &graph,
            file,
            &cancellation_flag.as_ref(),
            |g, ps, p| {
                db.add_partial_path_for_file(g, ps, &p, file).expect("TODO");
            },
        ) {
            Ok(_) => {}
            Err(_) => {
                file_status.warn("path computation timed out")?;
                return Ok(());
            }
        }

        file_status.ok("success")?;

        Ok(())
    }

    /// Determines if a path should be skipped because we have not seen the
    /// continue_from mark yet. The `seen_mark` parameter is necessary to keep
    /// track of the mark between the calls in one run.
    fn should_skip(&self, path: &Path, seen_mark: &mut bool) -> bool {
        if *seen_mark {
            return false; // return early and skip match
        }
        if let Some(mark) = &self.continue_from {
            if path == mark {
                *seen_mark = true; // this is the mark, we have seen it
            }
        } else {
            *seen_mark = true; // early return from now on
        }
        return !*seen_mark; // skip if we haven't seen the mark yet
    }

    fn run_queries(&self, db: &mut SQLiteReader) -> anyhow::Result<()> {
        let mut file_reader = FileReader::new();
        for reference in &self.query_definitions {
            let path = reference.to_string();
            let mut logger = FileStatusLogger::new(&Path::new(&path), true);
            logger.processing()?;

            let source = file_reader.get(&reference.path)?;
            let lines = PositionedSubstring::lines_iter(source);
            let mut span_calculator = SpanCalculator::new(source);

            db.load_graph_for_file(&reference.path.to_string_lossy())?;
            let (graph, _, _) = db.get();

            let reference = match reference.to_assertion_source(graph, lines, &mut span_calculator)
            {
                Ok(result) => result,
                Err(_) => {
                    logger.error("invalid file or position")?;
                    continue;
                }
            };

            if reference.references_iter(graph).next().is_none() {
                logger.error("no references")?;
                continue;
            }
            let starting_nodes = reference.references_iter(graph).collect::<Vec<_>>();

            let mut actual_paths = Vec::new();
            let mut reference_paths = Vec::new();
            if let Err(_) = db.find_all_complete_partial_paths(
                starting_nodes,
                &stack_graphs::NoCancellation,
                |_g, _ps, p| {
                    reference_paths.push(p.clone());
                },
            ) {
                logger.error("path finding timed out")?;
                continue;
            };

            let (graph, partials, _) = db.get();
            for reference_path in &reference_paths {
                if reference_paths
                    .iter()
                    .all(|other| !other.shadows(partials, reference_path))
                {
                    actual_paths.push(reference_path.clone());
                }
            }

            if actual_paths.is_empty() {
                logger.warn("no definitions")?;
                continue;
            }

            logger.ok("found definitions:")?;
            for (idx, path) in actual_paths.into_iter().enumerate() {
                let file = match graph[path.end_node].id().file() {
                    Some(f) => graph[f].to_string(),
                    None => "?".to_string(),
                };
                let line_col = match graph.source_info(path.end_node) {
                    Some(p) => format!(
                        "{}:{}",
                        p.span.start.line + 1,
                        p.span.start.column.grapheme_offset + 1
                    ),
                    None => "?:?".to_string(),
                };
                println!("  {:2}: {}:{}", idx, file, line_col);
            }
        }

        Ok(())
    }
}
