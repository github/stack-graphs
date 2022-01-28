// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;
use std::fmt;
use std::slice;

use lazy_static::lazy_static;
use lsp_positions::Offset;
use lsp_positions::Position;
use lsp_positions::PositionedSubstring;
use lsp_positions::SpanCalculator;
use regex::Regex;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;
use thiserror::Error;

lazy_static! {
    static ref ASSERTION_REGEX: Regex =
        Regex::new(r#"(\^)\s*defined:(\s*\d+(?:\s*,\s*\d+)*)?"#).unwrap();
    static ref LINE_NUMBER_REGEX: Regex = Regex::new(r#"\d+"#).unwrap();
}

/// An error that can occur while parsing and executing assertions
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Assertion cannot appear on first line")]
    AssertionOnFirstLine,
    #[error("Assertion on line {0} refers to missing column {1} on line {2}")]
    InvalidColumn(usize, usize, usize),
}

#[derive(Debug, Clone)]
pub struct Assertions {
    file: Handle<File>,
    values: Vec<Assertion>,
}

#[derive(Debug, Clone)]
pub enum Assertion {
    Defined(Position, LineNumbers),
}

impl Assertions {
    pub fn from_source(file: Handle<File>, source: &str) -> std::result::Result<Self, ParseError> {
        let mut result = Assertions {
            file,
            values: Vec::new(),
        };

        let source_length = Offset::string_length(source);
        let mut next_line_utf8_offset = 0;
        let mut next_line_number = 0;
        let mut current_line_span_calculator = SpanCalculator::new(source);
        let mut last_regular_line = None;
        let mut last_regular_line_number = None;
        let mut last_regular_line_span_calculator = SpanCalculator::new(source);
        while next_line_utf8_offset < source_length.utf8_offset {
            let current_line = PositionedSubstring::from_line(source, next_line_utf8_offset);
            let current_line_number = next_line_number;
            // FIXME Line bounds are without newline, therefore we add 1 (assuming a single `\n` newline).
            //       Should a line include its newline, should there be a method returning the full line including
            //       newline, or should lsp-positions provide a more convenient way to iterate over lines (in which
            //       case lines can be without newline, but the user does not have to think about that)?
            next_line_utf8_offset = current_line.utf8_bounds.end + 1;
            next_line_number += 1;

            let mut matches = ASSERTION_REGEX
                .captures_iter(current_line.content)
                .peekable();
            if matches.peek().is_none() {
                // regular source line
                last_regular_line = Some(current_line);
                last_regular_line_number = Some(current_line_number);
            } else {
                // assertion line
                let last_regular_line = last_regular_line
                    .as_ref()
                    .ok_or_else(|| ParseError::AssertionOnFirstLine)?;
                let last_regular_line_number = last_regular_line_number.unwrap();

                while let Some(m) = matches.next() {
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
                        return Err(ParseError::InvalidColumn(
                            current_line_number + 1,
                            column_grapheme_offset + 1,
                            last_regular_line_number + 1,
                        ));
                    }
                    let position = last_regular_line_span_calculator.for_line_and_grapheme(
                        last_regular_line_number,
                        last_regular_line.utf8_bounds.start,
                        column_grapheme_offset,
                    );

                    let mut line_numbers = LineNumbers::new();
                    for l in LINE_NUMBER_REGEX
                        .find_iter(line_numbers_match.map(|m| m.as_str()).unwrap_or(""))
                    {
                        line_numbers.insert(l.as_str().parse::<usize>().unwrap() - 1);
                    }

                    result
                        .values
                        .push(Assertion::Defined(position, line_numbers));
                }
            }
        }

        Ok(result)
    }

    pub fn count(&self) -> usize {
        self.values.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineNumbers(BTreeSet<usize>);

impl LineNumbers {
    fn new() -> Self {
        Self(BTreeSet::new())
    }

    fn insert(&mut self, line_number: usize) {
        self.0.insert(line_number);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for LineNumbers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for line_number in &self.0 {
            if first {
                first = false;
                write!(f, "{}", line_number + 1)?;
            } else {
                write!(f, ", {}", line_number + 1)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Results {
    pub values: Vec<Result>,
    pub success_count: usize,
    pub failure_count: usize,
}

#[derive(Debug)]
pub enum Result {
    Success,
    Failure(AssertError),
}

#[derive(Debug, Error)]
pub enum AssertError {
    #[error("{0}:{1}:{2}: no references found")]
    NoReferencesFound(String, usize, usize),
    #[error("{0}:{1}:{2}: expected {3}, but found {4}")]
    WrongDefinitions(String, usize, usize, String, String),
}

impl Results {
    fn add_success(&mut self) {
        self.values.push(Result::Success);
        self.success_count += 1;
    }

    fn add_failure(&mut self, reason: AssertError) {
        self.values.push(Result::Failure(reason));
        self.failure_count += 1;
    }

    pub fn success_count(&self) -> usize {
        self.success_count
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count
    }
}

impl IntoIterator for Results {
    type Item = Result;
    type IntoIter = std::vec::IntoIter<Result>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a Results {
    type Item = &'a Result;
    type IntoIter = slice::Iter<'a, Result>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl Assertions {
    pub fn run(&self, graph: &StackGraph, paths: &mut Paths) -> Results {
        let mut result = Results {
            values: Vec::new(),
            success_count: 0,
            failure_count: 0,
        };

        for assertion in &self.values {
            match assertion {
                Assertion::Defined(position, expected_line_numbers) => {
                    let references = graph
                        .nodes_for_file(self.file)
                        .filter(|n| {
                            graph[*n].is_reference()
                                && graph
                                    .source_info(*n)
                                    .map(|s| s.span.contains(position.clone()))
                                    .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();
                    if references.is_empty() {
                        result.add_failure(AssertError::NoReferencesFound(
                            graph[self.file].to_string(),
                            position.line + 1,
                            position.column.grapheme_offset + 1,
                        ));
                    } else {
                        let mut actual_line_numbers = LineNumbers::new();
                        paths.find_all_paths(graph, references.clone(), |g, _ps, p| {
                            if p.is_complete(g) {
                                let si = graph.source_info(p.end_node).unwrap();
                                actual_line_numbers.insert(si.span.start.line);
                            }
                        });
                        if *expected_line_numbers == actual_line_numbers {
                            result.add_success();
                        } else {
                            let reference = graph[references[0]].symbol().unwrap().display(&graph);
                            result.add_failure(AssertError::WrongDefinitions(
                                graph[self.file].to_string(),
                                position.line + 1,
                                position.column.grapheme_offset + 1,
                                if expected_line_numbers.is_empty() {
                                    format!("no definitions for '{}'", reference,)
                                } else {
                                    format!(
                                        "definitions for '{}' on lines {}",
                                        reference, *expected_line_numbers,
                                    )
                                },
                                if actual_line_numbers.is_empty() {
                                    format!("no definitions")
                                } else {
                                    format!("definitions on lines {}", actual_line_numbers,)
                                },
                            ));
                        }
                    }
                }
            }
        }
        result
    }
}
