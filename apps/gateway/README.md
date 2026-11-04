# gateway

> **Status: mock / not implemented yet.** This folder is a placeholder from the architecture
> (see `docs/architecture.md`). The first component being built is `services/audio-server`.

- **Language / tech:** Node.js / TypeScript
- **Role:** REST + WebSocket API for the web and mobile apps. Validates Cognito JWTs. Manages households, roles, settings and skills. Bridges NATS ↔ AWS IoT Core for remote access.
