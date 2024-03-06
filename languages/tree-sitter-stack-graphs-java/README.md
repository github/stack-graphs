# tree-sitter-stack-graphs definition for Java

This project defines tree-sitter-stack-graphs rules for Java using the [tree-sitter-java][] grammar.

[tree-sitter-java]: https://crates.io/crates/tree-sitter-java

- [API documentation](https://docs.rs/tree-sitter-stack-graphs-java/)
- [Release notes](https://github.com/github/stack-graphs/blob/main/languages/tree-sitter-stack-graphs-java/CHANGELOG.md)

## Using the API

To use this library, add the following to your `Cargo.toml`:

```toml
[dependencies]
tree-sitter-stack-graphs-java = "0.3"
```

Check out our [documentation](https://docs.rs/tree-sitter-stack-graphs-java/*/) for more details on how to use this library.

## Using the Command-line Program

The command-line program for `tree-sitter-stack-graphs-java` lets you do stack graph based analysis and lookup from the command line.

The CLI can be run as follows:

1. _(Installed)_ Install the CLI using Cargo as follows:

   ```sh
   cargo install --features cli tree-sitter-stack-graphs-java
   ```

   After this, the CLI should be available as `tree-sitter-stack-graphs-java`.

2. _(From source)_ Instead of installing the CLI, it can also be run directly from the crate directory, as a replacement for a `tree-sitter-stack-graphs-java` invocation, as follows:

   ```sh
   cargo run --features cli --
   ```

The basic CLI workflow for the command-line program is to index source code and issue queries against the resulting database:

1. Index a source folder as follows:

   ```sh
   tree-sitter-stack-graphs-java index SOURCE_DIR
   ```

   _Indexing will skip any files that have already be indexed. To force a re-index, add the `-f` flag._

   To check the status if a source folder, run:

   ```sh
   tree-sitter-stack-graphs-java status SOURCE_DIR
   ```

   To clean the database and start with a clean slate, run:

   ```sh
   tree-sitter-stack-graphs-java clean
   ```

   _Pass the `--delete` flag to not just empty the database, but also delete it. This is useful to resolve `unsupported database version` errors that may occur after a version update._

2. Run a query to find the definition(s) for a reference on a given line and column, run:

   ```sh
   tree-sitter-stack-graphs-java query definition SOURCE_PATH:LINE:COLUMN
   ```

   Resulting definitions are printed, including a source line if the source file is available.

Discover all available commands and flags by passing the `-h` flag to the CLI directly, or to any of the subcommands.

## Development

The project is written in Rust, and requires a recent version installed.  Rust can be installed and updated using [rustup][].

[rustup]: https://rustup.rs/

The project is organized as follows:

- The stack graph rules are defined in `src/stack-graphs.tsg`.
- Builtins sources and configuration are defined in `src/builtins.it` and `builtins.cfg` respectively.
- Tests are put into the `test` directory.

### Running Tests

Run the tests as follows:

```sh
cargo test
```

The project consists of a library and a CLI. By default, running `cargo` only applies to the library. To run `cargo` commands on the CLI as well, add `--features cli` or `--all-features`.

Run the CLI from source as follows:

```sh
cargo run --features cli -- ARGS
```

Sources are formatted using the standard Rust formatted, which is applied by running:

```sh
cargo fmt
```

### Writing TSG

The stack graph rules are written in [tree-sitter-graph][]. Checkout the [examples][],
which contain self-contained TSG rules for specific language features. A VSCode
[extension][] is available that provides syntax highlighting for TSG files.

[tree-sitter-graph]: https://github.com/tree-sitter/tree-sitter-graph
[examples]: https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/examples/
[extension]: https://marketplace.visualstudio.com/items?itemName=tree-sitter.tree-sitter-graph

Parse and test a single file by executing the following commands:

```sh
cargo run --features cli -- parse FILES...
cargo run --features cli -- test TESTFILES...
```

Generate a visualization to debug failing tests by passing the `-V` flag:

```sh
cargo run --features cli -- test -V TESTFILES...
```

To generate the visualization regardless of test outcome, execute:

```sh
cargo run --features cli -- test -V --output-mode=always TESTFILES...
```

Go to <https://crates.io/crates/tree-sitter-stack-graphs> for links to examples and documentation.
