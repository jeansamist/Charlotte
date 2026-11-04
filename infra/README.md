# infra

> **Status: planned, not implemented.** Today Charlotte runs entirely on the Debian machine, installed by
> [`scripts/install.sh`](../scripts/README.md), with PostgreSQL in an existing Docker container.

| Folder | Purpose |
|---|---|
| [`aws/`](aws/README.md) | AWS resources as code (CDK): Cognito, Bedrock access, IoT Core, S3, SNS |
| [`docker/`](docker/README.md) | Optional development stack (PostgreSQL, NATS) |
