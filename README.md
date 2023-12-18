# Reverse S3

A super simple reverse proxy for S3 written in Rust. Can
come in handy if you want to reverse proxy an S3 bucket
for an internal network website.

See example configuration in `.env`. This file is excluded from the Docker build so needs to be set server side.

The more popular option is NGinx with S3 backend but its
[Auth module](https://github.com/anomalizer/ngx_aws_auth) cannot
deal with `AWS_WEB_IDENTITY_TOKEN_FILE` as an auth mechanism and
that's what's needed if you run this on Kubernetes.

## Developing

Using `docker-compose up -d` will bootstrap localstack
to mimic S3, and will put files from `test_data` in that bucket.

The test data assumes a bucket prefix called "reports". This can be omitted.
