// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use colored::ColoredString;
use colored::Colorize;
use std::io::Write;
use std::path::Path;

/// Trait that supports reporting file processing status.
///
/// For each file, either
///  - [`skipped`] is called once, or
///  - [`started`] and one of [`succeeded`], [`failed`], or [`canceled`] are called.
///
/// Guidance for severity of these statuses:
///  - Failed files should be reported as errors.
///  - Canceled files can be reported as warnings.
///  - Succeeded and skipped files can be reported as info.
pub trait Reporter {
    /// File was skipped.
    fn skipped(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>);

    /// File processing started.
    fn started(&self, path: &Path);

    /// File was processed and succeeded.
    fn succeeded(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>);

    /// File was processed and failed.
    fn failed(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>);

    /// File could not be processed and was canceled.
    fn cancelled(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>);
}

/// An enum describing the level of detail that should be reported.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Level {
    None,
    Summary,
    Details,
}

/// A console reporter that outputs the path when processing starts, and appends
/// the status once finished.
#[derive(Clone, Copy, Debug)]
pub struct ConsoleReporter {
    pub skipped_level: Level,
    pub succeeded_level: Level,
    pub failed_level: Level,
    pub canceled_level: Level,
}

impl ConsoleReporter {
    pub fn none() -> Self {
        Self {
            skipped_level: Level::None,
            succeeded_level: Level::None,
            failed_level: Level::None,
            canceled_level: Level::None,
        }
    }

    pub fn summary() -> Self {
        Self {
            skipped_level: Level::Summary,
            succeeded_level: Level::Summary,
            failed_level: Level::Summary,
            canceled_level: Level::Summary,
        }
    }

    pub fn details() -> Self {
        Self {
            skipped_level: Level::Details,
            succeeded_level: Level::Details,
            failed_level: Level::Details,
            canceled_level: Level::Details,
        }
    }
    fn all_results_are_reported(&self) -> bool {
        *[self.succeeded_level, self.failed_level, self.canceled_level]
            .iter()
            .min()
            .unwrap()
            > Level::None
    }

    fn print_path(&self, path: &Path) {
        print!("{}: ", path.display());
        self.flush();
    }

    fn print_result(
        &self,
        print_details: bool,
        summary: ColoredString,
        details: Option<&dyn std::fmt::Display>,
    ) {
        println!("{}", summary);
        if !print_details {
            return;
        }
        if let Some(details) = details {
            println!("{}", details);
        }
    }

    fn flush(&self) {
        std::io::stdout().flush().expect("flush should succeed");
    }
}

impl Reporter for ConsoleReporter {
    fn skipped(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>) {
        if self.skipped_level < Level::Summary {
            return;
        }
        self.print_path(path);
        self.print_result(
            self.skipped_level >= Level::Details,
            summary.dimmed(),
            details,
        );
    }

    fn started(&self, path: &Path) {
        if self.all_results_are_reported() {
            // we can already output the path
            self.print_path(path);
        }
    }

    fn succeeded(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>) {
        if self.succeeded_level < Level::Summary {
            return;
        }
        if !self.all_results_are_reported() {
            // the path wasn't outputed when started
            self.print_path(path);
        }
        self.print_result(
            self.succeeded_level >= Level::Details,
            summary.green(),
            details,
        )
    }

    fn failed(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>) {
        if self.failed_level < Level::Summary {
            return;
        }
        if !self.all_results_are_reported() {
            // the path wasn't outputed when started
            self.print_path(path);
        }
        self.print_result(self.failed_level >= Level::Details, summary.red(), details)
    }

    fn cancelled(&self, path: &Path, summary: &str, details: Option<&dyn std::fmt::Display>) {
        if self.canceled_level < Level::Summary {
            return;
        }
        if !self.all_results_are_reported() {
            // the path wasn't outputed when started
            self.print_path(path);
        }
        self.print_result(
            self.canceled_level >= Level::Details,
            summary.yellow(),
            details,
        )
    }
}
