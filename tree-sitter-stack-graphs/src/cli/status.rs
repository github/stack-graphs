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

use super::util::ConsoleFileLogger;
use super::util::FileLogger;

/// Status of files in the database
#[derive(Args)]
#[clap(group(
    ArgGroup::new("paths")
        .required(true)
        .args(&["source-paths", "all"]),
))]

pub struct StatusArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
    )]
    pub source_paths: Vec<PathBuf>,

    /// List all data from the database.
    #[clap(long, short = 'a')]
    pub all: bool,

    #[clap(long, short = 'v')]
    pub verbose: bool,
}

impl StatusArgs {
    pub fn run(self, db_path: &Path) -> anyhow::Result<()> {
        let mut db = SQLiteReader::open(&db_path)?;
        if self.all {
            let mut files = db.list_all()?;
            let mut entries = files.try_iter()?;
            self.status(&mut entries)?;
        } else {
            for source_path in &self.source_paths {
                let source_path = source_path.canonicalize()?;
                let mut files = db.list_file_or_directory(&source_path)?;
                let mut entries = files.try_iter()?;
                self.status(&mut entries)?;
            }
        }
        Ok(())
    }

    fn status(
        &self,
        entries: &mut impl Iterator<Item = stack_graphs::storage::Result<FileEntry>>,
    ) -> anyhow::Result<()> {
        for entry in entries {
            let entry = entry?;
            let mut logger = ConsoleFileLogger::new(&Path::new(&entry.path), true, self.verbose);
            match &entry.status {
                FileStatus::Missing => {
                    logger.skipped("missing", None);
                }
                FileStatus::Indexed => {
                    logger.success("indexed", None);
                }
                FileStatus::Error(error) => {
                    logger.failure("failed", Some(error));
                }
            }
        }
        Ok(())
    }
}
