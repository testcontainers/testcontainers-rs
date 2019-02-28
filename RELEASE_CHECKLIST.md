# Release checklist

1. Make sure you are on latest master
2. Create a new branch
3. Navigate to the crate you want to release
    1. `tc_core` goes first, if applicable
    2. `images`, and `clients` follow
    3. `testcontainers` meta crate goes last
4. Do `git diff <CRATE>-<VERSION>` for the latest released version, f.e. `git diff testcontainers-0.5.0`
5. Determine the degree of changes with respect to the [release guide](./RELEASING.md)
6. Bump the version accordingly
7. Do step 3. through all crates that need bumping
8. Make sure `cargo test --all` pass
9. Make one commit
10. Create PR, merge in master
11. Checkout latest master
12. Create a new tag based on your new version: `git tag <CRATE>-<NEW_VERSION>`, f.e. `git tag testcontainers-0.5.1`
13. Make sure `cargo test --all` and `cargo package` pass
14. Do `cargo publish`
