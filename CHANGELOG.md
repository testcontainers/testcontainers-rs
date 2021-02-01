# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- How images express when a container is ready: Instead of implementing `wait_until_ready`, images now need to implement `ready_conditions` which returns a list of `WaitFor` instances.

### Removed

- `DYNAMODB_ADDITIONAL_SLEEP_PERIOD` variable from `dynamodb_local` image.
  Previously, we had a fallback of 2 seconds if this variable was not defined.
  We now wait for 2 seconds unconditionally after the specified message has been found.

## [0.12.0] - 2021-01-27

### Added

- Allow custom version for postgres image
- Remove `derivative` dependency
- `OrientDB` image
- `Zookeeper` image

### Changed

- Move port mapping logic to `RunArgs` instead of each Image.

## [0.11.0] - 2020-09-30

### Added

- `Docker::run_with_args` method. This allows naming a container and assigning it to a specific docker network. The docker
  network will be created if it doesn't exist yet. Once the client is dropped, the network will be removed again if it
  has previously been created. A network that already existed will not be removed.
- Address-type argument to `coblox/bitcoin-core` Image.
  We are setting `bech32` as a default here.
  This is different from the default of `bitcoind`.

### Fixed

- Block the thread until containers have been successfully removed.
  Previously, this was done in a fire-and-forget way and hence led to containers not being removed in certain situations.

### Changed

- MSRV is now 1.41.0

## [0.10.0] - 2020-08-20

### Added

- Mongo image.
- Support for the `fallbackfee` argument for the `bitcoin-core` image.
- Ability to customize the `entrypoint` used by the image.
- Ability to start a container once stopped.

### Changed

- MSRV bumped to 1.36 from 1.32.
- Change postgres image authentication POSTGRES_HOST_AUTH_METHOD rather than username and password.
- Bumped bitcoin-core default tag to 0.20.0.

## [0.9.1] - 2020-03-24

### Added

- A changelog
- Support volumes on containers

### Changed

- **Breaking**: `Container#get_host_port` now only accepts a `u16` instead of a `u32`.
  `u16` captures all possible port values.

### Fixes

- Provide a default password for the postgres image.
  There seems to be an unfortunate breaking change in the postgres image that we need to cater for.

[Unreleased]: https://github.com/testcontainers/testcontainers-rs/compare/0.12.0...HEAD
[0.12.0]: https://github.com/testcontainers/testcontainers-rs/compare/0.11.0...0.12.0
[0.11.0]: https://github.com/testcontainers/testcontainers-rs/compare/0.10.0...0.11.0
[0.10.0]: https://github.com/testcontainers/testcontainers-rs/compare/0.9.1...0.10.0
[0.9.1]: https://github.com/testcontainers/testcontainers-rs/compare/0.8.1...0.9.1
