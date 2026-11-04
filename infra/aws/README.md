# infra/aws

> **Status: planned, not implemented.**

**Tech:** AWS CDK (TypeScript). The region is configurable (default `us-east-1`; Bedrock model availability
differs by region).

Planned resources (managed services, to avoid writing boilerplate):
| Need | Service |
|---|---|
| LLM access (several models, one API) | Amazon Bedrock (IAM role for the box) |
| Speech-to-text / text-to-speech | Amazon Transcribe / Amazon Polly |
| Sign-in for the mobile app and remote access | Amazon Cognito |
| Remote access without opening ports | AWS IoT Core (the box keeps one outbound connection) |
| Camera clips and database backups | Amazon S3 |
| Mobile push notifications | Amazon SNS (FCM / APNs) |
| Optional face recognition | Amazon Rekognition |

Local login (username + password in PostgreSQL) stays the default on the LAN.
