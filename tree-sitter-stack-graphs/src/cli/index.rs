// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use clap::ValueHint;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::storage::FileStatus;
use stack_graphs::storage::SQLiteWriter;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tree_sitter_graph::Variables;

use crate::loader::FileReader;
use crate::loader::Loader;
use crate::BuildError;
use crate::CancelAfterDuration;
use crate::CancellationFlag;
use crate::NoCancellation;

use super::util::duration_from_seconds_str;
use super::util::iter_files_and_directories;
use super::util::path_exists;
use super::util::sha1;
use super::util::wait_for_input;
use super::util::ConsoleLogger;
use super::util::FileLogger;
use super::util::Logger;

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

    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Index files even if they are already present in the database.
    #[clap(long, short = 'f')]
    pub force: bool,

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
}

impl IndexArgs {
    pub fn new(source_paths: Vec<PathBuf>) -> Self {
        Self {
            source_paths,
            force: false,
            continue_from: None,
            verbose: false,
            hide_error_details: false,
            max_file_time: None,
            wait_at_start: false,
        }
    }

    pub fn run(self, db_path: &Path, mut loader: Loader) -> anyhow::Result<()> {
        if self.wait_at_start {
            wait_for_input()?;
        }
        let mut db = SQLiteWriter::open(&db_path)?;
        let logger = ConsoleLogger::new(self.verbose, !self.hide_error_details);
        let mut indexer = Indexer::new(&mut db, &mut loader, &logger);
        indexer.force = self.force;
        indexer.max_file_time = self.max_file_time;

        let source_paths = self
            .source_paths
            .into_iter()
            .map(|p| p.canonicalize())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        indexer.index_all(source_paths, self.continue_from, &NoCancellation)?;
        Ok(())
    }
}

pub struct Indexer<'a> {
    db: &'a mut SQLiteWriter,
    loader: &'a mut Loader,
    logger: &'a dyn Logger,
    /// Index files, even if they already exist in the database.
    pub force: bool,
    /// Maximum time per file.
    pub max_file_time: Option<Duration>,
}

impl<'a> Indexer<'a> {
    pub fn new(db: &'a mut SQLiteWriter, loader: &'a mut Loader, logger: &'a dyn Logger) -> Self {
        Self {
            db,
            loader,
            logger,
            force: false,
            max_file_time: None,
        }
    }

    pub fn index_all<P, IP, Q>(
        &mut self,
        source_paths: IP,
        mut continue_from: Option<Q>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        IP: IntoIterator<Item = P>,
        Q: AsRef<Path>,
    {
        for (source_root, source_path, strict) in iter_files_and_directories(source_paths) {
            cancellation_flag.check("indexing all files")?;
            self.index_file(
                &source_root,
                &source_path,
                strict,
                &mut continue_from,
                cancellation_flag,
            )?;
        }
        Ok(())
    }

    pub fn index(
        &mut self,
        source_root: &Path,
        source_path: &Path,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()> {
        self.index_file(
            &source_root,
            &source_path,
            true,
            &mut None::<&Path>,
            cancellation_flag,
        )?;
        Ok(())
    }

    /// Analyze file and add error context to any failures that are returned.
    fn index_file<P>(
        &mut self,
        source_root: &Path,
        source_path: &Path,
        missing_is_error: bool,
        continue_from: &mut Option<P>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut file_status = self.logger.file(source_path);
        match self.index_file_inner(
            source_root,
            source_path,
            missing_is_error,
            continue_from,
            cancellation_flag,
            file_status.as_mut(),
        ) {
            ok @ Ok(_) => ok,
            err @ Err(_) => {
                file_status.default_failure("error", Some(&format!("Error analyzing file {}. To continue analysis from this file later, add: --continue-from {}", source_path.display(), source_path.display())));
                err
            }
        }
    }

    fn index_file_inner<P>(
        &mut self,
        source_root: &Path,
        source_path: &Path,
        missing_is_error: bool,
        continue_from: &mut Option<P>,
        cancellation_flag: &dyn CancellationFlag,
        file_status: &mut dyn FileLogger,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        if self.should_skip(source_path, continue_from) {
            file_status.skipped("skipped", None);
            return Ok(());
        }

        let mut file_reader = FileReader::new();
        let lc = match self
            .loader
            .load_for_file(source_path, &mut file_reader, &NoCancellation)
        {
            Ok(Some(sgl)) => sgl,
            Ok(None) => {
                if missing_is_error {
                    file_status.failure("not supported", None);
                }
                return Ok(());
            }
            Err(crate::loader::LoadError::Cancelled(_)) => {
                file_status.warning("language loading timed out", None);
                return Ok(());
            }
            Err(e) => return Err(IndexError::LoadError(e)),
        };
        let source = file_reader.get(source_path)?;
        let tag = sha1(source);

        if !self.force {
            match self
                .db
                .status_for_file(&source_path.to_string_lossy(), Some(&tag))?
            {
                FileStatus::Missing => {}
                FileStatus::Indexed => {
                    file_status.skipped("cached index", None);
                    return Ok(());
                }
                FileStatus::Error(error) => {
                    file_status.skipped(&format!("cached error ({})", error), None);
                    return Ok(());
                }
            }
        }

        let file_cancellation_flag = CancelAfterDuration::from_option(self.max_file_time);
        let cancellation_flag = cancellation_flag | file_cancellation_flag.as_ref();

        file_status.processing();

        let mut graph = StackGraph::new();
        let file = graph
            .add_file(&source_path.to_string_lossy())
            .expect("file not present in emtpy graph");

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
                &cancellation_flag,
            )
        } else {
            let globals = Variables::new();
            lc.sgl
                .build_stack_graph_into(&mut graph, file, &source, &globals, &cancellation_flag)
        };
        match result {
            Err(BuildError::Cancelled(_)) => {
                file_status.warning("parsing timed out", None);
                self.db
                    .store_error_for_file(source_path, &tag, "parsing timed out")?;
                return Ok(());
            }
            Err(err @ BuildError::ParseErrors(_)) => {
                file_status.failure(
                    "parsing failed",
                    Some(&err.display_pretty(
                        source_path,
                        source,
                        lc.sgl.tsg_path(),
                        lc.sgl.tsg_source(),
                    )),
                );
                self.db.store_error_for_file(
                    source_path,
                    &tag,
                    &format!("parsing failed: {}", err),
                )?;
                return Ok(());
            }
            Err(err) => {
                file_status.failure(
                    "failed to build stack graph",
                    Some(&err.display_pretty(
                        source_path,
                        source,
                        lc.sgl.tsg_path(),
                        lc.sgl.tsg_source(),
                    )),
                );
                return Err(IndexError::StackGraph);
            }
            Ok(_) => true,
        };

        let mut partials = PartialPaths::new();
        let mut paths = Vec::new();
        match partials.find_minimal_partial_path_set_in_file(
            &graph,
            file,
            &(&cancellation_flag as &dyn CancellationFlag),
            |_g, _ps, p| {
                paths.push(p);
            },
        ) {
            Ok(_) => {}
            Err(_) => {
                file_status.warning("path computation timed out", None);
                self.db.store_error_for_file(
                    source_path,
                    &tag,
                    &format!("path computation timed out"),
                )?;
                return Ok(());
            }
        }

        self.db
            .store_result_for_file(&graph, file, &tag, &mut partials, &paths)?;

        file_status.success("success", None);

        Ok(())
    }

    /// Determines if a path should be skipped because we have not seen the
    /// continue_from mark yet. If the mark is seen, it is cleared, after which
    /// all paths are accepted.
    fn should_skip<P>(&self, path: &Path, continue_from: &mut Option<P>) -> bool
    where
        P: AsRef<Path>,
    {
        match continue_from {
            None => return false,
            Some(continue_from) if continue_from.as_ref() != path => return true,
            _ => {}
        };
        *continue_from = None;
        false
    }
}

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("cancelled at {0}")]
    Cancelled(&'static str),
    #[error("failed to load language")]
    LoadError(#[source] crate::loader::LoadError<'static>),
    #[error("failed to read file")]
    ReadError(#[from] std::io::Error),
    #[error("failed to build stank graph")]
    StackGraph,
    #[error(transparent)]
    StorageError(#[from] stack_graphs::storage::StorageError),
}

impl From<crate::CancellationError> for IndexError {
    fn from(value: crate::CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

type Result<T> = std::result::Result<T, IndexError>;
