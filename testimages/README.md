# testimages

This crate allows us to co-locate the source of the docker images used in tests with the
testcontainers-rs library itself. This allows us to create small, lightweight images that test the
specific scenarios we need and avoids unnecessary external dependencies.

## Adding New Images

Images are implemented in Rust. New images can be added by:

1. Creating a new binary in `src/bin/`
2. Creating a corresponding dockerfile in `src/dockerfiles/`. Ideally, these dockerfiles should
   build small, minimal images.
3. Finally, a new docker command should be added to `build.rs` to actually build the new image.

See the `no_expose_port` image as an example.
