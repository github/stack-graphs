# tree-sitter-stack-graphs

The `tree-sitter-stack-graphs` crate lets you create stack graphs using the [tree-sitter][] grammar for a language.

[tree-sitter]: https://tree-sitter.github.io/

- [API documentation](https://docs.rs/tree-sitter-stack-graphs/)
- [Examples](https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/examples/)
- [Release notes](https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/CHANGELOG.md)

## Using the API

To use this library, add the following to your `Cargo.toml`:

```toml
[dependencies]
tree-sitter-stack-graphs = "0.10"
```

Check out our [documentation](https://docs.rs/tree-sitter-stack-graphs/*/) for more details on how to use this library.

## Using the Command-line Program

The command-line program for `tree-sitter-stack-graphs` lets you do stack graph based analysis and lookup from the command line.

The CLI can be run as follows:

1. _(Installed)_ Install the CLI using Cargo as follows:

   ```sh
   cargo install --features cli tree-sitter-stack-graphs
   ```

   After this, the CLI should be available as `tree-sitter-stack-graphs`.

2. _(From source)_ Instead of installing the CLI, it can also be run directly from the crate directory, as a replacement for a `tree-sitter-stack-graphs` invocation, as follows:

   ```sh
   cargo run --features cli --
   ```

The basic CLI workflow for the command-line program is to index source code and issue queries against the resulting database:

1. Index a source folder as follows:

   ```sh
   tree-sitter-stack-graphs index SOURCE_DIR
   ```

   _Indexing will skip any files that have already be indexed. To force a re-index, add the `-f` flag._

   To check the status if a source folder, run:

   ```sh
   tree-sitter-stack-graphs status SOURCE_DIR
   ```

   To clean the database and start with a clean slate, run:

   ```sh
   tree-sitter-stack-graphs clean
   ```

   _Pass the `--delete` flag to not just empty the database, but also delete it. This is useful to resolve `unsupported database version` errors that may occur after a version update._

2. Run a query to find the definition(s) for a reference on a given line and column, run:

   ```sh
   tree-sitter-stack-graphs query definition SOURCE_PATH:LINE:COLUMN
   ```

   Resulting definitions are printed, including a source line if the source file is available.

Discover all available commands and flags by passing the `-h` flag to the CLI directly, or to any of the subcommands.

## Getting Started on a new Language

Starting a new project to develop stack graph definitions for your favourite language is as easy as running the `init` command:

```sh
tree-sitter-stack-graphs init PROJECT_DIR
```

Answer the questions to provide information about the language, the grammar dependency, and the project and hit `Generate` to generate the new project. Check out `PROJECT_DIR/README.md` to find out how to start developing.

Check out [examples][] of stack graph rules for typical language features.

[examples]: https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/examples/

## Development

The project is written in Rust, and requires a recent version installed.
Rust can be installed and updated using [rustup][].

[rustup]: https://rustup.rs/

Build the project by running:

```sh
cargo build
```

Run the tests by running:

```sh
cargo test
```

The project consists of a library and a CLI.
By default, running `cargo` only applies to the library.
To run `cargo` commands on the CLI as well, add `--features cli` or `--all-features`.

Run the CLI from source as follows:

```sh
cargo run --features cli -- ARGS
```

Sources are formatted using the standard Rust formatted, which is applied by running:

```sh
cargo fmt
```

## License

Licensed under either of

- [Apache License, Version 2.0][apache] ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license][mit] ([LICENSE-MIT](LICENSE-MIT))

at your option.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
[mit]: http://opensource.org/licenses/MIT
