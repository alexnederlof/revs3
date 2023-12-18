# Reverse S3

A super simple reverse proxy for S3 written in Rust. Can
come in handy if you want to reverse proxy an S3 bucket
for an internal network website.

See example configuration in `.env`. This file is excluded from the Docker build so needs to be set server side.

## Developing

Using `docker-compose up -d` will bootstrap localstack
to mimic S3, and will put files from `test_data` in that bucket.

The test data assumes a bucket prefix called "reports". This can be omitted.
