# Release checklist

1. Make sure you are on latest master
2. Create a new branch
3. Navigate to the crate you want to release
    1. `tc_core` goes first, if applicable
    2. `images`, and `clients` follow
    3. `testcontainers` meta crate goes last
4. Do `git diff <CRATE>-<VERSION>` for the latest released version, f.e. `git diff testcontainers-0.5.0`
5. Determine the degree of changes with respect to the [release guide](./RELEASING.md)
6. Bump the version accordingly and make a commit
7. Create a new tag based on your new version: `git tag <CRATE>-<NEW_VERSION>`, f.e. `git tag testcontainers-0.5.1`
8. Make sure `cargo test` and `cargo package` pass
9. Do `cargo release`
