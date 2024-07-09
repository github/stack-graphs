# stack-graphs

The `stack-graphs` crate provides a Rust implementation of _stack graphs_, which
allow you to define the name resolution rules for an arbitrary programming
language in a way that is efficient, incremental, and does not need to tap into
existing build or program analysis tools.

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
stack-graphs = "0.14"
```

Check out our [documentation](https://docs.rs/stack-graphs/) for more details on
how to use this library.

Notable changes for each version are documented in the [release notes](https://github.com/github/stack-graphs/blob/main/stack-graphs/CHANGELOG.md).

## Credits

Stack graphs are heavily based on the [_scope graphs_][scope graphs] framework
from Eelco Visser's group at TU Delft.

[scope graphs]: https://pl.ewi.tudelft.nl/research/projects/scope-graphs/

## License

Licensed under either of

  - [Apache License, Version 2.0][apache] ([LICENSE-APACHE](LICENSE-APACHE))
  - [MIT license][mit] ([LICENSE-MIT](LICENSE-MIT))

at your option.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
[mit]: http://opensource.org/licenses/MIT
