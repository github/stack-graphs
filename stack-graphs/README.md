# stack-graphs

The `stack-graphs` crate provides a Rust implementation of _stack graphs_, which
allow you to define the name resolution rules for an arbitrary programming
language in a way that is efficient, incremental, and does not need to tap into
existing build or program analysis tools.

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
stack-graphs = "0.12"
```

Check out our [documentation](https://docs.rs/stack-graphs/) for more details on
how to use this library.

Notable changes for each version are documented in the [release notes](https://github.com/github/stack-graphs/blob/main/stack-graphs/CHANGELOG.md).

## Lua bindings

This crate includes optional Lua bindings, allowing you to construct stack
graphs using Lua code.  Lua support is only enabled if you compile with the `lua`
feature.  This feature is not enough on its own, because the `mlua` crate
supports multiple Lua versions, and can either link against a system-installed
copy of Lua, or build its own copy from vendored Lua source.  These choices are
all controlled via additional features on the `mlua` crate.

When building and testing this crate, make sure to provide all necessary
features on the command line:

``` console
$ cargo test --features lua,mlua/lua54,mlua/vendored
```

When building a crate that depends on this crate, add a dependency on `mlua` so
that you can set its feature flags:

``` toml
[dependencies]
stack-graphs = { version="0.13", features=["lua"] }
mlua = { version="0.9", features=["lua54", "vendored"] }
```

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
