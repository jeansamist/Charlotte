# infra/docker

> **Status: planned, not implemented.**

An optional Docker Compose stack for development: PostgreSQL (+ pgvector) and NATS.

Today the Debian machine uses its **existing** PostgreSQL container `postgres_database` (database and role
`charlotte`, created by `scripts/install.sh`). NATS is not needed yet.
