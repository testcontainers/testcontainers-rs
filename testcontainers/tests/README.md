# Testcontainers testing strategy

The purpose of these tests is two-fold:

1. Showcase how a particular image works.
    - How do you I connect to the container?
    - How to retrieve authentication information etc?
2. To ensure the dependencies are set correctly. As most users will only depend on the `testcontainers` crate, having these tests ensures that what works for us, works for them!
