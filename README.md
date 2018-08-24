# Testcontainers

[![Build Status](https://travis-ci.com/coblox/testcontainers-rs.svg?branch=master)](https://travis-ci.com/coblox/testcontainers-rs)

Testcontainers is a Rust-library inspired by [http://testcontainers.org](http://testcontainers.org).

## Usage

Check [testcontainers/examples](./testcontainers/examples) on how to use the library.

## Structure

The repository is structured into the several crates.

- `testcontainers` contains the actual library and trait definitions
- The folder `images` contains several crates named after the respective docker image. Each crate adds support for one particular image. This allows to selectively import the images you need.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-Apache-2.0) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.