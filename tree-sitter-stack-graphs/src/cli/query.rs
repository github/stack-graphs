// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueHint;
use stack_graphs::storage::FileStatus;
use stack_graphs::storage::SQLiteReader;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tree_sitter_graph::parse_error::Excerpt;

use crate::loader::FileReader;
use crate::CancellationFlag;
use crate::NoCancellation;

use super::util::sha1;
use super::util::wait_for_input;
use super::util::ConsoleLogger;
use super::util::Logger;
use super::util::SourcePosition;
use super::util::SourceSpan;

/// Analyze sources
#[derive(Args)]
pub struct QueryArgs {
    /// Wait for user input before starting analysis. Useful for profiling.
    #[clap(long)]
    pub wait_at_start: bool,

    #[clap(subcommand)]
    target: Target,
}

impl QueryArgs {
    pub fn run(self, db_path: &Path) -> anyhow::Result<()> {
        if self.wait_at_start {
            wait_for_input()?;
        }
        let mut db = SQLiteReader::open(&db_path)?;
        self.target.run(&mut db)
    }
}

#[derive(Subcommand)]
pub enum Target {
    Definition(Definition),
}

impl Target {
    pub fn run(self, db: &mut SQLiteReader) -> anyhow::Result<()> {
        let logger = ConsoleLogger::new(true, true);
        let mut querier = Querier::new(db, &logger);
        match self {
            Self::Definition(cmd) => cmd.run(&mut querier),
        }
    }
}

#[derive(Parser)]
pub struct Definition {
    /// Reference source positions, formatted as PATH:LINE:COLUMN.
    #[clap(
        value_name = "SOURCE_POSITION",
        required = true,
        value_hint = ValueHint::AnyPath,
        parse(try_from_str),
    )]
    pub references: Vec<SourcePosition>,
}

impl Definition {
    pub fn run(self, querier: &mut Querier) -> anyhow::Result<()> {
        let cancellation_flag = NoCancellation;
        let mut file_reader = FileReader::new();
        for mut reference in self.references {
            reference.canonicalize()?;

            let results = querier.definitions(reference.clone(), &cancellation_flag)?;
            let numbered = results.len() > 1;
            let indent = if numbered { 6 } else { 0 };
            if numbered {
                println!("found {} references at position", results.len());
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
                    println!("{:4}: queried reference", idx);
                } else {
                    println!("queried reference");
                }
                println!(
                    "{}",
                    Excerpt::from_source(
                        &reference.path,
                        file_reader.get(&reference.path).unwrap_or_default(),
                        reference.first_line(),
                        reference.first_line_column_range(),
                        indent
                    )
                );
                match definitions.len() {
                    0 => println!("{}has no definitions", " ".repeat(indent)),
                    1 => println!("{}has definition", " ".repeat(indent)),
                    n => println!("{}has {} definitions", " ".repeat(indent), n),
                }
                for definition in definitions.into_iter() {
                    println!(
                        "{}",
                        Excerpt::from_source(
                            &definition.path,
                            file_reader.get(&definition.path).unwrap_or_default(),
                            definition.first_line(),
                            definition.first_line_column_range(),
                            indent
                        )
                    );
                }
            }
        }
        Ok(())
    }
}

pub struct Querier<'a> {
    db: &'a mut SQLiteReader,
    logger: &'a dyn Logger,
}

impl<'a> Querier<'a> {
    pub fn new(db: &'a mut SQLiteReader, logger: &'a dyn Logger) -> Self {
        Self { db, logger }
    }

    pub fn definitions(
        &mut self,
        reference: SourcePosition,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<Vec<QueryResult>> {
        let log_path = PathBuf::from(reference.to_string());
        let mut logger = self.logger.file(&log_path);

        let mut file_reader = FileReader::new();
        let tag = file_reader.get(&reference.path).ok().map(sha1);
        match self
            .db
            .status_for_file(&reference.path.to_string_lossy(), tag.as_ref())?
        {
            FileStatus::Indexed => {}
            _ => {
                logger.failure("file not indexed", None);
                return Ok(Vec::default());
            }
        }

        logger.processing();

        self.db
            .load_graph_for_file(&reference.path.to_string_lossy())?;
        let (graph, _, _) = self.db.get();

        let starting_nodes = reference.iter_references(graph).collect::<Vec<_>>();
        if starting_nodes.is_empty() {
            logger.warning("no references at location", None);
            return Ok(Vec::default());
        }

        let mut result = Vec::new();
        for (node, span) in starting_nodes {
            let reference_span = SourceSpan {
                path: reference.path.clone(),
                span,
            };

            let mut reference_paths = Vec::new();
            if let Err(err) = self.db.find_all_complete_partial_paths(
                std::iter::once(node),
                &cancellation_flag,
                |_g, _ps, p| {
                    reference_paths.push(p.clone());
                },
            ) {
                logger.failure("query timed out", None);
                return Err(err.into());
            }

            let (graph, partials, _) = self.db.get();
            let mut actual_paths = Vec::new();
            for reference_path in &reference_paths {
                if let Err(err) = cancellation_flag.check("shadowing") {
                    logger.failure("query timed out", None);
                    return Err(err.into());
                }
                if reference_paths
                    .iter()
                    .all(|other| !other.shadows(partials, reference_path))
                {
                    actual_paths.push(reference_path.clone());
                }
            }

            let definitions = actual_paths
                .into_iter()
                .filter_map(|path| {
                    let span = match graph.source_info(path.end_node) {
                        Some(p) => p.span.clone(),
                        None => return None,
                    };
                    let path = match graph[path.end_node].id().file() {
                        Some(f) => PathBuf::from(graph[f].name()),
                        None => return None,
                    };
                    Some(SourceSpan { path, span })
                })
                .collect::<Vec<_>>();

            result.push(QueryResult {
                source: reference_span,
                targets: definitions,
            });
        }

        let count: usize = result.iter().map(|r| r.targets.len()).sum();
        logger.success(
            &format!(
                "found {} definitions for {} references",
                count,
                result.len()
            ),
            None,
        );

        Ok(result)
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

impl From<crate::CancellationError> for QueryError {
    fn from(value: crate::CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

pub struct QueryResult {
    pub source: SourceSpan,
    pub targets: Vec<SourceSpan>,
}

type Result<T> = std::result::Result<T, QueryError>;
