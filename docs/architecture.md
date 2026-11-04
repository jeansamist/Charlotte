# Charlotte — Architecture (proposal v0.1, 2026-11-04)

> Status: **draft for discussion**. Nothing here is implemented yet.

## 1. Goal
A self-hosted, installable AI home assistant for Linux (an Alexa / Google Home you own).
- Configurable **assistant name** (also used as the wake word) and **language**.
- Talk with it (voice + text), let it **act** (home automation, cameras, audio, custom skills).
- **Local-first**: works on the LAN without the cloud. The cloud (AWS, hosted LLM/voice providers) is optional and pluggable.

## 2. Guiding principles
1. **Every capability is a skill = an MCP server.** The brain is an MCP client. Home Assistant,
   cameras, audio, presence and future custom features all plug in the same way.
2. **Right language for the job.**
   - Rust: real-time and low level (audio, video, supervisor/CLI).
   - Python: AI/ML (LLM agent, speech, vision).
   - Node/TypeScript: product backend and UIs.
3. **One event bus, typed contracts.** Services never call each other's internals. They talk over
   the bus using schemas from `packages/contracts`, with generated types for Rust, Python and TS.
4. **Providers behind interfaces.** LLM, STT, TTS and wake word are adapters (local or cloud),
   chosen in config.
5. **Ship as a product.** One installer and a `charlotte` CLI. systemd services. Upgradable.

## 3. Runtime view

```
                ┌──────────── clients ─────────────┐
                │ Web GUI (React)  Mobile (RN/Expo)│
                └───────────────┬──────────────────┘
                                │ HTTPS + WebSocket
                      ┌─────────▼─────────┐        ┌────────────┐
                      │ gateway (Node/TS) │◄──────►│ PostgreSQL │
                      │ auth, accounts,   │        │ + pgvector │
                      │ config, API       │        └─────▲──────┘
                      └─────────┬─────────┘              │
 ═══════════════════════ event bus (NATS) ═══════════════╪════════════════
     │              │               │             │      │          │
┌────▼─────┐  ┌─────▼─────┐  ┌──────▼─────┐  ┌────▼─────┐│   ┌──────▼──────┐
│ audio-   │  │ voice     │  │ brain      │  │ vision   ││   │ supervisor  │
│ server   │◄►│ (Python)  │◄►│ (Python)   │  │ (Python) ││   │ (Rust)      │
│ (Rust)   │  │ wake word │  │ LLM agent, │  │ faces,   ││   │ health,     │
│ mix,duck,│  │ VAD, STT, │  │ memory,    │  │ presence │┘   │ updates,    │
│ mic/spkr │  │ TTS       │  │ MCP client │  └────▲─────┘    │ CLI         │
└──────────┘  └───────────┘  └─────┬──────┘       │          └─────────────┘
                                   │ MCP     ┌────┴─────┐
              ┌────────────────────┼─────────│ camera   │
              ▼                    ▼         │ (Rust)   │
     skills/home-assistant   skills/camera   │ RTSP/V4L2│
     skills/audio            skills/presence └──────────┘
     skills/<your-custom-skill> ...
```

### Voice loop
mic → **audio-server** → **voice** (wake word "<name>" → VAD → STT) → **brain** (LLM + tools via MCP)
→ **voice** (TTS) → **audio-server** (lowers the music while the assistant speaks) → speaker.

## 4. Components

| Component | Lang | Responsibility |
|---|---|---|
| `services/supervisor` | Rust | Installs, starts and monitors all services. Updates, health checks, the `charlotte` CLI. |
| `services/audio-server` | Rust | Owns audio devices (PipeWire). Mixes sources (music, TTS, alerts, streams). Ducking, zones, mic stream. |
| `services/camera` | Rust | Camera capture (V4L2/RTSP via GStreamer). Re-streams video (WebRTC/HLS). Sends frames to vision. |
| `services/voice` | Python | Wake word (openWakeWord, trained on the configured name), VAD, STT, TTS. Provider adapters. |
| `services/brain` | Python | Conversation agent, persona (name, language, tone), memory (pgvector), MCP client, permissions. |
| `services/vision` | Python | Face detection and recognition (embeddings in pgvector), person-at-home events. |
| `apps/gateway` | Node/TS | REST + WebSocket API. Auth, multi-account, households, roles, settings, skill management. |
| `apps/web` | React | Settings GUI, dashboards, camera view, audio mixer, chat. |
| `apps/mobile` | React Native | Remote control, chat, notifications, camera view, presence. |
| `skills/*` | any | MCP servers: `home-assistant`, `camera`, `audio`, `presence`, custom ones. |
| `packages/contracts` | protobuf/JSON Schema | Event and API schemas. Generates code for rs/py/ts. |
| `infra/aws` | CDK (TS) | Optional: remote-access relay, backups (S3), push notifications, Cognito for remote login. |

## 5. Data
- **PostgreSQL** (single local instance): accounts, households, devices, settings, conversation
  history, skills registry, audit log. **pgvector** for long-term memory and face embeddings.
- Media (snapshots, clips) on local disk with a retention policy. Optional S3 backup.
- Secrets (provider API keys, HA token) are encrypted at rest and never sent to clients.

## 6. Multi-account and presence
- Household → Users (roles: owner, admin, member, guest) → linked identities (face, voice, phone).
- Presence comes from several sources: face recognition, phone on Wi-Fi/BLE, and HA device trackers.
  Result: `person.<id>.home = true/false` events. The brain uses them for personalization
  ("Welcome home, Ephraim") and permissions.

## 7. Repository layout (monorepo)
```
apps/        gateway/  web/  mobile/
services/    supervisor/  audio-server/  camera/  voice/  brain/  vision/
skills/      home-assistant/  camera/  audio/  presence/  _template/
packages/    contracts/  sdk-ts/  sdk-py/  sdk-rs/
infra/       aws/  packaging/ (deb, systemd)  docker/ (dev: postgres, nats)
docs/
```
Tooling: Cargo workspace (Rust), uv workspace (Python), pnpm + Turborepo (TS), and a root `justfile`
as the single entry point (`just dev`, `just test`, `just build`, `just package`).

## 8. Packaging and install
- Target: `.deb` packages (built with nfpm) plus `curl … | sh` bootstrap. Each service is a systemd unit.
  The supervisor coordinates them.
- Dev: run Postgres and NATS in Docker. Run services natively with hot reload.

## 9. Proposed build order
1. Skeleton monorepo, contracts, NATS + Postgres dev stack, gateway with auth and accounts.
2. Text chat end to end: web → gateway → brain (LLM) → reply. Persona config (name, language).
3. Home Assistant skill (MCP) → "turn on the living room light".
4. Voice loop: audio-server + voice service (wake word, STT, TTS).
5. Camera + vision + presence.
6. Mobile app, AWS remote access, installer/packaging.

## 10. Open questions
- Target hardware: x86 mini-PC only, or also Raspberry Pi / ARM (affects local model choices)?
- Default LLM: local (Ollama/llama.cpp) or cloud (Claude, OpenAI…) with local fallback?
- Default voice providers (local Whisper/Piper vs ElevenLabs/Deepgram…)?
- Event bus: NATS (proposed) vs MQTT (native in HA ecosystem). An MQTT bridge is possible either way.
- Is AWS only for remote access and backups, or also for hosted inference?
