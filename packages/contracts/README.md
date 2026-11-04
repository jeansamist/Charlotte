# contracts

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** protobuf or JSON Schema

## Role
Shared definitions of NATS events and internal APIs, so that the Rust, Python and TypeScript services agree.

## Planned design
- One schema per event or message (e.g. `voice.transcript`, `brain.reply`, `person.home`, `audio.mixer.changed`).
- Code generation for Rust, Python and TypeScript (`packages/sdk-*`).
- Versioned: breaking changes create a new subject or version.

## Interfaces
- Consumed by every service.

## Depends on
–
