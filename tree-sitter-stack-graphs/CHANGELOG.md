# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.9.0 -- 2024-07-09


### Library

- New crate-level constants `FILE_PATH_VAR` and `ROOT_PATH_VAR` standardize the TSG global variable names to use for the file and root path.
- The file path variable will only use the filename set in the stack graph if no value was explicitly set.

### CLI

#### Added

- Tests run faster for languages with builtins sources by caching the partial paths for the builtins.
- Indexing will set a value for the root path variable that is passed to TSG. The value is based on the directory that was provided on the command line.

#### Changed

- Failure to index a file will not abort indexing anymore, but simply mark the file as failed, as we already do for files with parse errors.

#### Removed

- The NPM distribution has been deprecated.

## v0.8.1 -- 2024-03-06

The `stack-graphs` dependency was updated to `v0.13` to fix the build problems of the `v0.8.0` release.

## v0.8.0 -- 2024-03-05

The `tree-sitter` dependency version was updated to fix install problems.

### Library

#### Changed

- A new `Reporter` trait is used to support reporting status from CLI actions such as indexing and testing. The CLI actions have been cleaned up to ensure that they are not writing directly to the console anymore, but only call the reporter for output. The `Reporter` trait replaces the old inaccessible `Logger` trait so that clients can more easily implement their own reporters if necessary. A `ConsoleLogger` is provided for clients who just need console printing.

## v0.7.1 -- 2023-07-27

Support `stack-graphs` version `0.12`.

## v0.7.0 -- 2023-06-08

### Library

#### Added

- A new `CancelAfterDuration` implementation of `CancellationFlag` that cancels the computation after a certain amount of time.
- Tests can use new `defines` and `refers` assertions to check that a definition or reference with the give name exists at the assertion's source location.

#### Changed

- The `LanguageConfiguration::matches_file` method takes a `ContentProvider` instead of an `Option<&str>` value. This allows lazy file reading *after* the filename is checked, instead of the unconditional loading required before. To give content readers the opportunity to cache read values, a mutable reference is required. The return type has changed to `std::io::Result` to propagate possible errors from content providers. A `FileReader` implementation that caches the last read file is provided as well.
- Tests run with the CI `Tester` timeout after 60 seconds by default. Set `Tester::max_test_time` to change this behavior.
- A new `StackGraphLanguage::from_source` function can be used to construct a stack graph language from a given TSG source. The `StackGraphLanguage` type can also record the TSG file path, which is used when displaying errors.
- The `LoadError` type has been renamed to `BuildError` to avoid confusion between it and the `loader::LoadError` type.
- Cancellation flags support the `|` (or) operator to allow easy composition.
- The `LanguageConfiguration::from_tsg_str` method has been renamed to `LanguageConfiguration::from_sources`, and additionally accepts path parameters which are used for error message display.
- The `loader::LoadError` type now has a lifetime parameter and supports pretty error display.
- The loaders return a `FileLanguageConfiguration` value instead of a `StackGraphLanguage`, which contains both the primary `StackGraphLanguage` as well as any file analyzers for other secondary languages.

#### Fixed

- Fix a panic condition when assertion refer to source lines beyond the end of the test file.

### CLI

#### Added

- A new `analyze` command that computes stack graphs and partial paths for all given source files and directories and stores results in a database. The command does not produce any output at the moment. Analysis per file can be limited using the `--max-file-time` flag.
- A new `query` command can be used to resolve references using the analysis database.
- A new `status` command shows the status of files in the analysis database. The status includes whether the file was analyzed or not, and if the analysis was successful.
- A new `clean` command can be used to clean the analysis database, either completely or for specific paths.
- A new `match` command executes the query patterns from the TSG source and outputs the matches with captured nodes to the console. The `--stanza/-S` flag can be used to select specific stanzas to match by giving the line number where the stanza appears in the source. (Any line that is part of the stanza will work.)
- A new `visualize` command generates HTML visualizations based on the analysis database. Note that visualizations do not scale well, so this should only be used on small and few files.
- A new `lsp` command implements a basic LSP server that can be used in e.g. a VS Code plugin. Note that the implementation is not optimized and currently rather slow.
- The `init` command was updated and supports a `--internal` flag to easily generate language projects that are meant to be part of the projects repository.

#### Changed

- The `--show-ignored` flag of the `test` command has been renamed to `--show-skipped`. Only explicitly skipped test files (with a `.skip` extension) will be shown. Other unsupported files are, such as generated HTML files, are never shown.
- The `--hide-passing` flag of the `test` command has been renamed to the more common `--quiet`/`-q`.
- The output of the `test` command has changed to print the test name before the test result, so that it clear which test is currently running.
- The output of the `test` and `analyze` commands has changed in debug builds to include the run time per file.
- The `--hide-failure-errors` has been renamed to the more general `--hide-error-details`. The new flag is supported by the `test` and `analyze` commands.
- The files in directory arguments are now processed in filename order.
- The `test` command also skips directories with a `.skip` extension, not just files.

## v0.6.0 -- 2023-01-13

### Library

#### Changed

- The `cli` module has been reorganized. Instead of providing a fully derived CLI type, the subcommands are now exposed, while deriving the CLI type is left to the user. This makes sure that the name and version reported by the version command are ones from the user crate, and not from this crate. Instead of using `cli::LanguageConfigurationsCli` and `cli::PathLoadingCli`, users should using from `cli::provided_languages::Subcommands` and `cli::path_loading::Subcommands` respectively. The `cli` module documentation shows complete examples of how to do this.
- The `cli::CiTester` type has been moved to `ci::Tester`. Because it uses `cli` code internally, it is still hidden behind the `cli` feature flag.

#### Fixed

- Fix issue with test directives in languages that do not support comments.

## v0.5.1 -- 2023-01-10

Patch release to update *all* version numbers.

## v0.5.0 -- 2023-01-10

### Library

#### Added

- A new `cli` module contains the CLI implementation. It can be reused to create language-specific CLIs that do not rely on loading from the file system.
- An `empty_source_span` attribute can be used in TSG rules to collapse the source span to its start, instead of covering the whole source node.
- A new `FileAnalyzer` trait can be implemented to implement custom analysis of special project files such as package manifests or project configurations.

#### Changed

- Language loading has been redesigned to have clearer responsiilities for the various types involved. Loaders now return instances of `LanguageConfiguration`, which holds not just the `StackGraphLanguage` necessary to execute the TSG, but also other data about the language, such as file types, special file analyzers, and the builtins stack graph. The `StackGraphLanguage` is now only responsible for executing TSGs, and does not contain the language's `builtins` anymore.

#### Fixed

- A bug in path normalization that would lose `..` prefixes for paths whose normal form starts with `..` components.

### CLI

## 0.4.1 -- 2022-10-19

### CLI

#### Changed

- The default values for the `init` commmand changed to match naming conventions.
- After `init` read all user input, it presents an overview of selected settings and asks for user confirmation before creating any files.

### Fixed

- Several issues with content or location of files generated by `init` command.

## v0.4.0 -- 2022-10-19

Depend on `tree-sitter-graph` version 0.7.

### CLI

#### Added

- Builtins can be explicitly supplied using the `--builtins` flag. If the given path does not have a file extension, the file extension of the language is implicitly added.
- A new `init` command can be used to generate new tree-sitter-stack-graphs projects for NPM distributed Tree-sitter grammars.
- The syntax tree printed by `parse` shows one-based node positions.

#### Changed

- The path supplied to `--tsg` may omit the file extension, in which case it is implicitly added.

### Library

#### Changed

- The `loader::Loader::from_*` functions now take two new arguments, a search path for the TSG file and a search path for builtins, instead of the `loader::Reader`. The search paths are specified as a vector of `Loader::LoadPath`s, which can be either regular paths, or paths relative to the grammar location.

#### Added

- Tests can specify global variables that are passed to the TSG rules using `--- global: NAME=VALUE ---` in comments.
- `loader::Loader` can read global variables for the builtins from an optional configuration file next to the builtins file. The configuration file should have the `.cfg` extension and the same name as the builtins file. API users can call `loader::Loader::load_globals_from_config_*` methods to read configuration files.

## 0.3.1 -- 2022-09-07

### Library

#### Fixed

- Cancellation errors from `tree_sitter_graph` execution are not wrapped
  but propagated as cancellations.
- Executing a `StackGraphLanguage` for a `File` for which the `StackGraph`
  already contains nodes does not crash anymore.

#### Added

- The `StackGraphLanguage::builder_into_stack_graph` method can be used to
  create a `Builer` that allows injecting preexisting `StackGraph` nodes
  using `Builder::inject_node` to obtain `tree_sitter_graph::graph::Value`
  instances. These can be used for global variables such that the TSG
  rules can refer to stack graph nodes that were not created by them.

#### Changed

- Global `Variables` do not require a mutable reference anymore.

## 0.3.0 -- 2022-08-23

### Library

#### Changed

- `StackGraphLanguage` instances can now safely be shared between
  threads. The `StackGraphLanguage::build_stack_graph_into` does not
  require a mutable instance anymore.
- `StackGraphLanguage::build_stack_graph_into` and
  `loader::Loader::load_for_file` now supports cancellation by passing
  an instance of `CancellationFlag`. The `NoCancellation` type provides
  a noop implementation.
- `test::Test::run` now supports cancellation by passing an instance of
  `CancellationFlag` and returns a `Result` indicating whether the test
  was canceled or not.
- Depend on `stack-graphs` version 0.10 and `tree-sitter-graph` version 0.6.

## 0.2.0 -- 2022-06-29

Depend on `stack-graphs` version 0.9.

## 0.1.0 -- 2022-05-09

### Library

#### Added

- `StackGraphLanguage` data type that handles parsing sources, executing
  the tree-sitter-graph rules, and constructing the resulting stack graph.
- `test` module that defines data types for parsing and running stack
  graph tests.
- `loader` module that defines data types for loading stack graph
  languages for source files.

### CLI

#### Added

- `test` command that allows running stack graph tests.
