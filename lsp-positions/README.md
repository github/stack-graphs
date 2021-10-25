# lsp-positions

The `lsp-positions` crate defines LSP-compatible positioning information for
source code.

When writing a tool that analyzes or operates on source code, there's a good
chance you need to interoperate with the [Language Server Protocol][lsp].  This
seemingly simple requirement makes it surprisingly difficult to deal with
_character locations_.  This is because Rust wants to store Unicode string
content (i.e., the source code you're analyzing) in UTF-8, while LSP wants to
specify character locations using [_UTF-16 code points_][lsp-utf16].

That means that we ideally need to keep track of each source code position using
at least two units: the UTF-8 offset within the file or containing line (to make
it easy to index into UTF-8 encoded strings), as well as the UTF-16 code point
offset within the line (to make it possible to generate `Position` values for
LSP).

[lsp]: https://microsoft.github.io/language-server-protocol/
[lsp-utf16]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocuments

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
lsp-positions = "0.1"
```

Check out our [documentation](https://docs.rs/lsp-positions/) for more details
on how to use this library.

## License

Licensed under either of

  - [Apache License, Version 2.0][apache] ([LICENSE-APACHE](LICENSE-APACHE))
  - [MIT license][mit] ([LICENSE-MIT](LICENSE-MIT))

at your option.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
[mit]: http://opensource.org/licenses/MIT
