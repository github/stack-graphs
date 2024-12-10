# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.5.0 -- Unreleased

- The `tree-sitter-stack-graphs` dependency is updated to version 0.10.

- The `tree-sitter-java` dependency is updated to version 0.23.4.

## v0.4.0 -- 2024-07-09

### Added

- Add rules for the `condition` node that was missing.

### Removed

- The `FILE_PATH_VAR` constant has been replaced in favor of `tree_sitter_stack_graphs::FILE_PATH_VAR`.

## v0.3.0 -- 2024-03-06

The `tree-sitter-stack-graphs` is updated to `v0.8`.

### Changed

- The `cli` feature is now required to install the CLI.

## v0.2.0 -- 2023-03-21

### Added

- Definitions and references for labels (#194)
- Definitions and references for enum switch statements, interfaces, added additional rules for class extension (#210)
- Updated rules to support finding definitions of imported classes and methods (#192)

### Fixed

- Corrected issue where `for` loops with commas raised errors (#224)
- Added missing nodes for `line_comments` and `block_comments` (#192)
