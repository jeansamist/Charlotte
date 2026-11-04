# vision

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** Python

## Role
Face detection and recognition, and "who is home" (presence).

## Planned design
- Face detection and recognition with InsightFace on CPU. Embeddings stored in pgvector and linked to user accounts.
- Amazon Rekognition as an optional adapter (local is the default: continuous video is costly and privacy-sensitive
  to send to the cloud).
- Presence fusion: faces, phones on Wi-Fi/Bluetooth, Home Assistant device trackers.

## Interfaces
- Consumes frames from `services/camera`.
- Publishes `person.<id>.home` and `vision.face_seen`.

## Depends on
camera, PostgreSQL + pgvector, NATS.
