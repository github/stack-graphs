# Changelog for tree-sitter-stack-graphs-python

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.3.0 -- 2024-12-12

- The `tree-sitter-stack-graphs` dependency is updated to version 0.10.

- The `tree-sitter-python` dependency is updated to version 0.23.5.

## v0.2.0 -- 2024-07-09

### Added

- Added support for root paths. This fixes import problems when indexing using absolute directory paths.

### Fixed

- Fixed crash for lambdas with parameters.
- Fixed crash for nested functions definitions.

### Removed

- The `FILE_PATH_VAR` constant has been replaced in favor of `tree_sitter_stack_graphs::FILE_PATH_VAR`.

## v0.1.0 -- 2024-03-06

Initial release.
