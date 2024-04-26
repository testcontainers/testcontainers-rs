# Contributing

First, thank you for contributing to `testcontainers-rs`.

## Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Code Contributions

### Setting up local development

- Ensure you have an [up-to-date Rust toolchain](https://rustup.rs/), with `clippy` and `rustfmt` components installed
- Install the [cargo-hack](https://github.com/taiki-e/cargo-hack) subcommand (recommended)
- Fork this repository

### Formatting

We rely on `rustfmt` (`nightly`):
```shell
cargo +nightly fmt - - all
```

### Commits

Strive for creating atomic commits.
That is, commits should capture a single feature, including its tests.
Ideally, each commits passes all CI checks (builds, tests, formatting, linter, ...).

When in doubt, squashing everything into a single but larger commit is preferable over commits that don't compile or are otherwise incomplete.

For writing good commit messages, you may find [this](https://chris.beams.io/posts/git-commit/) guide helpful.

## Contact

Feel free to drop by in the [testcontainers-rust channel](https://testcontainers.slack.com/archives/C048EPGRCER) of our [Slack workspace](https://testcontainers.slack.com).
