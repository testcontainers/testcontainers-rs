# Testcontainers

[![Build Status](https://travis-ci.com/coblox/testcontainers-rs.svg?branch=master)](https://travis-ci.com/coblox/testcontainers-rs)

Testcontainers is a Rust-library inspired by [http://testcontainers.org](http://testcontainers.org).

## Usage

Check [testcontainers/examples](./testcontainers/examples) on how to use the library.

## Structure

The repository is structured into the several crates.

- `testcontainers` contains the actual library and trait definitions
- The folder `images` contains several crates named after the respective docker image. Each crate adds support for one particular image. This allows to selectively import the images you need.
