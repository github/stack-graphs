# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.2.0 -- 2024-03-06

The `tree-sitter-stack-graphs` is updated to `v0.8`.

### Added

- An experimental VSCode LSP plugin that supports code navigation based on the stack graph rules. _Purely an experiment, not ready for serious use!_ Requires the `lsp` feature to be enabled.

### Changed

- Various improvements to the rules for imports and packages.

## v0.1.0 -- 2023-01-27

### Added

- Stack graph rules, tests, and basic `tsconfig.json` and `package.json` analysis.
