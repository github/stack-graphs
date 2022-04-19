# lsp-positions

The `lsp-positions` crate defines LSP-compatible positioning information for
source code.

When writing a tool that analyzes or operates on source code, there's a good
chance you need to interoperate with the [Language Server Protocol][lsp].  This
seemingly simple requirement makes it surprisingly difficult to deal with
_character locations_.  This is because Rust stores Unicode string content
(i.e., the source code you're analyzing) in UTF-8, while LSP specifies character
locations using [_UTF-16 code units_][lsp-utf16].

For some background, Unicode characters, or code points, are encoded as one or
more code units. In UTF-8 a code unit is 1 byte, and a character is encoded in
1–4 code units (1–4 bytes).  In UTF-16 a code unit is 2 bytes, and characters
are encoded in 1–2 code units (2 or 4 bytes). Rust strings are encoded as UTF-8,
and indexed by byte (which is the same as by code unit). Indices are only valid
if they point to the first code unit of a code point.

We keep track of each source code position using two units: the UTF-8 byte
position within the file or containing line, which can be used to index into
UTF-8 encoded `str` and `[u8]` data, and the UTF-16 code unit position within
the line, which can be used to generate `Position` values for LSP.

[lsp]: https://microsoft.github.io/language-server-protocol/
[lsp-utf16]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocuments

To use this library, add the following to your `Cargo.toml`:

``` toml
[dependencies]
lsp-positions = "0.3"
```

Check out our [documentation](https://docs.rs/lsp-positions/) for more details
on how to use this library.

Notable changes for each version are documented in the [release notes](https://github.com/github/stack-graphs/blob/main/lsp-positions/CHANGELOG.md).

## License

Licensed under either of

  - [Apache License, Version 2.0][apache] ([LICENSE-APACHE](LICENSE-APACHE))
  - [MIT license][mit] ([LICENSE-MIT](LICENSE-MIT))

at your option.

[apache]: http://www.apache.org/licenses/LICENSE-2.0
[mit]: http://opensource.org/licenses/MIT
