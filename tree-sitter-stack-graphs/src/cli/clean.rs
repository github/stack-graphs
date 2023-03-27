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
use std::path::PathBuf;

use super::util::path_exists;
use super::util::provided_or_default_database_path;

/// Clean database
#[derive(Args)]
#[clap(group(
    ArgGroup::new("paths")
        .required(true)
        .args(&["source-paths", "all"]),
))]
pub struct CleanArgs {
    /// Source file or directory paths.
    #[clap(
        value_name = "SOURCE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
    )]
    pub source_paths: Vec<PathBuf>,

    #[clap(
        long,
        short = 'D',
        value_name = "DATABASE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists,
    )]
    pub database: Option<PathBuf>,

    #[clap(long, short = 'a')]
    pub all: bool,
}

impl CleanArgs {
    pub fn run(&self) -> anyhow::Result<()> {
        let db_path = provided_or_default_database_path(&self.database)?;
        let mut db = SQLiteWriter::open(&db_path)?;
        if self.all {
            db.clean(None::<&PathBuf>)?;
        } else {
            for path in &self.source_paths {
                let path = path.canonicalize()?;
                db.clean(Some(path))?;
            }
        }
        Ok(())
    }
}
