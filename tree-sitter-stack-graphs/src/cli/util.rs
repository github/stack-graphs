// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use base64::Engine;
use colored::Colorize;
use lsp_positions::Span;
use sha1::Digest;
use sha1::Sha1;
use stack_graphs::arena::Handle;
use stack_graphs::graph::Node;
use stack_graphs::graph::StackGraph;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
#[cfg(debug_assertions)]
use std::time::Instant;
use walkdir::WalkDir;

pub fn path_exists(path: &OsStr) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(anyhow!("path does not exist"));
    }
    Ok(path)
}

/// A path specification that can be formatted into a path based on a root and path
/// contained in that root.
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
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        graph
            .get_file(&self.path.to_string_lossy())
            .into_iter()
            .flat_map(move |file| {
                graph.nodes_for_file(file).filter(move |n| {
                    graph[*n].is_reference()
                        && graph
                            .source_info(*n)
                            .map(|si| self.within_span(&si.span))
                            .unwrap_or(false)
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
    pub fn to_start_position(&self) -> SourcePosition {
        SourcePosition {
            path: self.path.clone(),
            line: self.span.start.line,
            column: self.span.start.column.grapheme_offset,
        }
    }

    pub fn into_start_position(self) -> SourcePosition {
        SourcePosition {
            path: self.path,
            line: self.span.start.line,
            column: self.span.start.column.grapheme_offset,
        }
    }
}

pub fn duration_from_seconds_str(s: &str) -> Result<Duration, anyhow::Error> {
    Ok(Duration::new(s.parse()?, 0))
}

pub fn duration_from_milliseconds_str(s: &str) -> Result<Duration, anyhow::Error> {
    Ok(Duration::new(0, 1_000_000_u32 * s.parse::<u32>()?))
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

pub trait Logger {
    fn file<'a>(&self, path: &'a Path) -> Box<dyn FileLogger + 'a>;
}

pub trait FileLogger {
    fn processing(&mut self) {}
    fn failure(&mut self, _status: &str, _details: Option<&dyn std::fmt::Display>) {}
    fn skipped(&mut self, _status: &str, _details: Option<&dyn std::fmt::Display>) {}
    fn success(&mut self, _status: &str, _details: Option<&dyn std::fmt::Display>) {}
    fn warning(&mut self, _status: &str, _details: Option<&dyn std::fmt::Display>) {}
    fn default_failure(&mut self, _status: &str, _details: Option<&dyn std::fmt::Display>) {}
}

pub struct ConsoleLogger {
    show_info: bool,
    show_details: bool,
}

impl ConsoleLogger {
    pub fn new(show_info: bool, show_details: bool) -> Self {
        Self {
            show_info,
            show_details,
        }
    }
}

impl Logger for ConsoleLogger {
    fn file<'a>(&self, path: &'a Path) -> Box<dyn FileLogger + 'a> {
        Box::new(ConsoleFileLogger::new(
            path,
            self.show_info,
            self.show_details,
        ))
    }
}

pub struct ConsoleFileLogger<'a> {
    path: &'a Path,
    show_info: bool,
    show_details: bool,
    path_logged: bool,
    #[cfg(debug_assertions)]
    processing_started: Option<Instant>,
}

impl<'a> ConsoleFileLogger<'a> {
    pub fn new(path: &'a Path, show_info: bool, show_details: bool) -> Self {
        Self {
            path,
            show_info,
            show_details,
            path_logged: false,
            #[cfg(debug_assertions)]
            processing_started: None,
        }
    }

    fn print_path(&mut self) {
        if self.path_logged {
            return;
        }
        print!("{}: ", self.path.display());
        self.path_logged = true;
    }

    #[cfg(debug_assertions)]
    fn print_processing_time(&mut self) {
        if let Some(processing_started) = self.processing_started {
            print!(" [{:.2} s]", processing_started.elapsed().as_secs_f64());
        }
    }

    fn flush(&mut self) {
        std::io::stdout().flush().expect("flush should succeed");
    }
}

impl FileLogger for ConsoleFileLogger<'_> {
    fn processing(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.processing_started = Some(Instant::now());
        }
        if !self.show_info {
            return;
        }
        self.print_path();
        self.flush();
    }

    fn success(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        if !self.show_info {
            return;
        }
        self.print_path();
        print!("{}", status.green());
        #[cfg(debug_assertions)]
        self.print_processing_time();
        println!();
        self.path_logged = false;
        self.flush();
        if !self.show_details {
            return;
        }
        if let Some(details) = details {
            println!("{}", details);
        }
    }

    fn skipped(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        if !self.show_info {
            return;
        }
        self.print_path();
        print!("{}", status.dimmed());
        #[cfg(debug_assertions)]
        self.print_processing_time();
        println!();
        self.path_logged = false;
        self.flush();
        if !self.show_details {
            return;
        }
        if let Some(details) = details {
            println!("{}", details);
        }
    }

    fn warning(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        self.print_path();
        print!("{}", status.yellow());
        #[cfg(debug_assertions)]
        self.print_processing_time();
        println!();
        self.path_logged = false;
        self.flush();
        if !self.show_details {
            return;
        }
        if let Some(details) = details {
            println!("{}", details);
        }
    }

    fn failure(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        self.print_path();
        print!("{}", status.red());
        #[cfg(debug_assertions)]
        self.print_processing_time();
        println!();
        self.path_logged = false;
        self.flush();
        if !self.show_details {
            return;
        }
        if let Some(details) = details {
            println!("{}", details);
        }
    }

    fn default_failure(&mut self, status: &str, details: Option<&dyn std::fmt::Display>) {
        if !self.path_logged {
            return;
        }
        self.failure(status, details);
    }
}

pub fn sha1(value: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value);
    base64::prelude::BASE64_STANDARD_NO_PAD.encode(hasher.finalize())
}

pub fn wait_for_input() -> anyhow::Result<()> {
    print!("<press ENTER to continue>");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}
