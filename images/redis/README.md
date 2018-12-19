# Redis 

This crate provides `Redis` as an `Image` for `testcontainers`.

By default the `5.0` will be used for the dynamodb-local container. This can be overridden by creating the DynamoDB
instance with `DynamoDB::with_tag`. 

Information on the Redis container can be found at: https://hub.docker.com/_/redis/
