# Images

This folder contains crate that provide support for `testcontainers` for a particular docker image.

## Naming convention

The crates are named are after the underlying docker image, prefixed with `tc_` (for testcontainers).

For example:

The crate for the docker image `coblox/bitcoin-core` is named `tc_coblox_bitcoincore`. All dashes are removed and the organization and the image name are separated with an underscore.

## Adding a new image

- Fork the repository
- Create a new subfolder in `images/`
- Implement the `Image` trait for your docker image.
- Provide an example
- Submit a pull request

After merging the PR, we will publish the crate on crates.io.
