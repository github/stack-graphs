# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0 -- 2022-08-23

### Library

#### Changed

- `StackGraphLanguage` instances can now safely be shared between threads. The `StackGraphLanguage::build_stack_graph_into` does not require a mutable instance anymore.
- `StackGraphLanguage::build_stack_graph_into` and `loader::Loader::load_for_file` now supports cancellation by passing an instance of `CancellationFlag`. The `NoCancellation` type provides a noop implementation.
- `test::Test::run` now supports cancellation by passing an instance of `CancellationFlag` and returns a `Result` indicating whether the test was canceled or not.
- Depend on `stack-graphs` version 0.10 and `tree-sitter-graph` version 0.6.

## 0.2.0 -- 2022-06-29

Depend on `stack-graphs` version 0.9.

## 0.1.0 -- 2022-05-09

### Library

#### Added

- `StackGraphLanguage` data type that handles parsing sources, executing the tree-sitter-graph rules, and constructing the resulting stack graph.
- `test` module that defines data types for parsing and running stack graph tests.
- `loader` module that defines data types for loading stack graph languages for source files.

### CLI

#### Added

- `test` command that allows running stack graph tests.
