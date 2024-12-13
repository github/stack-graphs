# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.4.0 -- 2024-12-13

- The `tree-sitter-stack-graphs` dependency is updated to version 0.10.

- The `tree-sitter-typescript` dependency is updated to version 0.23.2.

## v0.3.0 -- 2024-07-09

### Added

- Support for TSX. A new language configuration for TSX is available with `{try_,}language_configuration_tsx`. TSX is enabled in the CLI next to TypeScript.

### Fixed

- Imports are more robust to the presence of file extensions in the import name.

### Changed

- The functions `{try_,}language_configuration` have been renamed to `{try_,}language_configuration_typescript`.

### Removed

- The `FILE_PATH_VAR` constant has been replaced in favor of `tree_sitter_stack_graphs::FILE_PATH_VAR`.

## v0.2.0 -- 2024-03-06

The `tree-sitter-stack-graphs` is updated to `v0.8`.

### Added

- An experimental VSCode LSP plugin that supports code navigation based on the stack graph rules. _Purely an experiment, not ready for serious use!_ Requires the `lsp` feature to be enabled.

### Changed

- Various improvements to the rules for imports and packages.

## v0.1.0 -- 2023-01-27

### Added

- Stack graph rules, tests, and basic `tsconfig.json` and `package.json` analysis.
