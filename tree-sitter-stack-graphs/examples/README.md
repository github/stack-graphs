# Examples

Each directory contains a small example highlighting a particular name binding pattern.
The examples use Python syntax, but do not necessarily implement Python semantics.
Each directory contains a `stack-graphs.tsg` file that describes at the top what behavior it implements, and a `tests` directory showcasing the behavior.

Running the examples requires the Python grammar to be available. This can be installed (in this directory) by executing:

```bash
$ ./bootstrap
```

Run the tests for an example by executing:

```bash
$ ./run EXAMPLE_DIR
```

or, from within the example's directory:

```bash
$ ../run
```

To render HTML visualizations of the stack graphs for the tests in an example, add the `-V` flag to run.

The following examples are available:

- [Nested scoping](nested-scoping/)
- [Sequential definitions](sequential-definitions/)
- [Modules and imports](modules/)
