// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! This crate defines a reusable CI test runner.
//!
//! Use the test runner as follows:
//!
//! ``` no_run
//! use std::path::PathBuf;
//! use tree_sitter_stack_graphs::ci::Tester;
//! use tree_sitter_stack_graphs::NoCancellation;
//!
//! fn main() -> anyhow::Result<()> {
//!     let language_configurations = vec![/* add your language configurations here */];
//!     let test_paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test")];
//!     Tester::new(
//!         language_configurations,
//!         test_paths,
//!     )
//!     .run()
//! }
//! ```
//!
//! By default tests time out after 60 seconds. Set `Tester::max_test_time` to change the timeout.

use std::path::PathBuf;
use std::time::Duration;

use crate::cli::test::TestArgs;
use crate::loader::{LanguageConfiguration, Loader};

/// Run tests for the given languages. Test locations are reported relative to the current directory, which
/// results in better readable output when build tools only provides absolute test paths.
pub struct Tester {
    configurations: Vec<LanguageConfiguration>,
    test_paths: Vec<PathBuf>,
    pub max_test_time: Option<Duration>,
}

impl Tester {
    pub fn new(configurations: Vec<LanguageConfiguration>, test_paths: Vec<PathBuf>) -> Self {
        Self {
            configurations,
            test_paths,
            max_test_time: Some(Duration::from_secs(60)),
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        let test_paths = self
            .test_paths
            .into_iter()
            .map(|test_path| {
                std::env::current_dir()
                    .ok()
                    .and_then(|cwd| pathdiff::diff_paths(&test_path, &cwd))
                    .unwrap_or(test_path)
            })
            .collect::<Vec<_>>();
        for test_path in &test_paths {
            if !test_path.exists() {
                panic!("Test path {} does not exist", test_path.display());
            }
        }
        let loader = Loader::from_language_configurations(self.configurations, None)
            .expect("Expected loader");
        let mut args = TestArgs::new(test_paths);
        args.max_test_time = self.max_test_time;
        args.run(loader)
    }
}
