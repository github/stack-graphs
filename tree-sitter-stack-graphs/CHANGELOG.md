# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 -- 2022-05-09

### Library

#### Added

- `StackGraphLanguage` data type that handles parsing sources, executing the tree-sitter-graph rules, and constructing the resulting stack graph.
- `test` module that defines data types for parsing and running stack graph tests.
- `loader` module that defines data types for loading stack graph languages for source files.

### CLI

#### Added

- `test` command that allows running stack graph tests.
