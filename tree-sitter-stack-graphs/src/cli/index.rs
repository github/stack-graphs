// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use clap::ValueHint;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::ForwardPartialPathStitcher;
use stack_graphs::stitching::Stats as StitchingStats;
use stack_graphs::stitching::StitcherConfig;
use stack_graphs::storage::FileStatus;
use stack_graphs::storage::SQLiteWriter;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tree_sitter_graph::Variables;

use crate::cli::util::duration_from_seconds_str;
use crate::cli::util::iter_files_and_directories;
use crate::cli::util::print_stitching_stats;
use crate::cli::util::reporter::ConsoleReporter;
use crate::cli::util::reporter::Level;
use crate::cli::util::reporter::Reporter;
use crate::cli::util::sha1;
use crate::cli::util::wait_for_input;
use crate::cli::util::BuildErrorWithSource;
use crate::cli::util::CLIFileReporter;
use crate::cli::util::ExistingPathBufValueParser;
use crate::loader::FileLanguageConfigurations;
use crate::loader::FileReader;
use crate::loader::Loader;
use crate::BuildError;
use crate::CancelAfterDuration;
use crate::CancellationFlag;
use crate::NoCancellation;

#[derive(Args)]
pub struct IndexArgs {
    /// Source file or directory paths to index.
    #[clap(
        value_name = "SOURCE_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser = ExistingPathBufValueParser,
    )]
    pub source_paths: Vec<PathBuf>,

    /// Continue indexing from the given file.
    #[clap(
        long,
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
        value_parser = ExistingPathBufValueParser,
    )]
    pub continue_from: Option<PathBuf>,

    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Index files even if they are already present in the database.
    #[clap(long, short = 'f')]
    pub force: bool,

    /// Hide details of indexing errors on files.
    #[clap(long)]
    pub hide_error_details: bool,

    /// Maximum runtime per file in seconds.
    #[clap(
        long,
        value_name = "SECONDS",
        value_parser = duration_from_seconds_str,
    )]
    pub max_file_time: Option<Duration>,

    #[clap(long)]
    pub stats: bool,

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
            stats: false,
        }
    }

    pub fn run(self, db_path: &Path, mut loader: Loader) -> anyhow::Result<()> {
        if self.wait_at_start {
            wait_for_input()?;
        }
        let mut db = SQLiteWriter::open(&db_path)?;
        let reporter = self.get_reporter();
        let mut indexer = Indexer::new(&mut db, &mut loader, &reporter);
        indexer.force = self.force;
        indexer.max_file_time = self.max_file_time;

        let source_paths = self
            .source_paths
            .into_iter()
            .map(|p| p.canonicalize())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        indexer.index_all(source_paths, self.continue_from, &NoCancellation)?;

        if self.stats {
            println!();
            print_stitching_stats(indexer.into_stats());
        }
        Ok(())
    }

    fn get_reporter(&self) -> ConsoleReporter {
        return ConsoleReporter {
            skipped_level: if self.verbose {
                Level::Summary
            } else {
                Level::None
            },
            succeeded_level: if self.verbose {
                Level::Summary
            } else {
                Level::None
            },
            failed_level: if self.hide_error_details {
                Level::Summary
            } else {
                Level::Details
            },
            canceled_level: if self.hide_error_details {
                Level::Summary
            } else {
                Level::Details
            },
        };
    }
}

pub struct Indexer<'a> {
    db: &'a mut SQLiteWriter,
    loader: &'a mut Loader,
    reporter: &'a dyn Reporter,
    stats: StitchingStats,
    /// Index files, even if they already exist in the database.
    pub force: bool,
    /// Maximum time per file.
    pub max_file_time: Option<Duration>,
}

impl<'a> Indexer<'a> {
    pub fn new(
        db: &'a mut SQLiteWriter,
        loader: &'a mut Loader,
        reporter: &'a dyn Reporter,
    ) -> Self {
        Self {
            db,
            loader,
            reporter,
            force: false,
            max_file_time: None,
            stats: StitchingStats::default(),
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
            let mut file_status = CLIFileReporter::new(self.reporter, &source_path);
            cancellation_flag.check("indexing all files")?;
            self.index_file(
                &source_root,
                &source_path,
                strict,
                &mut continue_from,
                cancellation_flag,
                &mut file_status,
            )?;
            file_status.assert_reported();
        }
        Ok(())
    }

    pub fn index(
        &mut self,
        source_root: &Path,
        source_path: &Path,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()> {
        let mut file_status = CLIFileReporter::new(self.reporter, source_path);
        self.index_file(
            &source_root,
            &source_path,
            true,
            &mut None::<&Path>,
            cancellation_flag,
            &mut file_status,
        )?;
        file_status.assert_reported();
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
        file_status: &mut CLIFileReporter,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        match self.index_file_inner(
            source_root,
            source_path,
            missing_is_error,
            continue_from,
            cancellation_flag,
            file_status,
        ) {
            ok @ Ok(_) => {
                file_status.assert_reported();
                ok
            }
            err @ Err(_) => {
                file_status.failure_if_processing("error", Some(&format!("Error analyzing file {}. To continue analysis from this file later, add: --continue-from {}", source_path.display(), source_path.display())));
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
        file_status: &mut CLIFileReporter<'_>,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        if self.should_skip(source_path, continue_from) {
            file_status.skipped("skipped", None);
            return Ok(());
        }

        let mut file_reader = FileReader::new();
        let lcs = match self
            .loader
            .load_for_file(source_path, &mut file_reader, &NoCancellation)
        {
            Ok(lcs) if !lcs.has_some() => {
                if missing_is_error {
                    file_status.failure("not supported", None);
                }
                return Ok(());
            }
            Ok(lcs) => lcs,
            Err(crate::loader::LoadError::Cancelled(_)) => {
                file_status.warning("language loading timed out", None);
                return Ok(());
            }
            Err(e) => return Err(IndexError::LoadError(e)),
        };
        let stitcher_config =
            StitcherConfig::default().with_detect_similar_paths(!lcs.no_similar_paths_in_file());

        let source = file_reader.get(source_path)?;
        let tag = sha1(source);

        let success_status = match self
            .db
            .status_for_file(&source_path.to_string_lossy(), Some(&tag))?
        {
            FileStatus::Missing => "indexed",
            FileStatus::Indexed => {
                if self.force {
                    "reindexed"
                } else {
                    file_status.skipped("cached index", None);
                    return Ok(());
                }
            }
            FileStatus::Error(error) => {
                if self.force {
                    "reindexed"
                } else {
                    file_status.skipped(&format!("cached error ({})", error), None);
                    return Ok(());
                }
            }
        };

        let file_cancellation_flag = CancelAfterDuration::from_option(self.max_file_time);
        let cancellation_flag = cancellation_flag | file_cancellation_flag.as_ref();

        file_status.processing();

        let mut graph = StackGraph::new();
        let file = graph
            .add_file(&source_path.to_string_lossy())
            .expect("file not present in empty graph");

        let result = Self::build_stack_graph(
            &mut graph,
            file,
            source_root,
            source_path,
            &source,
            lcs,
            &cancellation_flag,
        );
        if let Err(err) = result {
            match err.inner {
                BuildError::Cancelled(_) => {
                    file_status.warning("parsing timed out", None);
                    self.db
                        .store_error_for_file(source_path, &tag, "parsing timed out")?;
                    return Ok(());
                }
                BuildError::ParseErrors { .. } => {
                    file_status.failure("parsing failed", Some(&err.display_pretty()));
                    self.db.store_error_for_file(
                        source_path,
                        &tag,
                        &format!("parsing failed: {}", err.inner),
                    )?;
                    return Ok(());
                }
                _ => {
                    file_status.failure("failed to build stack graph", Some(&err.display_pretty()));
                    return Err(IndexError::StackGraph);
                }
            }
        };

        let mut partials = PartialPaths::new();
        let mut paths = Vec::new();
        match ForwardPartialPathStitcher::find_minimal_partial_path_set_in_file(
            &graph,
            &mut partials,
            file,
            stitcher_config,
            &(&cancellation_flag as &dyn CancellationFlag),
            |_g, _ps, p| {
                paths.push(p.clone());
            },
        ) {
            Ok(stats) => {
                self.stats += &stats;
            }
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

        file_status.success(success_status, None);

        Ok(())
    }

    fn build_stack_graph<'b>(
        graph: &mut StackGraph,
        file: Handle<File>,
        source_root: &Path,
        source_path: &Path,
        source: &'b str,
        lcs: FileLanguageConfigurations<'b>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> std::result::Result<(), BuildErrorWithSource<'b>> {
        let relative_source_path = source_path.strip_prefix(source_root).unwrap();
        if let Some(lc) = lcs.primary {
            let globals = Variables::new();
            lc.sgl
                .build_stack_graph_into(graph, file, source, &globals, cancellation_flag)
                .map_err(|inner| BuildErrorWithSource {
                    inner,
                    source_path: source_path.to_path_buf(),
                    source_str: source,
                    tsg_path: lc.sgl.tsg_path().to_path_buf(),
                    tsg_str: &lc.sgl.tsg_source(),
                })?;
        }
        for (_, fa) in lcs.secondary {
            fa.build_stack_graph_into(
                graph,
                file,
                &relative_source_path,
                &source,
                &mut std::iter::empty(),
                &HashMap::new(),
                cancellation_flag,
            )
            .map_err(|inner| BuildErrorWithSource {
                inner,
                source_path: source_path.to_path_buf(),
                source_str: &source,
                tsg_path: PathBuf::new(),
                tsg_str: "",
            })?;
        }
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

    pub fn stats(&self) -> &StitchingStats {
        &self.stats
    }

    pub fn into_stats(self) -> StitchingStats {
        self.stats
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
