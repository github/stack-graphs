# stack-graphs

The `stack-graphs` crate provides a Rust implementation of _stack graphs_, which
allow you to define the name resolution rules for an arbitrary programming
language in a way that is efficient, incremental, and does not need to tap into
existing build or program analysis tools.

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
stack-graphs = "0.1"
```

Check out our [documentation](https://docs.rs/stack-graphs/) for more details on
how to use this library.

## How to contribute

We welcome your contributions!  Please see our [contribution
guidelines](CONTRIBUTING.md) and our [code of conduct](CODE_OF_CONDUCT.md) for
details on how to participate in our community.

## Credits

Stack graphs are heavily based on the [_scope graphs_][scope graphs] framework
from Eelco Visser's group at TU Delft.

[scope graphs]: https://pl.ewi.tudelft.nl/research/projects/scope-graphs/

## License

Licensed under the [Apache License, Version 2.0][apache].  See the
[COPYING](COPYING) file in this repo for more details.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
