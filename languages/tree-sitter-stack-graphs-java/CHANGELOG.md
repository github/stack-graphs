# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [0.2.0] - 2023-03-21

### Added

- Definitions and references for labels (#194)
- Definitions and references for enum switch statements, interfaces, added additional rules for class extension (#210)
- Updated rules to support finding definitions of imported classes and methods (#192)

### Fixed

- Corrected issue where `for` loops with commas raised errors (#224)
- Added missing nodes for `line_comments` and `block_comments` (#192)
