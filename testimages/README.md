# testimages

This directory allows us to co-locate the source of the docker images (build contexts) used in tests
with the `testcontainers-rs` library itself.

This allows us to create small, lightweight images that test the specific scenarios we need and avoids
unnecessary external dependencies.

They can also contain their own crates, these need to be excluded from the workspace, however as they
are built within the containers.
