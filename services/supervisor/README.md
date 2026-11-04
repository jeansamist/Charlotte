# supervisor

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** Rust

## Role
Installs, starts and watches every Charlotte service, and provides the `charlotte` command.

## Planned design
- Health checks and restarts (on top of systemd), updates, version reporting.
- `charlotte` CLI: status, logs, update, backup, reset password.
- Will replace parts of `scripts/install.sh` once packaging moves to `.deb` files.

## Interfaces
- Reads every service's health endpoint. Publishes `system.*` events (including `system.connectivity` for offline mode).

## Depends on
systemd, NATS.
