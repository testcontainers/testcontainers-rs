# Testcontainers-rs

![Continuous Integration](https://github.com/testcontainers/testcontainers-rs/workflows/Continuous%20Integration/badge.svg?branch=dev)
[![Crates.io](https://img.shields.io/crates/v/testcontainers.svg)](https://crates.io/crates/testcontainers)
[![Docs.rs](https://docs.rs/testcontainers/badge.svg)](https://docs.rs/testcontainers)
[![Dependabot Status](https://api.dependabot.com/badges/status?host=github&repo=testcontainers/testcontainers-rs)](https://dependabot.com)
[![Bors enabled](https://bors.tech/images/badge_small.svg)](https://app.bors.tech/repositories/20716)
[![Slack](https://img.shields.io/badge/Slack-join-orange?style=flat&logo=slack&)](https://join.slack.com/t/testcontainers/shared_invite/zt-2gra37tid-n9xDJGjjVb7hMRanGjowkw)

Testcontainers-rs is the official Rust language fork of [http://testcontainers.org](http://testcontainers.org).

## Usage

### `testcontainers` is the core crate

The crate provides an API for working with containers in a test environment.

1. Depend on `testcontainers`
2. Implement `testcontainers::core::Image` for necessary docker-images
3. Run it with any available client `testcontainers::clients::*`

### Ready-to-use images

The easiest way to use `testcontainers` is to depend on ready-to-use images (aka modules).

Modules are available as a community-maintained crate: [testcontainers-modules](https://github.com/testcontainers/testcontainers-rs-modules-community)

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-Apache-2.0) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
