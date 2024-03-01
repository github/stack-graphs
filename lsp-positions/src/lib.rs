// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines LSP-compatible positioning information for source code.
//!
//! When writing a tool that analyzes or operates on source code, there's a good chance you need to
//! interoperate with the [Language Server Protocol][lsp].  This seemingly simple requirement makes
//! it surprisingly difficult to deal with _character locations_.  This is because Rust stores
//! Unicode string content (i.e., the source code you're analyzing) in UTF-8, while LSP specifies
//! character locations using [_UTF-16 code units_][lsp-utf16].
//!
//! For some background, Unicode characters, or code points, are encoded as one or more code units.
//! In UTF-8 a code unit is 1 byte, and a character is encoded in 1–4 code units (1–4 bytes).  In
//! UTF-16 a code unit is 2 bytes, and characters are encoded in 1–2 code units (2 or 4 bytes).
//! Rust strings are encoded as UTF-8, and indexed by byte (which is the same as by code unit).
//! Indices are only valid if they point to the first code unit of a code point.
//!
//! We keep track of each source code position using two units: the UTF-8 byte position within the
//! file or containing line, which can be used to index into UTF-8 encoded `str` and `[u8]` data,
//! and the UTF-16 code unit position within the line, which can be used to generate `Position`
//! values for LSP.
//!
//! [lsp]: https://microsoft.github.io/language-server-protocol/
//! [lsp-utf16]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocuments

use std::ops::Range;

use memchr::memchr;

use unicode_segmentation::UnicodeSegmentation as _;

#[cfg(feature = "lua")]
pub mod lua;

fn grapheme_len(string: &str) -> usize {
    string.graphemes(true).count()
}

fn utf16_len(string: &str) -> usize {
    string.chars().map(char::len_utf16).sum()
}

/// All of the position information that we have about a character in a source file
#[repr(C)]
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Position {
    /// The 0-indexed line number containing the character
    pub line: usize,
    /// The offset of the character within its containing line, expressed as both a UTF-8 byte
    /// index and a UTF-16 code unit index
    pub column: Offset,
    /// The UTF-8 byte indexes (within the file) of the start and end of the line containing the
    /// character
    pub containing_line: Range<usize>,
    /// The UTF-8 byte indexes (within the file) of the start and end of the line containing the
    /// character, with any leading and trailing whitespace removed
    pub trimmed_line: Range<usize>,
}

impl Position {
    /// Returns a tree-sitter [`Point`][Point] for this position.
    ///
    /// [Point]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Point.html
    #[cfg(feature = "tree-sitter")]
    pub fn as_point(&self) -> tree_sitter::Point {
        tree_sitter::Point {
            row: self.line,
            column: self.column.utf8_offset,
        }
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Position) -> std::cmp::Ordering {
        self.line
            .cmp(&other.line)
            .then_with(|| self.column.utf8_offset.cmp(&other.column.utf8_offset))
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Position) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "tree-sitter")]
impl PartialEq<tree_sitter::Point> for Position {
    fn eq(&self, other: &tree_sitter::Point) -> bool {
        self.line == other.row && self.column.utf8_offset == other.column
    }
}

#[cfg(feature = "tree-sitter")]
impl PartialOrd<tree_sitter::Point> for Position {
    fn partial_cmp(&self, other: &tree_sitter::Point) -> Option<std::cmp::Ordering> {
        Some(
            self.line
                .cmp(&other.row)
                .then_with(|| self.column.utf8_offset.cmp(&other.column)),
        )
    }
}

/// All of the position information that we have about a range of content in a source file
#[repr(C)]
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn contains(&self, position: &Position) -> bool {
        &self.start <= position && &self.end > position
    }

    #[cfg(feature = "tree-sitter")]
    pub fn contains_point(&self, point: &tree_sitter::Point) -> bool {
        &self.start <= point && &self.end > point
    }
}

impl Ord for Span {
    fn cmp(&self, other: &Span) -> std::cmp::Ordering {
        self.start
            .cmp(&other.start)
            .then_with(|| self.end.cmp(&other.end))
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Span) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The offset of a character within a string (typically a line of source code), using several
/// different units
///
/// All offsets are 0-indexed.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Offset {
    /// The number of UTF-8-encoded bytes appearing before this character in the string
    pub utf8_offset: usize,
    /// The number of UTF-16 code units appearing before this character in the string
    pub utf16_offset: usize,
    /// The number of graphemes appearing before this character in the string
    pub grapheme_offset: usize,
}

impl Offset {
    /// Calculates the length of a string, expressed as the position of the non-existent character
    /// after the end of the line.
    pub fn string_length(string: &str) -> Offset {
        Offset {
            utf8_offset: string.len(),
            utf16_offset: utf16_len(string),
            grapheme_offset: grapheme_len(string),
        }
    }

    /// Calculates the offset of each character within a string.  Typically the string will contain
    /// a single line of text, in which case the results are column offsets.  (In this case, the
    /// string should not contain any newlines, though we don't verify this.)
    ///
    /// Each character's offset is returned both as the byte offset from the beginning of the line,
    /// as well as the number of UTF-16 code units before the character in the line.  (This is the
    /// column unit used by the [Language Server Protocol][lsp-utf16].)
    ///
    /// The result is an iterator of offsets, one for each character.  The results will be sorted,
    /// so you can collect them into a `Vec` and use `binary_search_by_key` to look for particular
    /// characters.
    ///
    /// [lsp-utf16]: https://microsoft.github.io/language-server-protocol/specification#textDocuments
    pub fn all_chars(line: &str) -> impl Iterator<Item = Offset> + '_ {
        let mut grapheme_utf8_offsets = line
            .grapheme_indices(true)
            .map(|(utf8_offset, cluster)| Range {
                start: utf8_offset,
                end: utf8_offset + cluster.len(),
            })
            .peekable();
        // We want the output to include an entry for the end of the string — i.e., for the byte
        // offset immediately after the last character of the string.  To do this, we add a dummy
        // character to list of actual characters from the string.
        line.chars()
            .chain(std::iter::once(' '))
            .scan(Offset::default(), move |offset, ch| {
                let result = Some(*offset);
                // If there is no next grapheme, we assume it is the extra ' ' that was chained
                if grapheme_utf8_offsets
                    .peek()
                    .map(|r| r.start == offset.utf8_offset)
                    .unwrap_or(true)
                {
                    grapheme_utf8_offsets.next();
                    offset.grapheme_offset += 1;
                }
                offset.utf8_offset += ch.len_utf8();
                offset.utf16_offset += ch.len_utf16();
                result
            })
    }
}

/// A substring and information about where that substring occurs in a larger string.  (Most often,
/// this is a “line” and information about where that line occurs within a “file”.)
#[derive(Clone)]
pub struct PositionedSubstring<'a> {
    /// The content of the substring
    pub content: &'a str,
    /// The UTF-8 byte offsets of the beginning and end of the substring within the larger string
    pub utf8_bounds: Range<usize>,
    /// The number of UTF-16 code units in the substring
    pub utf16_length: usize,
    /// The number of graphemes in the substring
    pub grapheme_length: usize,
}

impl<'a> PositionedSubstring<'a> {
    /// Constructs a new positioned substring.  You must provide the larger string, and the byte
    /// range of the desired substring.
    pub fn from_range(string: &'a str, utf8_bounds: Range<usize>) -> PositionedSubstring<'a> {
        let substring = &string[utf8_bounds.clone()];
        PositionedSubstring {
            content: substring,
            utf8_bounds,
            utf16_length: utf16_len(substring),
            grapheme_length: grapheme_len(substring),
        }
    }

    /// Constructs a new positioned substring for a newline-terminated line within a file.  You
    /// provide the byte offset of the start of the line, and we automatically find the end of the
    /// line.
    pub fn from_line(string: &'a str, line_utf8_offset: usize) -> PositionedSubstring<'a> {
        // The line's byte index lets us trim all preceding lines in the file.
        let line_plus_others = &string[line_utf8_offset..];

        // The requested line stops at the first newline, or at the end of the file if there aren't
        // any newlines.
        let line = match memchr(b'\n', line_plus_others.as_bytes()) {
            Some(newline_offset) => &line_plus_others[..newline_offset],
            None => line_plus_others,
        };

        let length = Offset::string_length(line);
        PositionedSubstring {
            content: line,
            utf8_bounds: Range {
                start: line_utf8_offset,
                end: line_utf8_offset + length.utf8_offset,
            },
            utf16_length: length.utf16_offset,
            grapheme_length: length.grapheme_offset,
        }
    }

    // Returns an iterator over the lines of the given string.
    pub fn lines_iter(string: &'a str) -> impl Iterator<Item = PositionedSubstring<'a>> + 'a {
        let mut next_utf8_offset = 0;
        std::iter::from_fn(move || {
            if string.len() <= next_utf8_offset {
                return None;
            }
            let next = PositionedSubstring::from_line(string, next_utf8_offset);
            next_utf8_offset = next.utf8_bounds.end + 1;
            Some(next)
        })
    }

    /// Trims ASCII whitespace from both ends of a substring.
    pub fn trim_whitespace(&mut self) {
        let leading_whitespace = self
            .content
            .bytes()
            .enumerate()
            .find(|(_, ch)| !(*ch as char).is_ascii_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(self.content.len());
        let left_whitespace = &self.content[0..leading_whitespace];
        let trimmed_left = &self.content[leading_whitespace..];

        let trailing_whitespace = trimmed_left
            .bytes()
            .enumerate()
            // Point at the last non-whitespace character
            .rfind(|(_, ch)| !(*ch as char).is_ascii_whitespace())
            // Point at the immediately following whitespace character.  Note we are only looking
            // for _ASCII_ whitespace, so we can assume that the last whitespace character that we
            // found is 1 byte long.
            .map(|(index, _)| index + 1)
            .unwrap_or(0);
        let trimmed = &trimmed_left[0..trailing_whitespace];
        let right_whitespace = &trimmed_left[trailing_whitespace..];

        self.content = trimmed;
        self.utf8_bounds.start += left_whitespace.len();
        self.utf8_bounds.end -= right_whitespace.len();
        self.utf16_length -= utf16_len(left_whitespace);
        self.utf16_length -= utf16_len(right_whitespace);
        self.grapheme_length -= grapheme_len(left_whitespace);
        self.grapheme_length -= grapheme_len(right_whitespace);
    }
}

/// Automates the construction of [`Span`][] instances for content within a string.
pub struct SpanCalculator<'a> {
    string: &'a str,
    containing_line: Option<PositionedSubstring<'a>>,
    trimmed_line: Option<PositionedSubstring<'a>>,
    columns: Vec<Offset>,
}

// Note that each time you calculate the position of a node on a _different line_, we have to
// calculate some information about line.  You'd think that would mean it would be most efficient
// to use this type if you made to sure group all of your nodes by their rows before asking for us
// to create Spans for them.  However, it turns out that sorting your nodes to make sure that
// they're in row order is just as much work as recalculating the UTF16 column offsets if we ever
// revisit a line!

impl<'a> SpanCalculator<'a> {
    /// Creates a new span calculator for locations within the given string.
    pub fn new(string: &'a str) -> SpanCalculator<'a> {
        SpanCalculator {
            string,
            containing_line: None,
            trimmed_line: None,
            columns: Vec::new(),
        }
    }

    /// Constructs a [`Position`][] instance for a particular line and column in the string.
    /// You must provide the 0-indexed line number, the byte offset of the line within the string,
    /// and the UTF-8 byte offset of the character within the line.
    pub fn for_line_and_column(
        &mut self,
        line: usize,
        line_utf8_offset: usize,
        column_utf8_offset: usize,
    ) -> Position {
        self.replace_current_line(line_utf8_offset);
        Position {
            line: line,
            column: *self.for_utf8_offset(column_utf8_offset),
            containing_line: self.containing_line.as_ref().unwrap().utf8_bounds.clone(),
            trimmed_line: self.trimmed_line.as_ref().unwrap().utf8_bounds.clone(),
        }
    }

    /// Constructs a [`Span`][] instance for a tree-sitter node.
    #[cfg(feature = "tree-sitter")]
    pub fn for_node(&mut self, node: &tree_sitter::Node) -> Span {
        let start = self.position_for_node(node.start_byte(), node.start_position());
        let end = self.position_for_node(node.end_byte(), node.end_position());
        Span { start, end }
    }

    /// Constructs a [`Position`][] instance for a tree-sitter location.
    #[cfg(feature = "tree-sitter")]
    pub fn position_for_node(
        &mut self,
        byte_offset: usize,
        position: tree_sitter::Point,
    ) -> Position {
        // Since we know the byte offset of the node within the file, and of the node within the
        // line, subtracting gives us the offset of the line within the file.
        let line_utf8_offset = byte_offset - position.column;
        self.for_line_and_column(position.row, line_utf8_offset, position.column)
    }

    /// Constructs a [`Position`][] instance for a particular line and column in the string.
    /// You must provide the 0-indexed line number, the byte offset of the line within the string,
    /// and the grapheme offset of the character within the line.
    pub fn for_line_and_grapheme(
        &mut self,
        line: usize,
        line_utf8_offset: usize,
        column_grapheme_offset: usize,
    ) -> Position {
        self.replace_current_line(line_utf8_offset);
        Position {
            line: line,
            column: *self.for_grapheme_offset(column_grapheme_offset),
            containing_line: self.containing_line.as_ref().unwrap().utf8_bounds.clone(),
            trimmed_line: self.trimmed_line.as_ref().unwrap().utf8_bounds.clone(),
        }
    }

    /// Updates our internal state to represent the information about the line that starts at a
    /// particular byte offset within the file.
    fn replace_current_line(&mut self, line_utf8_offset: usize) {
        if let Some(containing_line) = &self.containing_line {
            if containing_line.utf8_bounds.start == line_utf8_offset {
                return;
            }
        }
        let line = PositionedSubstring::from_line(self.string, line_utf8_offset);
        self.columns.clear();
        self.columns.extend(Offset::all_chars(line.content));
        let mut trimmed = line.clone();
        trimmed.trim_whitespace();
        self.containing_line = Some(line);
        self.trimmed_line = Some(trimmed);
    }

    /// Returns the offset of the character at a particular UTF-8 offset in the line.
    /// Assumes that you've already called `replace_current_line` for the containing line.
    fn for_utf8_offset(&self, utf8_offset: usize) -> &Offset {
        let index = self
            .columns
            .binary_search_by_key(&utf8_offset, |pos| pos.utf8_offset)
            .unwrap();
        &self.columns[index]
    }

    /// Returns the offset of the character at a particular grapheme offset in the line.
    /// Assumes that you've already called `replace_current_line` for the containing line.
    fn for_grapheme_offset(&self, grapheme_offset: usize) -> &Offset {
        let mut index = self
            .columns
            .binary_search_by_key(&grapheme_offset, |pos| pos.grapheme_offset)
            .unwrap();
        // make sure to return the first offset for this grapheme
        let mut offset = &self.columns[index];
        while index > 0 {
            index -= 1;
            let prev_offset = &self.columns[index];
            if prev_offset.grapheme_offset != offset.grapheme_offset {
                break;
            }
            offset = prev_offset;
        }
        offset
    }
}
