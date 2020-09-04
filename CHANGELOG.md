# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

-   `Docker#run_with_args` method. This allows naming a container and assigning it to a specific docker network. The docker
network will be created if it doesn't exist yet. Once the client is dropped, the network will be removed again if it
has previously been created. A network that already existed will not be removed.

## [0.10.0] - 2020-08-20

### Added

-   Mongo image.
-   `run_with_options` method that allows running a docker container with custom options instead of the currently
    hardcoded ones.
-   Support for the `fallbackfee` argument for the `bitcoin-core` image.
-   Ability to customize the `entrypoint` used by the image.
-   Ability to start a container once stopped. 
 
### Changed

-   MSRV bumped to 1.36 from 1.32.
-   Change postgres image authentication POSTGRES_HOST_AUTH_METHOD rather than username and password.
-   Bumped bitcoin-core default tag to 0.20.0.

## [0.9.1] - 2020-03-24

### Added

-   A changelog
-   Support volumes on containers

### Changed

-   **Breaking**: `Container#get_host_port` now only accepts a `u16` instead of a `u32`.
`u16` captures all possible port values.

### Fixes

-   Provide a default password for the postgres image.
There seems to be an unfortunate breaking change in the postgres image that we need to cater for.

[Unreleased]: https://github.com/testcontainers/testcontainers-rs/compare/0.9.1...HEAD

[0.9.0]: https://github.com/testcontainers/testcontainers-rs/compare/0.8.1...0.9.1
