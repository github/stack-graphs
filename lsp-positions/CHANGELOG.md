# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0 - 2022-04-19

### Added

- `PositionedSubstring::lines_iter` function to iterate over the lines of a string.

### Changed

- `Span::contains` and `Span::contains_point` take a reference argument, instead of
  taking ownership.

## 0.2.0 - 2022-02-17

### Added

- Added a `grapheme_offset` field to the `Offset` type, to track grapheme
  positions along with UTF-8 and UTF-16 positions.
