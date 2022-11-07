# tree-sitter-stack-graphs definition for TypeScript

This project defines tree-sitter-stack-graphs rules for TypeScript using the [tree-sitter-typescript](https://www.npmjs.com/package/tree-sitter-typescript) grammar.

## Development

The project is organized as follows:

- The stack graph rules are defined in `src/stack-graphs.tsg`.
- Builtins sources and configuration are defined in `src/builtins.ts` and `builtins.cfg` respectively.
- Tests are put into the `test` directory.

Make sure all development dependencies are installed by running:

    npm install

Run all tests in the project by executing the following:

    npm test

Parse and test a single file by executing the following commands:

    npm run parse-file test/test.ts
    npm run test-file test/test.ts

Additional flags can be passed to these commands as well. For example, to generate a visualization for the test, execute:

    npm run test-file -- -V test/test.ts

To generate the visualization regardless of test outcome, execute:

    npm run test-file -- -V --output-mode=always test/test.ts

These commands should be enough for regular development. If necessary, the `tree-sitter-stack-graphs` command can be invoked directly as well, by executing:

    npx tree-sitter-stack-graphs

Go to https://crates.io/crates/tree-sitter-stack-graphs for links to examples and documentation.
