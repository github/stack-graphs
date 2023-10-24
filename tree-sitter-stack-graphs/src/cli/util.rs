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
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Write;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use walkdir::WalkDir;

use self::reporter::Reporter;

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
    pub fn iter_references<'a>(
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
        if !self.path_logged {
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
