# Images

This folder contains crate that provide support for `testcontainers` for a particular docker image.

## Naming convention

The crates are named are after the underlying docker image, prefixed with `tc_` (for testcontainers).

For example:

The crate for the docker image `coblox/bitcoin-core` is named `tc_coblox_bitcoincore`. All dashes are removed and the organization and the image name are separated with an underscore.

## Adding a new image

1. Fork the repository
2. Create a new subfolder in `images/`
    1. Implement the `Image` trait for your docker image.
    2. Add necessary unit-tests for your `Image`. Check the [coblox_bitcoincore](coblox_bitcoincore/) image for an example.
3. Add your image as a dependency to the `testcontainers` meta-crate.
    1. Re-export all necessary items from your crate in the `images` module.
    2. Add a test to `tests/images.rs` for your image. Check the [README.md](../testcontainers/tests/README.md) for information on the testing strategy.
4. Submit a pull request!

After merging the PR, we will publish the crate on crates.io.
