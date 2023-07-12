# tree-sitter-stack-graphs

The `tree-sitter-stack-graphs` crate lets you create stack graphs using the
[tree-sitter][] grammar for a language.

[tree-sitter]: https://tree-sitter.github.io/

- [API documentation](https://docs.rs/tree-sitter-stack-graphs/)
- [Examples](https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/examples/)
- [Release notes](https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/CHANGELOG.md)

## Usage

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
tree-sitter-stack-graphs = "0.7"
```

Check out our [documentation](https://docs.rs/tree-sitter-stack-graphs/*/) for
more details on how to use this library.

## Command-line Program

The command-line program for `tree-sitter-stack-graphs` lets you do stack
graph based analysis and lookup from the command line.

Install the program using `cargo install` as follows:

``` sh
$ cargo install --features cli tree-sitter-stack-graphs
$ tree-sitter-stack-graphs --help
```

Alternatively, the program can be invoked via NPM as follows:

``` sh
$ npx tree-sitter-stack-graphs
```

## Getting Started

Starting a new project to develop stack graph definitions for your favourite language
is as easy as running the `init` command:

``` sh
$ tree-sitter-stack-graphs init PROJECT_DIR
```

Answer the questions to provide information about the language, the grammar dependency, and
the project and hit `Generate` to generate the new project. Check out `PROJECT_DIR/README.md`
to find out how to start developing.

Also check out our [examples][] for more details on how to work with the CLI.

[examples]: https://github.com/github/stack-graphs/blob/main/tree-sitter-stack-graphs/examples/

## Development

The project is written in Rust, and requires a recent version installed.
Rust can be installed and updated using [rustup][].

[rustup]: https://rustup.rs/

Build the project by running:

```
$ cargo build
```

Run the tests by running:

```
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

```
$ cargo fmt
```

## License

Licensed under either of

  - [Apache License, Version 2.0][apache] ([LICENSE-APACHE](LICENSE-APACHE))
  - [MIT license][mit] ([LICENSE-MIT](LICENSE-MIT))

at your option.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
[mit]: http://opensource.org/licenses/MIT
