// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::ArgGroup;
use clap::Args;
use clap::ValueHint;
use stack_graphs::storage::SQLiteWriter;
use std::path::Path;
use std::path::PathBuf;

#[derive(Args)]
#[clap(group(
    ArgGroup::new("paths")
        .required(true)
        .args(&["source_paths", "all", "delete"]),
))]
pub struct CleanArgs {
    /// Source file or directory paths for which to clean indexing data.
    #[clap(
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
    )]
    pub source_paths: Vec<PathBuf>,

    /// Remove all data from the database.
    #[clap(long, short = 'a')]
    pub all: bool,

    /// Delete the database file.
    #[clap(long)]
    pub delete: bool,

    #[clap(long, short = 'v')]
    pub verbose: bool,
}

impl CleanArgs {
    pub fn run(self, db_path: &Path) -> anyhow::Result<()> {
        if self.delete {
            self.delete(db_path)
        } else {
            self.clean(db_path)
        }
    }

    fn delete(&self, db_path: &Path) -> anyhow::Result<()> {
        if !db_path.exists() {
            return Ok(());
        }
        std::fs::remove_file(db_path)?;
        if self.verbose {
            println!("deleted database {}", db_path.display());
        }
        Ok(())
    }

    fn clean(&self, db_path: &Path) -> anyhow::Result<()> {
        let mut db = SQLiteWriter::open(&db_path)?;
        let count = if self.all {
            db.clean_all()?
        } else {
            let mut count = 0usize;
            for path in &self.source_paths {
                let path = path.canonicalize()?;
                count += db.clean_file_or_directory(&path)?;
            }
            count
        };
        if self.verbose {
            println!("removed data for {} files", count);
        }
        Ok(())
    }
}
