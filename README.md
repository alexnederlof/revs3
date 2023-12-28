# Reverse S3

A super simple reverse proxy for S3 written in Rust. Can be used
as a web server with S3 as the backend.

This project was created because S3 Buckets cannot be configured
to function as a website on an internal network.
If they are configured as a website, it is always public. This
project aims to solve that problem.

The more popular option is NGinx with S3 backend but its
[Auth module](https://github.com/anomalizer/ngx_aws_auth) cannot
deal with `AWS_WEB_IDENTITY_TOKEN_FILE` as an auth mechanism and
that's what's needed if you run this on Kubernetes.

## Configuration

Besides the config below, you also need some form of AWS Authentication set through
environment variables. See
[the official docs](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/credentials.html)
for specifics

| Configuration Option | Description                                                           | Default | Required |
| -------------------- | --------------------------------------------------------------------- | ------- | -------- |
| AWS_REGION           | The AWS region where your S3 bucket is located.                       |         | Yes      |
| S3_BUCKET            | The name of your S3 bucket.                                           |         | Yes      |
| LOG_LEVEL            | The log level for the reverse proxy server.                           | info    | No       |
| KEY_PREFIX           | A prefix you want to use, in case your files are in a subfolder of S3 |         | No       |

See example configuration in `.env`. This file is excluded from the Docker build so needs to be set server side.

Besides this, the service needs a least the following permissions to be
able to function:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowReadAccessToBucket",
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:ListBucket"],
      "Resource": [
        "arn:aws:s3:::your-bucket-name/*",
        "arn:aws:s3:::your-bucket-name"
      ]
    }
  ]
}
```

## Caching

The server does no caching itself. It relies on the E-Tag headers to
enable browser caching. Further more, it will relay any expire headers
set by S3 to the browser.

## Developing

Using `docker-compose up -d` will bootstrap localstack
to mimic S3, and will put files from `test_data` in that bucket.

The test data assumes a bucket prefix called "reports". This can be omitted.
