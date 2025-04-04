// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use base64::Engine;
use clap::builder::PathBufValueParser;
use clap::builder::TypedValueParser;
use clap::error::ContextKind;
use clap::error::ContextValue;
use clap::error::ErrorKind;
use lsp_positions::Span;
use sha1::Digest;
use sha1::Sha1;
use stack_graphs::arena::Handle;
use stack_graphs::graph::Node;
use stack_graphs::graph::StackGraph;
use stack_graphs::stats::FrequencyDistribution;
use stack_graphs::stitching::Stats as StitchingStats;
use stack_graphs::storage::Stats as StorageStats;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt::Display;
use std::hash::Hash;
use std::io::Write;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use walkdir::WalkDir;

use crate::cli::index::IndexingStats;
use crate::cli::util::reporter::Reporter;

pub mod reporter;

#[derive(Clone)]
pub(crate) struct ExistingPathBufValueParser;

impl TypedValueParser for ExistingPathBufValueParser {
    type Value = PathBuf;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner = PathBufValueParser::new();
        let value = inner.parse_ref(cmd, arg, value)?;

        if value.exists() {
            return Ok(value);
        }

        let mut err = clap::Error::new(ErrorKind::ValueValidation);
        if let Some(arg) = arg {
            err.insert(
                ContextKind::InvalidArg,
                ContextValue::String(arg.to_string()),
            );
        }
        err.insert(
            ContextKind::InvalidValue,
            ContextValue::String(value.to_string_lossy().to_string()),
        );
        err.insert(
            ContextKind::Custom,
            ContextValue::String("path does not exist".to_string()),
        );

        Err(err)
    }
}

/// A path specification that can be formatted into a path based on a root and path
/// contained in that root.
#[derive(Clone)]
pub struct PathSpec {
    spec: String,
}

impl PathSpec {
    pub fn format(&self, root: &Path, full_path: &Path) -> PathBuf {
        if !full_path.starts_with(root) {
            panic!(
                "Path {} not contained in root {}",
                full_path.display(),
                root.display()
            );
        }
        let relative_path = full_path.strip_prefix(root).unwrap();
        if relative_path.is_absolute() {
            panic!(
                "Path {} not relative to root {}",
                full_path.display(),
                root.display()
            );
        }
        self.format_path(
            &self.dir_os_str(Some(root)),
            &self.dir_os_str(relative_path.parent()),
            relative_path.file_stem(),
            relative_path.extension(),
        )
    }

    /// Convert an optional directory path to an OsString representation. If the
    /// path is missing or empty, we return `.`.
    fn dir_os_str(&self, path: Option<&Path>) -> OsString {
        let s = path.map_or("".into(), |p| p.as_os_str().to_os_string());
        let s = if s.is_empty() { ".".into() } else { s };
        s
    }

    fn format_path(
        &self,
        root: &OsStr,
        dirs: &OsStr,
        name: Option<&OsStr>,
        ext: Option<&OsStr>,
    ) -> PathBuf {
        let mut path = OsString::new();
        let mut in_placeholder = false;
        for c in self.spec.chars() {
            if in_placeholder {
                in_placeholder = false;
                match c {
                    '%' => path.push("%"),
                    'd' => {
                        path.push(dirs);
                    }
                    'e' => {
                        if let Some(ext) = ext {
                            path.push(".");
                            path.push(ext);
                        }
                    }
                    'n' => {
                        if let Some(name) = name {
                            path.push(name);
                        }
                    }
                    'r' => path.push(root),
                    c => panic!("Unsupported placeholder '%{}'", c),
                }
            } else if c == '%' {
                in_placeholder = true;
            } else {
                path.push(c.to_string());
            }
        }
        if in_placeholder {
            panic!("Unsupported '%' at end");
        }
        let path = Path::new(&path);
        match crate::functions::path::normalize(&path) {
            Some(path) => path,
            None => panic!("Cannot normalize '{}'", path.display()),
        }
    }
}

impl std::str::FromStr for PathSpec {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { spec: s.into() })
    }
}

impl From<&str> for PathSpec {
    fn from(s: &str) -> Self {
        Self { spec: s.into() }
    }
}

pub trait SourceIterator {
    fn iter_references<'a>(
        &'a self,
        graph: &'a StackGraph
    ) -> impl Iterator<Item = (Handle<Node>, Span)> + 'a;

    fn get_path<'a>(&'a self) -> &'a PathBuf;
}

#[derive(Clone, Debug)]
/// A source position.
pub struct SourcePosition {
    /// File path
    pub path: PathBuf,
    /// Position line (0-based)
    pub line: usize,
    /// Position column (0-based grapheme)
    pub column: usize,
}

impl SourcePosition {
    fn within_span(&self, span: &lsp_positions::Span) -> bool {
        ((span.start.line < self.line)
            || (span.start.line == self.line && span.start.column.grapheme_offset <= self.column))
            && ((span.end.line == self.line && span.end.column.grapheme_offset >= self.column)
                || (span.end.line > self.line))
    }

    pub fn canonicalize(&mut self) -> std::io::Result<()> {
        self.path = self.path.canonicalize()?;
        Ok(())
    }
}

impl SourceIterator for SourcePosition {
    fn iter_references<'a>(
        &'a self,
        graph: &'a StackGraph,
    ) -> impl Iterator<Item = (Handle<Node>, Span)> + 'a {
        graph
            .get_file(&self.path.to_string_lossy())
            .into_iter()
            .flat_map(move |file| {
                graph.nodes_for_file(file).filter_map(move |node| {
                    if !graph[node].is_reference() {
                        return None;
                    }
                    let source_info = match graph.source_info(node) {
                        Some(source_info) => source_info,
                        None => return None,
                    };
                    if !self.within_span(&source_info.span) {
                        return None;
                    }
                    Some((node, source_info.span.clone()))
                })
            })
    }

    fn get_path<'a>(&'a self) -> &'a PathBuf {
        &self.path
    }
}

impl std::fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.path.display(),
            self.line + 1,
            self.column + 1
        )
    }
}

impl std::str::FromStr for SourcePosition {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut values = s.split(':');
        let path = match values.next() {
            Some(path) => PathBuf::from(path),
            None => return Err(anyhow!("Missing path in expected format PATH:LINE:COLUMN")),
        };
        let line = match values.next() {
            Some(line) => {
                let line = usize::from_str(line).map_err(|_| {
                    anyhow!(
                        "Expected line to be a number, got {} in expected format PATH:LINE:COLUMN",
                        line
                    )
                })?;
                if line == 0 {
                    return Err(anyhow!(
                        "Line numbers are 1-based, got 0 in expected format PATH:LINE:COLUMN"
                    ));
                }
                line - 1
            }
            None => {
                return Err(anyhow!(
                    "Missing line and column numbers in expected format PATH:LINE:COLUMN"
                ))
            }
        };
        let column = match values.next() {
            Some(column) => {
                let column = usize::from_str(column)
                    .map_err(|_| anyhow!("Expected column to be a number, got {} in expected format PATH:LINE:COLUMN", column))?;
                if column == 0 {
                    return Err(anyhow!(
                        "Column numbers are 1-based, got 0 in expected format PATH:LINE:COLUMN"
                    ));
                }
                column - 1
            }
            None => {
                return Err(anyhow!(
                    "Missing column number in expected format PATH:LINE:COLUMN"
                ))
            }
        };
        if values.next().is_some() {
            return Err(anyhow!(
                "Found unexpected components in expected format PATH:LINE:COLUMN"
            ));
        }
        Ok(Self { path, line, column })
    }
}

#[derive(Clone, Debug)]
/// A source span.
pub struct SourceSpan {
    /// File path
    pub path: PathBuf,
    /// Span
    pub span: Span,
}

impl SourceSpan {
    pub(crate) fn first_line(&self) -> usize {
        self.span.start.line
    }

    /// Returns a range for the first line of this span. If multiple lines are spanned, it
    /// will use usize::MAX for the range's end.
    pub(crate) fn first_line_column_range(&self) -> Range<usize> {
        let start = self.span.start.column.grapheme_offset;
        let end = if self.span.start.line == self.span.end.line {
            self.span.end.column.grapheme_offset
        } else {
            usize::MAX
        };
        start..end
    }

    fn within_span(&self, span: &lsp_positions::Span) -> bool {
        ((self.span.start.line >= span.start.line)
            && (self.span.start.line <= span.end.line))
        ||
        ((self.span.end.line >= span.start.line)
            && (self.span.end.line <= span.end.line))
    }

    pub fn canonicalize(&mut self) -> std::io::Result<()> {
        self.path = self.path.canonicalize()?;
        Ok(())
    }
}

impl SourceIterator for SourceSpan {

    fn iter_references<'a>(
        &'a self,
        graph: &'a StackGraph,
    ) -> impl Iterator<Item = (Handle<Node>, Span)> + 'a {
        graph
            .get_file(&self.path.to_string_lossy())
            .into_iter()
            .flat_map(move |file| {
                graph.nodes_for_file(file).filter_map(move |node| {
                    if !graph[node].is_reference() {
                        return None;
                    }
                    let source_info = match graph.source_info(node) {
                        Some(source_info) => source_info,
                        None => return None,
                    };
                    if !self.within_span(&source_info.span) {
                        return None;
                    }
                    Some((node, source_info.span.clone()))
                })
            })
    }

    fn get_path<'a>(&'a self) -> &'a PathBuf {
        &self.path
    }
}

impl std::fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}-{}",
            self.path.display(),
            self.span.start.line + 1,
            self.span.end.line + 1,
        )
    }
}

impl std::str::FromStr for SourceSpan {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut values = s.split(':');
        let path = match values.next() {
            Some(path) => PathBuf::from(path),
            None => return Err(anyhow!("Missing path in expected format PATH:LINE_LO:LINE_HI")),
        };
        let line_lo = match values.next() {
            Some(line_lo) => {
                let line = usize::from_str(line_lo).map_err(|_| {
                    anyhow!(
                        "Expected line-lo to be a number, got {} in expected format PATH:LINE_LO:LINE_HI",
                        line_lo
                    )
                })?;
                if line == 0 {
                    return Err(anyhow!(
                        "Line numbers are 1-based, got 0 in expected format PATH:LINE_LO:LINE_HI"
                    ));
                }
                line - 1
            }
            None => {
                return Err(anyhow!(
                    "Missing line numbers in expected format PATH:LINE_LO:LINE_HI"
                ))
            }
        };
        let line_hi = match values.next() {
            Some(column) => {
                let column = usize::from_str(column)
                    .map_err(|_| anyhow!("Expected line-hi to be a number, got {} in expected format PATH:LINE_LO:LINE_HI", column))?;
                if column == 0 {
                    return Err(anyhow!(
                        "Line numbers are 1-based, got 0 in expected format PATH:LINE_LO:LINE_HI"
                    ));
                }
                column - 1
            }
            None => {
                return Err(anyhow!(
                    "Missing line -hi number in expected format PATH:LINE_LO:LINE_HI"
                ))
            }
        };
        if values.next().is_some() {
            return Err(anyhow!(
                "Found unexpected components in expected format PATH:LINE_LO:LINE_HI"
            ));
        }
        Ok(Self {
            path,
            span: Span {
                start: lsp_positions::Position {
                    line: line_lo,
                    column: lsp_positions::Offset {
                        utf8_offset: 0,
                        utf16_offset: 0,
                        grapheme_offset: 0
                    },
                    containing_line: Range { start: 0, end: 0 },
                    trimmed_line: Range { start: 0, end: 0 },
                },
                end: lsp_positions::Position {
                    line: line_hi,
                    column: lsp_positions::Offset {
                        utf8_offset: 0,
                        utf16_offset: 0,
                        grapheme_offset: 0
                    },
                    containing_line: Range { start: 0, end: 0 },
                    trimmed_line: Range { start: 0, end: 0 },
                },
            }
        })
    }
}

pub(crate) fn duration_from_seconds_str(s: &str) -> Result<Duration, anyhow::Error> {
    let seconds = s.parse::<u64>()?;
    Ok(Duration::new(seconds, 0))
}

#[cfg(feature = "lsp")]
pub(crate) fn duration_from_milliseconds_str(s: &str) -> Result<Duration, anyhow::Error> {
    let milliseconds = s.parse::<u64>()?;
    let seconds = milliseconds / 1000;
    let nano_seconds = (milliseconds % 1000) as u32 * 1_000_000;
    Ok(Duration::new(seconds, nano_seconds))
}

pub fn iter_files_and_directories<'a, P, IP>(
    paths: IP,
) -> impl Iterator<Item = (PathBuf, PathBuf, bool)> + 'a
where
    P: AsRef<Path> + 'a,
    IP: IntoIterator<Item = P> + 'a,
{
    paths
        .into_iter()
        .filter_map(
            |source_path| -> Option<Box<dyn Iterator<Item = (PathBuf, PathBuf, bool)>>> {
                if source_path.as_ref().is_dir() {
                    let source_root = source_path;
                    let paths = WalkDir::new(&source_root)
                        .follow_links(true)
                        .sort_by_file_name()
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                        .map(move |e| (source_root.as_ref().to_path_buf(), e.into_path(), false));
                    Some(Box::new(paths))
                } else {
                    let source_root = source_path
                        .as_ref()
                        .parent()
                        .expect("expect file to have parent");
                    Some(Box::new(std::iter::once((
                        source_root.to_path_buf(),
                        source_path.as_ref().to_path_buf(),
                        true,
                    ))))
                }
            },
        )
        .flatten()
}

/// Wraps a reporter and ensures that reporter is called properly without requiring
/// the caller of the wrapper to be overly careful about which methods must be called
/// in which order
pub(super) struct CLIFileReporter<'a> {
    reporter: &'a dyn Reporter,
    path: &'a Path,
    path_logged: bool,
    status_logged: bool,
}

impl<'a> CLIFileReporter<'a> {
    pub(super) fn new(reporter: &'a dyn Reporter, path: &'a Path) -> Self {
        Self {
            reporter,
            path,
            path_logged: false,
            status_logged: false,
        }
    }

    pub(super) fn processing(&mut self) {
        if self.path_logged {
            panic!("Already started or finished");
        }
        self.reporter.started(self.path);
        self.path_logged = true;
    }

    fn ensure_started(&mut self) {
        if self.status_logged {
            panic!("Status already logged");
        }
        if !self.path_logged {
            self.reporter.started(self.path);
            self.path_logged = true;
        }
    }

    pub(super) fn success(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        self.ensure_started();
        self.reporter.succeeded(self.path, status, details);
        self.status_logged = true;
    }

    pub(super) fn skipped(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        if self.path_logged {
            panic!("Skipped after starting");
        }
        if self.status_logged {
            panic!("Status already logged");
        }
        self.reporter.skipped(self.path, status, details);
        self.status_logged = true;
    }

    pub(super) fn warning(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        self.ensure_started();
        self.reporter.cancelled(self.path, status, details);
        self.status_logged = true;
    }

    pub(super) fn failure(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        self.ensure_started();
        self.reporter.failed(self.path, status, details);
        self.status_logged = true;
    }

    pub(super) fn failure_if_processing(
        &mut self,
        status: &str,
        details: Option<&dyn std::fmt::Display>,
    ) {
        if !self.path_logged || self.status_logged {
            return;
        }
        self.failure(status, details);
    }

    pub(super) fn assert_reported(&mut self) {
        if self.path_logged && !self.status_logged {
            panic!("status not reported");
        }
    }
}

pub(crate) fn sha1(value: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value);
    base64::prelude::BASE64_STANDARD_NO_PAD.encode(hasher.finalize())
}

pub(crate) fn wait_for_input() -> anyhow::Result<()> {
    print!("<press ENTER to continue>");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}

/// Wraps a build error with the relevant sources
pub(crate) struct BuildErrorWithSource<'a> {
    pub inner: crate::BuildError,
    pub source_path: PathBuf,
    pub source_str: &'a str,
    pub tsg_path: PathBuf,
    pub tsg_str: &'a str,
}

impl<'a> BuildErrorWithSource<'a> {
    pub fn display_pretty(&'a self) -> impl std::fmt::Display + 'a {
        DisplayBuildErrorPretty(self)
    }
}

struct DisplayBuildErrorPretty<'a>(&'a BuildErrorWithSource<'a>);

impl std::fmt::Display for DisplayBuildErrorPretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.inner.display_pretty(
                &self.0.source_path,
                self.0.source_str,
                &self.0.tsg_path,
                self.0.tsg_str,
            )
        )
    }
}

pub(super) fn print_indexing_stats(stats: IndexingStats) {
    print_quartiles_header("graph stats");
    print_quartiles_row("total graph nodes", stats.total_graph_nodes);
    print_quartiles_row("total graph edges", stats.total_graph_edges);
    print_quartiles_row("node out degrees", stats.node_out_degrees);
    print_value_row("root out degree", stats.root_out_degree);
    println!();
    print_stitching_stats(stats.stitching_stats);
}

pub(super) fn print_stitching_stats(stats: StitchingStats) {
    print_quartiles_header("stitching stats");
    print_quartiles_row("initial paths", stats.initial_paths);
    print_quartiles_row("queued paths per phase", stats.queued_paths_per_phase);
    print_quartiles_row("processed paths per phase", stats.processed_paths_per_phase);
    print_quartiles_row("accepted path length", stats.accepted_path_length);
    print_quartiles_row("terminal path length", stats.terminal_path_lengh);
    print_quartiles_row("node path candidates", stats.candidates_per_node_path);
    print_quartiles_row("node path extensions", stats.extensions_per_node_path);
    print_quartiles_row("root path candidates", stats.candidates_per_root_path);
    print_quartiles_row("root path extensions", stats.extensions_per_root_path);
    print_quartiles_row("node visits", stats.node_visits.frequencies());
    print_value_row("root visits", stats.root_visits);
    print_quartiles_row(
        "similar path counts",
        stats.similar_paths_stats.similar_path_count,
    );
    print_quartiles_row(
        "similar path bucket sizes",
        stats.similar_paths_stats.similar_path_bucket_size,
    );
}

pub(super) fn print_database_stats(stats: StorageStats) {
    println!(
        "| {:^29} | {:^9} | {:^9} |",
        "database stats", "loads", "cached",
    );
    println!("|-------------------------------|-----------|-----------|");
    println!(
        "| {:>29} | {:>9} | {:>9} |",
        "files", stats.file_loads, stats.file_cached
    );
    println!(
        "| {:>29} | {:>9} | {:>9} |",
        "node paths", stats.node_path_loads, stats.node_path_cached
    );
    println!(
        "| {:>29} | {:>9} | {:>9} |",
        "rootpaths", stats.root_path_loads, stats.root_path_cached
    );
}

fn print_quartiles_header(title: &str) {
    println!(
        "| {:^29} | {:^9} | {:^9} | {:^9} | {:^9} | {:^9} | {:^9} |",
        title, "min", "p25", "p50", "p75", "max", "count",
    );
    println!(
        "|-------------------------------|-----------|-----------|-----------|-----------|-----------|-----------|"
    );
}

fn print_quartiles_row<X: Display + Eq + Hash + Ord>(title: &str, hist: FrequencyDistribution<X>) {
    let qs = hist.quantiles(4);
    if qs.is_empty() {
        println!(
            "| {:>29} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} |",
            title, "-", "-", "-", "-", "-", 0
        );
    } else {
        println!(
            "| {:>29} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} |",
            title,
            qs[0],
            qs[1],
            qs[2],
            qs[3],
            qs[4],
            hist.count(),
        );
    }
}

fn print_value_row<X: Display>(title: &str, value: X) {
    println!(
        "| {:>29} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} |",
        title, "-", "-", "-", "-", "-", value
    );
}
