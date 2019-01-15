# Testcontainers-rs

[![Build Status](https://travis-ci.org/testcontainers/testcontainers-rs.svg?branch=master)](https://travis-ci.org/testcontainers/testcontainers-rs)
[![Crates.io](https://img.shields.io/crates/v/testcontainers.svg)](https://crates.io/crates/testcontainers)
[![Docs.rs](https://docs.rs/testcontainers/badge.svg)](https://docs.rs/testcontainers)
[![Dependabot Status](https://api.dependabot.com/badges/status?host=github&repo=testcontainers/testcontainers-rs)](https://dependabot.com)

Testcontainers-rs is the official Rust language fork of [http://testcontainers.org](http://testcontainers.org).

## Usage

**TL:DR**: Depend on `testcontainers`, this will give you everything you need.

Check [the integration tests](./testcontainers/tests) on how to use the library.

## Structure

The repository is structured into the several crates.

- `core`: Contains the core traits and structs necessary for the testcontainers ecosystem.
- `cli_client`: Contains an implementation of the `Docker` trait that uses the `docker`-CLI to issue commands.
- The folder `images` contains several crates named after the respective docker image. 
Each crate adds support for one particular image. This allows to selectively import the images you need.

Last but not least:
`testcontainers`: This is a meta crate that bundles all these crates together for convenient usage.

## Versioning

To not make it too inconvenient, the meta crate `testcontainers` cannot strictly follow semver.
Thus you are encouraged to at least depend on the minor version of the library, i.e. `X.Y`. 
If you are interested in the versioning approach, chosen by this crate, checkout the [release guide](./RELEASING.md).

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-Apache-2.0) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Developing

While developing, please make sure that your code is formatted using `cargo-fmt`.
You can easily do that by using the pre-defined `git` hooks:
```bash
git config core.hookspath .githooks
```

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
