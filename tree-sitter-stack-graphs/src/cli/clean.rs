// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use clap::ValueHint;
use stack_graphs::storage::SQLiteWriter;
use std::path::PathBuf;

/// Clean database
#[derive(Args)]
pub struct CleanArgs {
    #[clap(
        long,
        short = 'D',
        value_name = "DATABASE_PATH",
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
    )]
    pub database: PathBuf,
}

impl CleanArgs {
    pub fn new(database: PathBuf) -> Self {
        Self { database }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let mut db = SQLiteWriter::open(&self.database)?;
        db.clean()?;
        Ok(())
    }
}
