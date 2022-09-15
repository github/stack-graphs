# Examples

Each directory contains a small example highlighting a particular name binding pattern.
The examples use Python syntax, but do not necessarily implement Python semantics.
Each directory contains a `stack-graphs.tsg` file that describes at the top what behavior it implements, and a `tests` directory showcasing the behavior.

Running the examples requires a working Tree-sitter installation with a Python grammar available. The tests for an example can be executed with the following commands in the example directory:

```bash
$ cargo run --features=cli -- test --tsg stack-graphs.tsg tests/
```

The following examples are available:

- [Nested scoping](nested-scoping/)
- [Sequential Definitions](sequential-definitions/)
