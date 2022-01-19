# tree-sitter-stack-graphs

The `tree-sitter-stack-graphs` crate lets you create stack graphs using the
[tree-sitter][] grammar for a language.

[tree-sitter]: https://tree-sitter.github.io/

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
tree-sitter-stack-graphs = "0.0"
```

Check out our [documentation](https://docs.rs/tree-sitter-stack-graphs/*/) for
more details on how to use this library.

## Command-line Program

The command-line program for `tree-sitter-stack-graphs` lets you do stack
graph based analysis and lookup from the command line.

To run from source, run the following command:

``` sh
cargo run --bin tree-sitter-stack-graphs --features cli -- ARGS
```

To install, run the following command:

``` sh
cargo install --path TARGETDIR --bin tree-sitter-stack-graphs --features cli
```

Run the program as follows to show the supported commands:

```sh
tree-sitter-stack-graphs --help
```

## License

Licensed under either of

  - [Apache License, Version 2.0][apache] ([LICENSE-APACHE](LICENSE-APACHE))
  - [MIT license][mit] ([LICENSE-MIT](LICENSE-MIT))

at your option.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
[mit]: http://opensource.org/licenses/MIT
