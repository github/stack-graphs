# stack-graphs

The `stack-graphs` crate provides a Rust implementation of _stack graphs_, which
allow you to define the name resolution rules for an arbitrary programming
language in a way that is efficient, incremental, and does not need to tap into
existing build or program analysis tools.

## Credits

Stack graphs are heavily based on the [_scope graphs_][scope graphs] framework
from Eelco Visser's group at TU Delft.

[scope graphs]: https://pl.ewi.tudelft.nl/research/projects/scope-graphs/
