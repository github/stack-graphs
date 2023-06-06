// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::Args;
use clap::ValueHint;
use std::path::PathBuf;

#[derive(Args)]
pub struct DatabaseArgs {
    /// Path of the indexing database to use.
    #[clap(
        long,
        short = 'D',
        value_name = "DATABASE_PATH",
        value_hint = ValueHint::AnyPath,
    )]
    pub database: Option<PathBuf>,
}

impl DatabaseArgs {
    pub fn get_or(self, default_path: PathBuf) -> PathBuf {
        self.database.clone().unwrap_or_else(|| default_path)
    }
}

/// Returns the default database path in the current user's local data directory for the
/// given crate name. Distinct crate names will have distinct database paths.
pub fn default_user_database_path_for_crate(crate_name: &str) -> anyhow::Result<PathBuf> {
    match dirs::data_local_dir() {
        Some(dir) => Ok(dir.join(format!("{}.sqlite", crate_name))),
        None => Err(anyhow!(
            "unable to determine data local directory for database"
        )),
    }
}
