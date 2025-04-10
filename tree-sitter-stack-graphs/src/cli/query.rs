// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueHint;
use stack_graphs::stitching::ForwardPartialPathStitcher;
use stack_graphs::stitching::Stats as StitchingStats;
use stack_graphs::stitching::StitcherConfig;
use stack_graphs::storage::FileStatus;
use stack_graphs::storage::SQLiteReader;
use thiserror::Error;
use tree_sitter_graph::parse_error::Excerpt;

use crate::cli::util::print_database_stats;
use crate::cli::util::print_stitching_stats;
use crate::cli::util::reporter::ConsoleReporter;
use crate::cli::util::reporter::Reporter;
use crate::cli::util::sha1;
use crate::cli::util::wait_for_input;
use crate::cli::util::SourceIterator;
use crate::cli::util::SourcePosition;
use crate::cli::util::SourceSpan;
use crate::loader::FileReader;
use crate::CancellationFlag;
use crate::NoCancellation;

type Result<T> = anyhow::Result<T>;

#[derive(Args)]
pub struct QueryArgs {
    /// Wait for user input before starting analysis. Useful for profiling.
    #[clap(long)]
    pub wait_at_start: bool,

    #[clap(long)]
    pub stats: bool,

    #[clap(long)]
    pub silent: bool,

    #[clap(subcommand)]
    target: Target,
}

impl QueryArgs {
    pub fn run(self, db_path: &Path) -> Result<QueryResults> {
        if self.wait_at_start {
            wait_for_input()?;
        }
        let mut db = SQLiteReader::open(&db_path)?;
        let (results, stitching_stats) = self.target.run(&mut db, self.stats)?;
        if !self.silent && self.stats {
            println!();
            print_stitching_stats(stitching_stats);
            println!();
            print_database_stats(db.stats());
        }
        if !self.silent {
            print!("{}", results);
        }
        Ok(results)
    }
}

#[derive(Subcommand)]
pub enum Target {
    Definition(Definition),
    Span(Span),
}

impl Target {
    fn run(
        self,
        db: &mut SQLiteReader,
        collect_stats: bool,
    ) -> Result<(QueryResults, StitchingStats)> {
        let reporter = ConsoleReporter::details();
        let mut querier = Querier::new(db, &reporter);
        querier.set_collect_stats(collect_stats);
        let result = match self {
            Self::Definition(cmd) => cmd.run(&mut querier)?,
            Self::Span(cmd) => cmd.run(&mut querier)?,
        };
        Ok((result, querier.into_stats()))
    }
}

pub trait QueryRunner {
    fn run(self, querier: &mut Querier) -> Result<QueryResults>;
}

#[derive(Parser)]
pub struct Definition {
    /// Reference source positions, formatted as PATH:LINE:COLUMN.
    #[clap(
        value_name = "SOURCE_POSITION",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser,
    )]
    pub references: Vec<SourcePosition>,
}

#[derive(Parser)]
pub struct Span {
    /// Reference source spans, formatted as PATH:LINE_LO:LINE_HI.
    #[clap(
        value_name = "SOURCE_SPAN",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser,
    )]
    pub references: Vec<SourceSpan>,
}

impl QueryRunner for Definition {
    fn run(self, querier: &mut Querier) -> Result<QueryResults> {
        let cancellation_flag = NoCancellation;
        let mut results: QueryResults = Default::default();
        for mut reference in self.references {
            reference.canonicalize()?;
            let res = querier.definitions(reference.clone(), &cancellation_flag)?;
            results.extend(res);
        }
        Ok(results)
    }
}

impl QueryRunner for Span {
    fn run(self, querier: &mut Querier) -> Result<QueryResults> {
        let cancellation_flag = NoCancellation;
        let mut results: QueryResults = Default::default();
        for mut reference in self.references {
            reference.canonicalize()?;
            let res = querier.definitions(reference.clone(), &cancellation_flag)?;
            results.extend(res);
        }
        Ok(results)
    }
}

pub struct Querier<'a> {
    db: &'a mut SQLiteReader,
    reporter: &'a dyn Reporter,
    stats: Option<StitchingStats>,
}

impl<'a> Querier<'a> {
    pub fn new(db: &'a mut SQLiteReader, reporter: &'a dyn Reporter) -> Self {
        Self {
            db,
            reporter,
            stats: None,
        }
    }

    pub fn set_collect_stats(&mut self, collect_stats: bool) {
        if !collect_stats {
            self.stats = None;
        } else if self.stats.is_none() {
            self.stats = Some(StitchingStats::default());
        }
    }

    pub fn definitions<T>(
        &mut self,
        reference: T,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<QueryResults>
    where
        T: SourceIterator + std::fmt::Display,
    {
        let log_path = PathBuf::from(reference.to_string());

        let mut file_reader = FileReader::new();
        let tag = file_reader.get(reference.get_path()).ok().map(sha1);
        match self
            .db
            .status_for_file(&reference.get_path().to_string_lossy(), tag.as_ref())?
        {
            FileStatus::Indexed => {}
            _ => {
                self.reporter.started(&log_path);
                self.reporter.failed(&log_path, "file not indexed", None);
                return Ok(Default::default());
            }
        }

        self.reporter.started(&log_path);

        self.db
            .load_graph_for_file(&reference.get_path().to_string_lossy())?;
        let (graph, _, _) = self.db.get();

        let starting_nodes = reference.iter_references(graph).collect::<Vec<_>>();
        if starting_nodes.is_empty() {
            self.reporter
                .cancelled(&log_path, "no references at location", None);
            return Ok(Default::default());
        }

        let mut result = Vec::new();
        for (node, span) in starting_nodes {
            let reference_span = SourceSpan {
                path: reference.get_path().clone(),
                span,
            };

            let mut reference_paths = Vec::new();
            let stitcher_config = StitcherConfig::default()
                // always detect similar paths, we don't know the language configurations for the data in the database
                .with_detect_similar_paths(true)
                .with_collect_stats(self.stats.is_some());
            let ref_result = ForwardPartialPathStitcher::find_all_complete_partial_paths(
                self.db,
                std::iter::once(node),
                stitcher_config,
                &cancellation_flag,
                |_g, _ps, p| {
                    reference_paths.push(p.clone());
                },
            );
            match ref_result {
                Ok(ref_stats) => {
                    if let Some(stats) = &mut self.stats {
                        *stats += ref_stats
                    }
                }
                Err(err) => {
                    self.reporter.failed(&log_path, "query timed out", None);
                    return Err(err.into());
                }
            }

            let (graph, partials, _) = self.db.get();
            let mut actual_paths = Vec::new();
            for reference_path in &reference_paths {
                if let Err(err) = cancellation_flag.check("shadowing") {
                    self.reporter.failed(&log_path, "query timed out", None);
                    return Err(err.into());
                }
                if reference_paths
                    .iter()
                    .all(|other| !other.shadows(partials, reference_path))
                {
                    actual_paths.push(reference_path.clone());
                }
            }

            let valid_paths = actual_paths
                .into_iter()
                .filter_map(|path| {
                    let source_info = graph.source_info(path.end_node);
                    if source_info.is_none() {
                        return None;
                    }
                    if graph[path.end_node].id().file().is_none() {
                        return None;
                    };
                    Some(path)
                })
                .collect::<Vec<_>>();

            let definitions = valid_paths
                .into_iter()
                .filter_map(|path| {
                    let source_info = graph.source_info(path.start_node);
                    if source_info.is_none() {
                        return None;
                    }
                    let span = source_info.unwrap().span.clone();
                    let fname = graph[path.end_node].id().file().unwrap();
                    let path = PathBuf::from(graph[fname].name());
                    Some(SourceSpan { span, path })
                })
                .collect::<Vec<_>>();

            if definitions.len() == 0 {
                continue;
            }

            result.push(QueryResult {
                source: reference_span,
                targets: definitions,
            });
        }

        let count: usize = result.iter().map(|r| r.targets.len()).sum();
        self.reporter.succeeded(
            &log_path,
            &format!(
                "found {} definitions for {} references",
                count,
                result.len()
            ),
            None,
        );

        Ok(QueryResults { results: result })
    }

    pub fn into_stats(self) -> StitchingStats {
        self.stats.unwrap_or_default()
    }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("cancelled at {0}")]
    Cancelled(&'static str),
    #[error("failed to read file")]
    ReadError(#[from] std::io::Error),
    #[error(transparent)]
    StorageError(#[from] stack_graphs::storage::StorageError),
}

impl From<stack_graphs::CancellationError> for QueryError {
    fn from(value: stack_graphs::CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

impl From<crate::CancellationError> for QueryError {
    fn from(value: crate::CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

pub struct QueryResult {
    pub source: SourceSpan,
    pub targets: Vec<SourceSpan>,
}

pub struct QueryResults {
    pub results: Vec<QueryResult>,
}

impl Default for QueryResults {
    fn default() -> Self {
        Self {
            results: Vec::new(),
        }
    }
}

impl QueryResults {
    pub fn extend(&mut self, other: QueryResults) {
        self.results.extend(other.results);
    }
}

impl std::fmt::Display for QueryResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut file_reader = FileReader::new();
        let numbered = self.results.len() > 1;
        let indent = if numbered { 6 } else { 0 };
        let results = &self.results;
        if numbered {
            write!(f, "found {} references at position", results.len())?;
        }
        for (
            idx,
            QueryResult {
                source: reference,
                targets: definitions,
            },
        ) in results.into_iter().enumerate()
        {
            if numbered {
                write!(f, "{:4}: queried reference", idx)?;
            } else {
                write!(f, "queried reference")?;
            }
            write!(
                f,
                "{}",
                Excerpt::from_source(
                    &reference.path,
                    file_reader.get(&reference.path).unwrap_or_default(),
                    reference.first_line(),
                    reference.first_line_column_range(),
                    indent
                )
            )?;
            match definitions.len() {
                0 => write!(f, "{}has no definitions", " ".repeat(indent))?,
                1 => write!(f, "{}has definition", " ".repeat(indent))?,
                n => write!(f, "{}has {} definitions", " ".repeat(indent), n)?,
            }
            for definition in definitions.into_iter() {
                write!(
                    f,
                    "{}",
                    Excerpt::from_source(
                        &definition.path,
                        file_reader.get(&definition.path).unwrap_or_default(),
                        definition.first_line(),
                        definition.first_line_column_range(),
                        indent
                    )
                )?;
            }
        }
        Ok(())
    }
}
