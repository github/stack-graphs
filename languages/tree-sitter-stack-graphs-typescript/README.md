# tree-sitter-stack-graphs definition for TypeScript

This project defines tree-sitter-stack-graphs rules for TypeScript using the [tree-sitter-typescript][] grammar.

[tree-sitter-typescript]: https://crates.io/crates/tree-sitter-typescript

## Usage

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
tree-sitter-stack-graphs-typescript = "0.1"
```

Check out our [documentation](https://docs.rs/tree-sitter-stack-graphs-typescript/*/) for
more details on how to use this library.

## Command-line Program

The command-line program for `tree-sitter-stack-graphs-typescript` lets you do
stack graph based analysis and lookup from the command line.

Install the program using `cargo install` as follows:

``` sh
$ cargo install --features cli tree-sitter-stack-graphs-typescript
$ tree-sitter-stack-graphs-typescript --help
```

## Development

The project is written in Rust, and requires a recent version installed.
Rust can be installed and updated using [rustup][].

[rustup]: https://rustup.rs/


The project is organized as follows:

- The stack graph rules are defined in `src/stack-graphs.tsg`.
- Builtins sources and configuration are defined in `src/builtins.ts` and `builtins.cfg` respectively.
- Tests are put into the `test` directory.

Build the project by running:

``` sh
$ cargo build
```

Run the tests as follows:

``` sh
$ cargo test
```

The project consists of a library and a CLI.
By default, running `cargo` only applies to the library.
To run `cargo` commands on the CLI as well, add `--features cli` or `--all-features`.

Run the CLI from source as follows:

``` sh
$ cargo run --features cli -- ARGS
```

Sources are formatted using the standard Rust formatted, which is applied by running:

``` sh
$ cargo fmt
```

The stack graph rules are written in [tree-sitter-graph][], which provides a VSCode
extension for syntax highlighting.

[tree-sitter-graph]: https://github.com/tree-sitter/tree-sitter-graph

Parse and test a single file by executing the following commands:

``` sh
$ cargo run -- parse FILES...
$ cargo run -- test TESTFILES...
```

Additional flags can be passed to these commands as well. For example, to generate
a visualization for the test, execute:

``` sh
$ cargo run -- test -V TESTFILES...
```

To generate the visualization regardless of test outcome, execute:

``` sh
$ cargo run -- test -V --output-mode=always TESTFILES...
```

Go to https://crates.io/crates/tree-sitter-stack-graphs for links to examples and documentation.
