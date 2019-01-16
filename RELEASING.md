# Release guide

This document summaries important knowledge in releasing crates in the testcontainers-rs ecosystem.

## Core

The `tc_core` crate provides the main types like the `Image` trait or the `Container` struct.
All other crates depend on this. 
Any backwards-incompatible change must be appropriately reflected through versioning. 
Very likely, all downstream crates like concrete images and clients will require an equivalent version update.

## Images

Versioning the image crates can be quite tricky and the consequences are not obvious. 
When releasing a new version of an image crate, two aspects should be considered when choosing the next version:

- Compatibility of the Rust API
- Compatibility of the exposed Docker image

In regards to the Rust API of an Image (the `XYArgs` struct or the `XY` Image struct itself), the usual Rust rules apply. 
For example, removing or renaming a previously public field is a breaking change.

Changes to the used Docker image a bit trickier. 
It very much depends on what changed in the underlying Docker image. 
If there are backwards-incompatible changes in the network API (whatever the service inside the container speaks over the exposed ports), you should appropriately reflect this in the versioning. 

Let's take the `coblox/bitcoincore` image as an example.
The image contains the `bitcoind` binary, which has a JSON-RPC interface. 
Between version 0.16 and 0.17, breaking changes have been made to this API.
Changing the docker-tag that the `tc_coblox_bitcoincore` crate is referring to is thus a breaking change for this crate and should be appropriately reflected in the version number.  

## The meta crate

The meta-crate plays an interesting role in this ecosystem. 
Its main purpose is convenience by re-exporting a **valid** and **working** set of images and clients.

To achieve this, the version of `tc_core` needs to be compatible among all the dependencies (i.e. the image and client crates). 

The meta-crate doesn't have any code on its own, so you may wonder what does the version number actually represent.
It represents which images you can use, i.e. a set of features.
Because it is a meta-crate, reflecting all major version upgrades as a major version upgrade would result in an explosion of versions.

For this reason, breaking changes in downstream system **will not** be propagated as such to users of the meta crate.
This means a version upgrade of `tc_coblox_bitcoincore` from `1.0` to `2.0` **does not** result in a major version upgrade of the `testcontainers` meta crate.
Instead, `testcontainers` will only have its minor version bumped.
Because of that, users of `testcontainers` are encouraged to at least specify the minor version when depending on `testcontainers`.

## What about the problems this causes?

There are some potential problems with this approach.
Remember, the meta-crate only exists as a **convenience** for the majority of users.
If it doesn't work for you, you can always depend on the individual crates themselves and pick exactly the versions you want.
