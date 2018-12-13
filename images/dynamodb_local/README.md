# DynamoDb Local 

This crate provides `DynamoDB` as an `Image` for `testcontainers`.

By default the `latest` will be used for the dynamodb-local container. This can be overridden by creating the DynamoDB
instance with `DynamoDB::with_tag_args`. 

The period of time waiting for the dynamodb-local image to be available can be adjusted from the default value of 
2000 milliseconds by setting the environment variable DYNAMODB_ADDITIONAL_SLEEP_PERIOD (in millis).

Information on the DynamoDB local container can be found at: https://hub.docker.com/r/amazon/dynamodb-local/
