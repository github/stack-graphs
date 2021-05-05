## Contributing

[fork]: https://github.com/github/stack-graphs/fork
[pr]: https://github.com/github/stack-graphs/compare
[code-of-conduct]: CODE_OF_CONDUCT.md
[`rustfmt`]: https://github.com/rust-lang/rustfmt

Hi there! We're thrilled that you'd like to contribute to this project. Your help is essential for keeping it great.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as described in the [project README](README.md), without any additional terms or conditions.

Please note that this project is released with a [Contributor Code of Conduct][code-of-conduct]. By participating in this project you agree to abide by its terms.

## Submitting a pull request

0. [Fork][fork] and clone the repository
0. Make sure the tests pass on your machine: `cargo test`
0. Create a new branch: `git checkout -b my-branch-name`
0. Make your change, add tests, and make sure the tests still pass
0. Push to your fork and [submit a pull request][pr]
0. Pat your self on the back and wait for your pull request to be reviewed and merged.

Here are a few things you can do that will increase the likelihood of your pull request being accepted:

- Use [`rustfmt`] to make sure your code automatically conforms to the standard Rust style guide.
- Write tests.
- Keep your change as focused as possible. If there are multiple changes you would like to make that are not dependent upon each other, consider submitting them as separate pull requests.
- Write a [good commit message](http://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html).

## Publishing a new version

If you are one of the maintainers of this package, bump the version numbers in [`Cargo.toml`](Cargo.toml) and [`README.md`](README.md), then follow the typical instructions to publish a new version to [crates.io][]:

```
$ cargo package
$ cargo publish
```

[crates.io]: https://crates.io/stack-graphs/

## Resources

- [How to Contribute to Open Source](https://opensource.guide/how-to-contribute/)
- [Using Pull Requests](https://help.github.com/articles/about-pull-requests/)
- [GitHub Help](https://help.github.com)

