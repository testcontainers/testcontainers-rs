# Contributing

First, thank you for contributing to `testcontainers-rs`.

Most likely, your contribution is about adding support for a new docker image.
Docker images are contained within the [images](./src/images) folder.
Each file represents a docker image.
The files follow the name of the image on DockerHub.
Please follow this convention while developing new images.

## Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Git hooks

While developing, please make sure that your code is formatted using `cargo-fmt`.
You can easily do that by using the pre-defined `git` hooks:
```bash
git config core.hookspath .githooks
```
