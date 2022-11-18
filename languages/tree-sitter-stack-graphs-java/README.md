# tree-sitter-stack-graphs definition for Java

This project defines tree-sitter-stack-graphs rules for Java using the [tree-sitter-java](https://www.npmjs.com/package/tree-sitter-java) grammar.

## Development

The project is organized as follows:

- The stack graph rules are defined in `src/stack-graphs.tsg`.
- Tests are put into the `tests` directory.

Make sure all development dependencies are installed by running:
    ./bootstrap

Run all tests in the project by executing the following:

    cargo test

Parse and test a single file by executing the following commands:

    <!-- TODO: allow this option via cargo -->

Additional flags can be passed to these commands as well. For example, to generate a visualization for the test, execute:

    ./run -V

Go to https://crates.io/crates/tree-sitter-stack-graphs for links to examples and documentation.
