# Redis 

This crate provides `Redis` as an `Image` for `testcontainers`.

By default the `5.0` will be used for the redis container. This can be overridden by creating the Redis
instance with `Redis::with_tag`. 

Information on the Redis container can be found at: https://hub.docker.com/_/redis/
