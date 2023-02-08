# tree-sitter-stack-graphs definition for Java

This project defines tree-sitter-stack-graphs rules for Java using the [tree-sitter-java](https://www.npmjs.com/package/tree-sitter-java) grammar.

## Local Development

The project is organized as follows:

- The stack graph rules are defined in `src/stack-graphs.tsg`.
- Tests are put into the `test` directory.

The following commands are intended to be run from the repo root.

Run all tests in the project by executing the following:

    cargo test -p tree-sitter-stack-graphs-java

Parse a single test file:
  `cargo run -p tree-sitter-stack-graphs-java -- parse OPTIONS FILENAME`

Test a single file:
  `cargo run -p tree-sitter-stack-graphs-java -- test OPTIONS FILENAME`

For debugging purposes, it is useful to run the visualization tool to generate graphs for the tests being run.

To run a test and generate the visualization:

`cargo run -p tree-sitter-stack-graphs-java -- test --output-mode=always -V=%r/%d/%n.html FILENAME`

Go to https://crates.io/crates/tree-sitter-stack-graphs for links to examples and documentation.
