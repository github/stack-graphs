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
use lsp_positions::PositionedSubstring;
use lsp_positions::SpanCalculator;
use stack_graphs::storage::SQLiteReader;
use stack_graphs::storage::StorageError;
use std::path::Path;
use std::path::PathBuf;

use crate::loader::FileReader;

use super::util::path_exists;
use super::util::sha1;
use super::util::FileStatusLogger;
use super::util::SourcePosition;

/// Analyze sources
#[derive(Args)]
pub struct QueryArgs {
    #[clap(
        long,
        short = 'D',
        value_name = "DATABASE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub database: PathBuf,

    #[clap(subcommand)]
    target: Target,
}

impl QueryArgs {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut db = SQLiteReader::open(&self.database)?;
        self.target.run(&mut db)
    }
}

#[derive(Subcommand)]
pub enum Target {
    Definition(Definition),
}

impl Target {
    pub fn run(&self, db: &mut SQLiteReader) -> anyhow::Result<()> {
        match self {
            Self::Definition(cmd) => cmd.run(db),
        }
    }
}

#[derive(Parser)]
pub struct Definition {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub source_path: PathBuf,

    /// Line number
    pub line: usize,

    /// Column number
    pub column: usize,
}

impl Definition {
    pub fn run(&self, db: &mut SQLiteReader) -> anyhow::Result<()> {
        let source_path = self.source_path.canonicalize()?;

        let reference = SourcePosition {
            path: source_path.clone(),
            line: self.line - 1,
            column: self.column - 1,
        };
        let mut file_reader = FileReader::new();

        let path = reference.to_string();
        let mut logger = FileStatusLogger::new(&Path::new(&path), true);
        logger.processing()?;

        let source = file_reader.get(&reference.path)?;
        let tag = sha1(source);

        if !db.file_exists(&source_path.to_string_lossy(), Some(&tag))? {
            logger.error("file not indexed")?;
            return Ok(());
        }

        let lines = PositionedSubstring::lines_iter(source);
        let mut span_calculator = SpanCalculator::new(source);

        db.load_graph_for_file(&reference.path.to_string_lossy())?;
        let (graph, _, _) = db.get();

        let reference = match reference.to_assertion_source(graph, lines, &mut span_calculator) {
            Ok(result) => result,
            Err(_) => {
                logger.error("invalid file or position")?;
                return Ok(());
            }
        };

        if reference.references_iter(graph).next().is_none() {
            logger.error("no references")?;
            return Ok(());
        }
        let starting_nodes = reference.references_iter(graph).collect::<Vec<_>>();

        let mut actual_paths = Vec::new();
        let mut reference_paths = Vec::new();
        match db.find_all_complete_partial_paths(
            starting_nodes,
            &stack_graphs::NoCancellation,
            |_g, _ps, p| {
                reference_paths.push(p.clone());
            },
        ) {
            Ok(_) => {}
            Err(StorageError::Cancelled(..)) => {
                logger.error("path finding timed out")?;
                return Ok(());
            }
            err => err?,
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
            return Ok(());
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

        Ok(())
    }
}
