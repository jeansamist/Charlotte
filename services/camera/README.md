# camera

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** Rust · GStreamer

## Role
Camera capture and streaming.

## Planned design
- Capture from V4L2 (USB cameras) and RTSP (IP cameras) through GStreamer.
- Re-streams to the web and mobile apps (WebRTC or HLS), with snapshots and clips saved to disk under a retention policy.
- Sends frames to `vision` at a reduced rate.

## Interfaces
- Local API (like audio-server) proxied by the gateway behind login.
- Frame feed to `vision`, events on NATS.

## Depends on
GStreamer, NATS.
