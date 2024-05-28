// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines a test file format for stack graph resolution.
//!
//! ## Assertions
//!
//! Test files are source files in the language under test with assertions added in comments.
//! Assertions indicate the position of a reference or definition with a carrot `^` in the
//! source, and specify comma separated expected values.
//!
//! An example test for Python might be defined in `test.py` and look as follows:
//!
//! ``` skip
//! foo = 42
//! # ^ defines: foo
//! print(foo, bar)
//! #     ^ refers: foo
//! #     ^ defined: 1
//! #          ^ refers: bar
//! #          ^ defined:
//! ```
//!
//! Consecutive lines with assertions all apply to the last source line without an assertion.
//! In the example, both assertions refer to positions on line 3.
//!
//! The following assertions are supported:
//!
//!  - `defined`: takes a comma-separated list of line numbers, and expects a reference at this
//!    position to resolves to definitions on those lines.
//!  - `defines`: takes a comma-separated list of names, and expects definitions at this position
//!    with the given names.
//!  - `refers`: takes a comma-separated list of names, and expects references at this position
//!    with the given names.
//!
//! ## Fragments for multi-file testing
//!
//! Test files may also consist of multiple fragments, which are treated as separate files in the
//! stack graph. An example test that simulates two different Python files:
//!
//! ``` skip
//! # --- path: one.py ---
//! x = 42
//! y = -1
//! # --- path: one.py ---
//! print(x, y)
//! #     ^ defined: 2
//! #        ^ defined: 3
//! ```
//!
//! Note that the line numbers still refer to lines in the complete test file, and are not relative
//! to a fragment.
//!
//! Any content before the first fragment header of the file is ignored, and will not be part of the test.

use itertools::Itertools;
use lsp_positions::Position;
use lsp_positions::PositionedSubstring;
use lsp_positions::SpanCalculator;
use once_cell::sync::Lazy;
use regex::Regex;
use stack_graphs::arena::Handle;
use stack_graphs::assert::Assertion;
use stack_graphs::assert::AssertionError;
use stack_graphs::assert::AssertionSource;
use stack_graphs::assert::AssertionTarget;
use stack_graphs::graph::File;
use stack_graphs::graph::Node;
use stack_graphs::graph::SourceInfo;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use stack_graphs::stitching::StitcherConfig;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tree_sitter_graph::Variables;

use crate::CancellationFlag;

const DEFINED: &'static str = "defined";
const DEFINES: &'static str = "defines";
const REFERS: &'static str = "refers";

static PATH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"---\s*path:\s*([^\s]+)\s*---"#).unwrap());
static GLOBAL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"---\s*global:\s*([^\s]+)=([^\s]+)\s*---"#).unwrap());
static ASSERTION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(\^)\s*(\w+):\s*([^\s,]+(?:\s*,\s*[^\s,]+)*)?"#).unwrap());
static LINE_NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\d+"#).unwrap());
static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[^\s,]+"#).unwrap());

/// An error that can occur while parsing tests
#[derive(Debug, Error)]
pub enum TestError {
    AssertionRefersToNonSourceLine(usize),
    DuplicateGlobalVariable(usize, String),
    DuplicatePath(usize, String),
    GlobalBeforeFirstFragment(usize),
    InvalidAssertion(usize, String),
    InvalidColumn(usize, usize, usize),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssertionRefersToNonSourceLine(line) => {
                write!(
                    f,
                    "Assertion on line {} refers to non-source line",
                    line + 1
                )
            }
            Self::DuplicateGlobalVariable(line, global) => {
                write!(
                    f,
                    "Duplicate global variable {} on line {}",
                    global,
                    line + 1
                )
            }
            Self::DuplicatePath(line, path) => {
                write!(f, "Duplicate path {} on line {}", path, line + 1)
            }
            Self::GlobalBeforeFirstFragment(line) => {
                write!(f, "Global set before first fragment on line {}", line + 1)
            }
            Self::InvalidAssertion(line, assertion) => {
                write!(f, "Invalid assertion {} on line {}", assertion, line + 1)
            }
            Self::InvalidColumn(line, column, regular_line) => write!(
                f,
                "Assertion on line {} refers to missing column {} on line {}",
                line + 1,
                column + 1,
                regular_line + 1
            ),
        }
    }
}

/// A stack graph test
pub struct Test {
    pub path: PathBuf,
    pub fragments: Vec<TestFragment>,
    pub graph: StackGraph,
}

/// A fragment from a stack graph test
#[derive(Debug, Clone)]
pub struct TestFragment {
    pub file: Handle<File>,
    pub path: PathBuf,
    pub root_path: PathBuf,
    pub source: String,
    pub assertions: Vec<Assertion>,
    pub globals: HashMap<String, String>,
}

impl Test {
    /// Creates a test from source. If the test contains no `path` sections,
    /// the default fragment path is used for the test's single test fragment.
    pub fn from_source(
        path: &Path,
        source: &str,
        default_fragment_path: &Path,
    ) -> Result<Self, TestError> {
        let mut graph = StackGraph::new();
        let mut fragments = Vec::new();
        let mut have_fragments = false;
        let mut current_path = default_fragment_path.to_path_buf();
        let mut current_source = String::new();
        let mut current_globals = HashMap::new();
        let mut have_globals = false;
        let mut prev_source = String::new();
        let mut line_files = Vec::new();
        let mut line_count = 0;
        let default_root_path = PathBuf::from("");
        for (current_line_number, current_line) in
            PositionedSubstring::lines_iter(source).enumerate()
        {
            line_count += 1;
            if let Some(m) = PATH_REGEX.captures_iter(current_line.content).next() {
                // in a test with fragments, any content before the first fragment is
                // ignored, so that the file name of the test does not interfere with
                // the file names of the fragments
                if have_fragments {
                    let file = graph
                        .add_file(&current_path.to_string_lossy())
                        .map_err(|_| {
                            TestError::DuplicatePath(
                                line_files.len(),
                                format!("{}", current_path.display()),
                            )
                        })?;
                    (line_files.len()..current_line_number)
                        .for_each(|_| line_files.push(Some(file)));
                    fragments.push(TestFragment {
                        file,
                        path: current_path,
                        root_path: default_root_path.clone(),
                        source: current_source,
                        assertions: Vec::new(),
                        globals: current_globals,
                    });
                } else {
                    if have_globals {
                        return Err(TestError::GlobalBeforeFirstFragment(current_line_number));
                    }
                    have_fragments = true;
                    (line_files.len()..current_line_number).for_each(|_| line_files.push(None));
                }
                current_path = m.get(1).unwrap().as_str().into();
                current_source = prev_source.clone();
                current_globals = HashMap::new();

                Self::push_whitespace_for(&current_line, &mut current_source);
            } else if let Some(m) = GLOBAL_REGEX.captures_iter(current_line.content).next() {
                have_globals = true;
                let global_name = m.get(1).unwrap().as_str();
                let global_value = m.get(2).unwrap().as_str();
                if current_globals
                    .insert(global_name.into(), global_value.into())
                    .is_some()
                {
                    return Err(TestError::DuplicateGlobalVariable(
                        current_line_number,
                        global_name.to_string(),
                    ));
                }

                Self::push_whitespace_for(&current_line, &mut current_source);
            } else {
                current_source.push_str(current_line.content);
            }
            current_source.push_str("\n");

            Self::push_whitespace_for(&current_line, &mut prev_source);
            prev_source.push_str("\n");
        }
        {
            let file = graph
                .add_file(&current_path.to_string_lossy())
                .map_err(|_| {
                    TestError::DuplicatePath(
                        line_files.len(),
                        format!("{}", current_path.display()),
                    )
                })?;
            (line_files.len()..line_count).for_each(|_| line_files.push(Some(file)));
            fragments.push(TestFragment {
                file,
                path: current_path,
                root_path: default_root_path.clone(),
                source: current_source,
                assertions: Vec::new(),
                globals: current_globals,
            });
        }

        for fragment in &mut fragments {
            fragment
                .parse_assertions(&mut graph, |line| line_files.get(line).cloned().flatten())?;
        }

        Ok(Self {
            path: path.to_path_buf(),
            fragments,
            graph,
        })
    }

    /// Pushes whitespace equivalent to the given line into the string.
    /// This is used to "erase" preceding content in multi-file test.
    /// It is implemented as pushing as many SPACE-s as there are code
    /// units in the original line.
    /// * This preserves global UTF-8 positions, i.e., a UTF-8 position in a
    ///   test file source points to the same thing in the original source
    ///   and vice versa. However, global UTF-16 and grapheme positions are
    ///   not preserved.
    /// * Line numbers are preserved, i.e., a line in the test file source
    ///   has the same line number as in the overall source, and vice versa.
    /// * Positions within "erased" lines are not preserved, regardless of
    ///   whether they are UTF-8, UTF-16, or grapheme positions. Positions
    ///   in the actual content are preserved between the test file source and
    ///   the test source.
    fn push_whitespace_for(line: &PositionedSubstring, into: &mut String) {
        (0..line.utf8_bounds.end).for_each(|_| into.push_str(" "));
    }
}

impl TestFragment {
    /// Parse assertions in the source.
    fn parse_assertions<F>(&mut self, graph: &mut StackGraph, line_file: F) -> Result<(), TestError>
    where
        F: Fn(usize) -> Option<Handle<File>>,
    {
        self.assertions.clear();

        let mut current_line_span_calculator = SpanCalculator::new(&self.source);
        let mut last_regular_line: Option<PositionedSubstring> = None;
        let mut last_regular_line_number = None;
        let mut last_regular_line_span_calculator = SpanCalculator::new(&self.source);
        for (current_line_number, current_line) in
            PositionedSubstring::lines_iter(&self.source).enumerate()
        {
            if let Some(m) = ASSERTION_REGEX.captures_iter(current_line.content).next() {
                // assertion line
                let last_regular_line = last_regular_line.as_ref().ok_or_else(|| {
                    TestError::AssertionRefersToNonSourceLine(current_line_number)
                })?;
                let last_regular_line_number = last_regular_line_number.unwrap();

                let carret_match = m.get(1).unwrap();
                let assertion_match = m.get(2).unwrap();
                let values_match = m.get(3);

                let column_utf8_offset = carret_match.start();
                let column_grapheme_offset = current_line_span_calculator
                    .for_line_and_column(
                        current_line_number,
                        current_line.utf8_bounds.start,
                        column_utf8_offset,
                    )
                    .column
                    .grapheme_offset;
                if column_grapheme_offset >= last_regular_line.grapheme_length {
                    return Err(TestError::InvalidColumn(
                        current_line_number,
                        column_grapheme_offset,
                        last_regular_line_number,
                    ));
                }
                let position = last_regular_line_span_calculator.for_line_and_grapheme(
                    last_regular_line_number,
                    last_regular_line.utf8_bounds.start,
                    column_grapheme_offset,
                );
                let source = AssertionSource {
                    file: self.file,
                    position,
                };

                match assertion_match.as_str() {
                    DEFINED => {
                        let mut targets = Vec::new();
                        for line in LINE_NUMBER_REGEX
                            .find_iter(values_match.map(|m| m.as_str()).unwrap_or(""))
                        {
                            let line = line.as_str().parse::<usize>().unwrap() - 1;
                            let file = line_file(line).ok_or(
                                TestError::AssertionRefersToNonSourceLine(current_line_number),
                            )?;
                            targets.push(AssertionTarget { file, line });
                        }
                        self.assertions.push(Assertion::Defined { source, targets });
                    }
                    DEFINES => {
                        let mut symbols = Vec::new();
                        for name in
                            NAME_REGEX.find_iter(values_match.map(|m| m.as_str()).unwrap_or(""))
                        {
                            let symbol = graph.add_symbol(name.as_str());
                            symbols.push(symbol);
                        }
                        self.assertions.push(Assertion::Defines { source, symbols });
                    }
                    REFERS => {
                        let mut symbols = Vec::new();
                        for name in
                            NAME_REGEX.find_iter(values_match.map(|m| m.as_str()).unwrap_or(""))
                        {
                            let symbol = graph.add_symbol(name.as_str());
                            symbols.push(symbol);
                        }
                        self.assertions.push(Assertion::Refers { source, symbols });
                    }
                    _ => {
                        return Err(TestError::InvalidAssertion(
                            current_line_number,
                            assertion_match.as_str().to_string(),
                        ));
                    }
                }
            } else {
                // regular source line
                last_regular_line = Some(current_line);
                last_regular_line_number = Some(current_line_number);
            }
        }

        Ok(())
    }
}

/// Result of running a stack graph test.
#[derive(Debug, Clone)]
pub struct TestResult {
    success_count: usize,
    failures: Vec<TestFailure>,
}

impl TestResult {
    pub fn new() -> Self {
        Self {
            failures: Vec::new(),
            success_count: 0,
        }
    }

    fn add_success(&mut self) {
        self.success_count += 1;
    }

    fn add_failure(&mut self, reason: TestFailure) {
        self.failures.push(reason);
    }

    /// Number of successfull assertions.
    pub fn success_count(&self) -> usize {
        self.success_count
    }

    /// Number of failed assertions.
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn failures_iter(&self) -> std::slice::Iter<'_, TestFailure> {
        self.failures.iter()
    }

    pub fn into_failures_iter(self) -> std::vec::IntoIter<TestFailure> {
        self.failures.into_iter()
    }

    /// Total number of assertions that were run.
    pub fn count(&self) -> usize {
        self.success_count() + self.failure_count()
    }

    pub fn absorb(&mut self, other: TestResult) {
        self.success_count += other.success_count;
        let mut failures = other.failures;
        self.failures.append(&mut failures);
    }
}

impl std::fmt::Display for TestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} tests: {} passed, {} failed",
            self.count(),
            self.success_count(),
            self.failure_count()
        )
    }
}

/// Description of test failures.
// This mirrors AssertionError, but provides cleaner error messages. The underlying
// assertions report errors in terms of the virtual files in the test. This type
// ensures errors are reported in terms of locations in the original test file.
// This makes errors clickable in e.g. the VS Code console, improving the developer
// experience.
#[derive(Debug, Clone)]
pub enum TestFailure {
    NoReferences {
        path: PathBuf,
        position: Position,
    },
    IncorrectResolutions {
        path: PathBuf,
        position: Position,
        references: Vec<String>,
        missing_lines: Vec<usize>,
        unexpected_lines: HashMap<String, Vec<Option<usize>>>,
    },
    IncorrectDefinitions {
        path: PathBuf,
        position: Position,
        missing_symbols: Vec<String>,
        unexpected_symbols: Vec<String>,
    },
    IncorrectReferences {
        path: PathBuf,
        position: Position,
        missing_symbols: Vec<String>,
        unexpected_symbols: Vec<String>,
    },
    Cancelled(stack_graphs::CancellationError),
}

impl std::fmt::Display for TestFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoReferences { path, position } => {
                write!(
                    f,
                    "{}:{}:{}: no references found",
                    path.display(),
                    position.line + 1,
                    position.column.grapheme_offset + 1
                )
            }
            Self::IncorrectResolutions {
                path,
                position,
                references,
                missing_lines,
                unexpected_lines,
            } => {
                write!(
                    f,
                    "{}:{}:{}: ",
                    path.display(),
                    position.line + 1,
                    position.column.grapheme_offset + 1
                )?;
                write!(f, "definition(s) for reference(s)")?;
                for reference in references {
                    write!(f, " ‘{}’", reference)?;
                }
                if !missing_lines.is_empty() {
                    write!(
                        f,
                        " missing expected on line(s) {}",
                        missing_lines.iter().map(|l| l + 1).format(", ")
                    )?;
                }
                if !unexpected_lines.is_empty() {
                    write!(f, " found unexpected",)?;
                    let mut first = true;
                    for (definition, lines) in unexpected_lines.into_iter() {
                        if first {
                            first = false;
                        } else {
                            write!(f, ",")?;
                        }
                        write!(f, " ‘{}’ on lines(s) ", definition)?;
                        write!(
                            f,
                            "{}",
                            lines
                                .into_iter()
                                .map(|l| l.map(|l| format!("{}", l + 1)).unwrap_or("?".into()))
                                .format(", ")
                        )?;
                    }
                }
                Ok(())
            }
            Self::IncorrectDefinitions {
                path,
                position,
                missing_symbols,
                unexpected_symbols,
            } => {
                write!(
                    f,
                    "{}:{}:{}: definitions",
                    path.display(),
                    position.line + 1,
                    position.column.grapheme_offset + 1
                )?;
                if !missing_symbols.is_empty() {
                    write!(
                        f,
                        " missing expected {}",
                        missing_symbols.iter().format(", ")
                    )?;
                }
                if !unexpected_symbols.is_empty() {
                    write!(
                        f,
                        " found unexpected {}",
                        unexpected_symbols.iter().format(", ")
                    )?;
                }
                Ok(())
            }
            Self::IncorrectReferences {
                path,
                position,
                missing_symbols,
                unexpected_symbols,
            } => {
                write!(
                    f,
                    "{}:{}:{}: references",
                    path.display(),
                    position.line + 1,
                    position.column.grapheme_offset + 1
                )?;
                if !missing_symbols.is_empty() {
                    write!(
                        f,
                        " missing expected {}",
                        missing_symbols.iter().format(", ")
                    )?;
                }
                if !unexpected_symbols.is_empty() {
                    write!(
                        f,
                        " found unexpected {}",
                        unexpected_symbols.iter().format(", ")
                    )?;
                }
                Ok(())
            }
            Self::Cancelled(err) => write!(f, "{}", err),
        }
    }
}

impl Test {
    /// Run the test. It is the responsibility of the caller to ensure that
    /// the stack graph for the test fragments has been constructed, and the
    /// database has been filled with partial paths before running the test.
    pub fn run(
        &mut self,
        partials: &mut PartialPaths,
        db: &mut Database,
        stitcher_config: StitcherConfig,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<TestResult, stack_graphs::CancellationError> {
        let mut result = TestResult::new();
        for fragment in &self.fragments {
            for assertion in &fragment.assertions {
                match assertion
                    .run(
                        &self.graph,
                        partials,
                        db,
                        stitcher_config,
                        &cancellation_flag,
                    )
                    .map_or_else(|e| self.from_error(e), |v| Ok(v))
                {
                    Ok(_) => result.add_success(),
                    Err(f) => result.add_failure(f),
                }
            }
        }
        Ok(result)
    }

    /// Construct a TestFailure from an AssertionError.
    fn from_error(&self, err: AssertionError) -> Result<(), TestFailure> {
        match err {
            AssertionError::NoReferences { source } => Err(TestFailure::NoReferences {
                path: self.path.clone(),
                position: source.position,
            }),
            AssertionError::IncorrectlyDefined {
                source,
                references,
                missing_targets,
                unexpected_paths,
            } => {
                let references = references
                    .into_iter()
                    .map(|r| self.graph[self.graph[r].symbol().unwrap()].to_string())
                    .unique()
                    .sorted()
                    .collect();
                let missing_lines = missing_targets
                    .into_iter()
                    .map(|t| t.line)
                    .unique()
                    .sorted()
                    .collect::<Vec<_>>();
                let unexpected_lines = unexpected_paths
                    .into_iter()
                    .filter(|p| {
                        // ignore results outside of this test, which may be include files or builtins
                        self.fragments
                            .iter()
                            .any(|f| f.file == self.graph[p.end_node].id().file().unwrap())
                    })
                    .map(|p| {
                        let symbol =
                            self.graph[self.graph[p.end_node].symbol().unwrap()].to_string();
                        let line = self
                            .get_source_info(p.end_node)
                            .map(|si| si.span.start.line);
                        (symbol, line)
                    })
                    .unique()
                    .sorted()
                    .into_group_map();
                if missing_lines.is_empty() && unexpected_lines.is_empty() {
                    return Ok(());
                }
                Err(TestFailure::IncorrectResolutions {
                    path: self.path.clone(),
                    position: source.position,
                    references,
                    missing_lines,
                    unexpected_lines,
                })
            }
            AssertionError::IncorrectDefinitions {
                source,
                missing_symbols,
                unexpected_symbols,
            } => {
                let missing_symbols = missing_symbols
                    .iter()
                    .map(|s| self.graph[*s].to_string())
                    .collect::<Vec<_>>();
                let unexpected_symbols = unexpected_symbols
                    .iter()
                    .map(|s| self.graph[*s].to_string())
                    .collect::<Vec<_>>();
                Err(TestFailure::IncorrectDefinitions {
                    path: self.path.clone(),
                    position: source.position,
                    missing_symbols,
                    unexpected_symbols,
                })
            }
            AssertionError::IncorrectReferences {
                source,
                missing_symbols,
                unexpected_symbols,
            } => {
                let missing_symbols = missing_symbols
                    .iter()
                    .map(|s| self.graph[*s].to_string())
                    .collect::<Vec<_>>();
                let unexpected_symbols = unexpected_symbols
                    .iter()
                    .map(|s| self.graph[*s].to_string())
                    .collect::<Vec<_>>();
                Err(TestFailure::IncorrectReferences {
                    path: self.path.clone(),
                    position: source.position,
                    missing_symbols,
                    unexpected_symbols,
                })
            }
            AssertionError::Cancelled(err) => Err(TestFailure::Cancelled(err)),
        }
    }

    /// Get source info for a node, using a heuristic to rule default null source info results.
    fn get_source_info(&self, node: Handle<Node>) -> Option<&SourceInfo> {
        self.graph.source_info(node).filter(|si| {
            !(si.span.start.line == 0
                && si.span.start.column.utf8_offset == 0
                && si.span.end.line == 0
                && si.span.end.column.utf8_offset == 0)
        })
    }
}

impl TestFragment {
    pub fn add_globals_to(&self, variables: &mut Variables) {
        for (name, value) in self.globals.iter() {
            variables
                .add(name.as_str().into(), value.as_str().into())
                .unwrap();
        }
    }
}
