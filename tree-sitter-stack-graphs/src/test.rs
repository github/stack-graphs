// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines

use itertools::Itertools;
use lazy_static::lazy_static;
use lsp_positions::Position;
use lsp_positions::PositionedSubstring;
use lsp_positions::SpanCalculator;
use regex::Regex;
use stack_graphs::arena::Handle;
use stack_graphs::assert::Assertion;
use stack_graphs::assert::AssertionError;
use stack_graphs::assert::AssertionSource;
use stack_graphs::assert::AssertionTarget;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;
use thiserror::Error;

lazy_static! {
    static ref PATH_REGEX: Regex = Regex::new(r#"---\s*path:\s*([^\s]+)\s*---"#).unwrap();
    static ref ASSERTION_REGEX: Regex =
        Regex::new(r#"(\^)\s*defined:\s*(\d+(?:\s*,\s*\d+)*)?"#).unwrap();
    static ref LINE_NUMBER_REGEX: Regex = Regex::new(r#"\d+"#).unwrap();
}

/// An error that can occur while parsing tests
#[derive(Debug, Error)]
pub enum TestError {
    AssertionOnFirstLine,
    DuplicatePath(String),
    InvalidColumn(usize, usize, usize),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssertionOnFirstLine => write!(f, "Assertion cannot appear on first line"),
            Self::DuplicatePath(path) => write!(f, "Duplicate path {}", path),
            Self::InvalidColumn(assertion_line, column, regular_line) => write!(
                f,
                "Assertion on line {} refers to missing column {} on line {}",
                assertion_line + 1,
                column + 1,
                regular_line + 1
            ),
        }
    }
}

/// A stack graph test
pub struct Test {
    pub path: String,
    pub files: Vec<TestFile>,
    pub graph: StackGraph,
    pub paths: Paths,
}

/// A file from a stack graph test
#[derive(Debug, Clone)]
pub struct TestFile {
    pub file: Handle<File>,
    pub source: String,
    pub assertions: Vec<Assertion>,
}

impl Test {
    /// Creates a test from source.
    pub fn from_source(path: &str, source: &str) -> Result<Self, TestError> {
        let mut graph = StackGraph::new();
        let mut test_files = Vec::new();
        let mut current_file = graph.add_file(path).unwrap();
        let mut current_source = String::new();
        let mut prev_source = String::new();
        let mut line_files = Vec::new();
        for (current_line_number, current_line) in
            PositionedSubstring::lines_iter(source).enumerate()
        {
            if let Some(m) = PATH_REGEX.captures_iter(current_line.content).next() {
                // if the test starts with a file header, we do not create an empty entry for the
                // test file itself
                if current_line_number != 0 {
                    test_files.push(TestFile {
                        file: current_file,
                        source: current_source,
                        assertions: Vec::new(),
                    });
                }
                let path = m.get(1).unwrap().as_str();
                current_file = graph
                    .add_file(path)
                    .map_err(|_| TestError::DuplicatePath(path.to_string()))?;
                current_source = prev_source.clone();
            }

            current_source.push_str(current_line.content);
            current_source.push_str("\n");

            line_files.push(current_file);

            Self::push_whitespace_for(&current_line, &mut prev_source);
            prev_source.push_str("\n");
        }
        test_files.push(TestFile {
            file: current_file,
            source: current_source,
            assertions: Vec::new(),
        });

        for test_file in &mut test_files {
            test_file.parse_assertions(|l| line_files[l])?;
        }

        Ok(Self {
            path: path.to_string(),
            files: test_files,
            graph,
            paths: Paths::new(),
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

impl TestFile {
    /// Parse assertions in the source.
    fn parse_assertions<F>(&mut self, line_file: F) -> Result<(), TestError>
    where
        F: Fn(usize) -> Handle<File>,
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
                let last_regular_line = last_regular_line
                    .as_ref()
                    .ok_or_else(|| TestError::AssertionOnFirstLine)?;
                let last_regular_line_number = last_regular_line_number.unwrap();

                let carret_match = m.get(1).unwrap();
                let line_numbers_match = m.get(2);

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

                let targets = LINE_NUMBER_REGEX
                    .find_iter(line_numbers_match.map(|m| m.as_str()).unwrap_or(""))
                    .map(|l| l.as_str().parse::<usize>().unwrap() - 1)
                    .map(|l| AssertionTarget {
                        file: line_file(l),
                        line: l,
                    })
                    .collect::<Vec<_>>();

                self.assertions.push(Assertion::Defined { source, targets });
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
    fn new() -> Self {
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
        path: String,
        position: Position,
    },
    IncorrectDefinitions {
        path: String,
        position: Position,
        symbols: Vec<String>,
        missing_lines: Vec<usize>,
        unexpected_lines: Vec<usize>,
    },
}

impl std::fmt::Display for TestFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoReferences { path, position } => {
                write!(
                    f,
                    "{}:{}:{}: no references found",
                    path,
                    position.line + 1,
                    position.column.grapheme_offset + 1
                )
            }
            Self::IncorrectDefinitions {
                path,
                position,
                symbols,
                missing_lines,
                unexpected_lines,
            } => {
                write!(
                    f,
                    "{}:{}:{}: ",
                    path,
                    position.line + 1,
                    position.column.grapheme_offset + 1
                )?;
                write!(f, "definition(s) for reference(s)")?;
                for symbol in symbols {
                    write!(f, " ‘{}’", symbol)?;
                }
                if !missing_lines.is_empty() {
                    write!(
                        f,
                        " missing expected on line(s) {}",
                        missing_lines.iter().map(|l| l + 1).format(", ")
                    )?;
                }
                if !unexpected_lines.is_empty() {
                    write!(
                        f,
                        " found unexpected on line(s) {}",
                        unexpected_lines.iter().map(|l| l + 1).format(", ")
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl Test {
    /// Run the test. It is the responsibility of the caller to ensure that
    /// the stack graph has been constructed for the test files before running
    /// the test.
    pub fn run(&mut self) -> TestResult {
        let mut result = TestResult::new();
        for file in &self.files {
            for assertion in &file.assertions {
                match assertion.run(&self.graph, &mut self.paths) {
                    Ok(_) => result.add_success(),
                    Err(e) => result.add_failure(self.from_error(e)),
                }
            }
        }
        result
    }

    /// Construct a TestFailure from an AssertionError.
    fn from_error(&self, err: AssertionError) -> TestFailure {
        match err {
            AssertionError::NoReferences { source } => TestFailure::NoReferences {
                path: self.path.clone(),
                position: source.position,
            },
            AssertionError::IncorrectDefinitions {
                source,
                symbols,
                missing_targets,
                unexpected_paths,
            } => TestFailure::IncorrectDefinitions {
                path: self.path.clone(),
                position: source.position,
                symbols: symbols
                    .iter()
                    .map(|s| self.graph[*s].to_string())
                    .sorted()
                    .dedup()
                    .collect(),
                missing_lines: missing_targets
                    .iter()
                    .map(|t| t.line)
                    .sorted()
                    .dedup()
                    .collect(),
                unexpected_lines: unexpected_paths
                    .iter()
                    .map(|p| self.graph.source_info(p.end_node).unwrap().span.start.line)
                    .sorted()
                    .dedup()
                    .collect(),
            },
        }
    }
}
