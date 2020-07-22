# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

-   MSRV bumped to 1.36 from 1.32.
-   Change postgres image authentication POSTGRES_HOST_AUTH_METHOD rather than username and password

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
