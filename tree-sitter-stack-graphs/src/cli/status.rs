// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::ArgGroup;
use clap::Args;
use clap::ValueHint;
use stack_graphs::storage::FileEntry;
use stack_graphs::storage::FileStatus;
use stack_graphs::storage::SQLiteReader;
use std::path::Path;
use std::path::PathBuf;

use super::util::reporter::ConsoleReporter;
use super::util::reporter::Reporter;

#[derive(Args)]
#[clap(group(
    ArgGroup::new("paths")
        .required(true)
        .args(&["source_paths", "all"]),
))]
pub struct StatusArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
    )]
    pub source_paths: Vec<PathBuf>,

    /// Show status of all indexed source paths.
    #[clap(long, short = 'a')]
    pub all: bool,

    #[clap(long, short = 'v')]
    pub verbose: bool,
}

impl StatusArgs {
    pub fn run(self, db_path: &Path) -> anyhow::Result<()> {
        let reporter = self.get_reporter();
        let mut db = SQLiteReader::open(&db_path)?;
        if self.all {
            let mut files = db.list_all()?;
            let mut entries = files.try_iter()?;
            self.status(&mut entries, &reporter)?;
        } else {
            for source_path in &self.source_paths {
                let source_path = source_path.canonicalize()?;
                let mut files = db.list_file_or_directory(&source_path)?;
                let mut entries = files.try_iter()?;
                self.status(&mut entries, &reporter)?;
            }
        }
        Ok(())
    }

    fn get_reporter(&self) -> ConsoleReporter {
        if self.verbose {
            ConsoleReporter::details()
        } else {
            ConsoleReporter::summary()
        }
    }

    fn status(
        &self,
        entries: &mut impl Iterator<Item = stack_graphs::storage::Result<FileEntry>>,
        reporter: &dyn Reporter,
    ) -> anyhow::Result<()> {
        for entry in entries {
            let entry = entry?;
            reporter.started(&entry.path);
            match &entry.status {
                FileStatus::Missing => {
                    reporter.cancelled(&entry.path, "missing", None);
                }
                FileStatus::Indexed => {
                    reporter.succeeded(&entry.path, "indexed", None);
                }
                FileStatus::Error(error) => {
                    reporter.failed(&entry.path, "failed", Some(error));
                }
            }
        }
        Ok(())
    }
}
