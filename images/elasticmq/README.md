# ElasticMQ 

This crate provides `ElasticMQ` as an `Image` for `testcontainers`. The ElasticMQ docker image can be used for simulating Amazon SQS for local testing. 

By default the `0.14.6` will be used for the softwaremill/elasticmq container. This can be overridden by creating the ElasticMQ
instance with `ElasticMQ::with_tag`. 

Information on the softwaremill/elasticmq container can be found at: https://hub.docker.com/r/softwaremill/elasticmq
