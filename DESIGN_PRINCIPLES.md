# Design principles

This documents aims to capture the design principles that went into this library.
It should serve as a reference point when making decisions on what features to include or exclude.

## Simple

One of most important goals that we want to adhere to is creating a _simple_ API.
Overall, this means keeping the API as small as possible to get the task done.
When in doubt, we'd rather not add flags or configuration options for certain use cases.

Tests should be easier to write, easy to understand and easy to maintain.
`testcontainers` aims to support this as much as possible.
Having too many configuration options makes it harder for users to achieve this goal.

Another advantage of a small, public API is that we have to make fewer breaking changes.
This makes upgrades easier for our users.

## Reliable

Tests need to be reliable to provide value.
We strive to make `testcontainers` as reliable as possible and try to control as many aspects of the container to make sure they work consistently.

One consequence of this decision is that the container _tag_ is typically not configurable for images that ship with `testcontainers`.
Whilst an image behind a tag can also change, image authors tend to preserve compatibility there.
If we were to allow users to change the `tag` of an image, we wouldn't be able to guarantee that it works because we cannot test all combinations.

## Ease of use

The library should be easy to use.
For example, users should be able to make their own implementation of `Image` without much boilerplate.
In fact, the path forward may very well be that we stop shipping a lot of images in the crate and instead require users to create their own images.
