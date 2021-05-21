# Contributing

First, thank you for contributing to `testcontainers-rs`.

Most likely, your contribution is about adding support for a new docker image.
Docker images are contained within the [images](./src/images) folder.
Each file represents a docker image.
The files follow the name of the image on DockerHub.
Please follow this convention while developing new images.

## Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Formatting

While developing, please make sure that your code is formatted using `dprint fmt`.
You can install `dprint` in various ways: https://dprint.dev/install/

## Commits

Strive for creating atomic commits.
That is, commits should capture a single feature, including its tests.
Ideally, each commits passes all CI checks (builds, tests, formatting, linter, ...).

When in doubt, squashing everything into a single but larger commit is preferable over commits that don't compile or are otherwise incomplete.

For writing good commit messages, you may find [this](https://chris.beams.io/posts/git-commit/) guide helpful.

## Git hooks

To ensure every commit is properly formatted, you can opt into pre-defined `git` hooks:

```bash
git config core.hookspath .githooks
```
