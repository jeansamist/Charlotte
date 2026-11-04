# Charlotte

A **self-hosted AI home assistant** for Linux, in the spirit of Alexa or Google Home, but owned and run by you.
You can talk to it, it can act for you, and every capability is a plug-in.
The assistant's **name** (also its wake word) and its **language** are configurable.

> **Status (2026-11-04):** the audio system, web interface, accounts and installer work on Debian.
> The AI parts (brain, voice, vision) are designed but not implemented yet. See [Status](#status).

## What works today
- **Multi-source Bluetooth speaker.** Phones and computers connect to "Charlotte" (configurable name) as a speaker.
  Several can play at the same time and are **mixed in real time**.
- **Smart mixing (ducking).** When a device starts playing, the others are lowered so it can be heard,
  then restored when it stops.
- **Any Bluetooth output.** The mix plays on the laptop speaker, or on any Bluetooth headphones or speaker that connects.
- **Web interface** (`http://<server>:3000`) with **username/password login**: mixer, Bluetooth (name, pairing,
  devices), assistant settings, users and roles, account.
- **One-command install** on Debian: [`scripts/install.sh`](scripts/README.md).

## Status
| Component | Path | Tech | Status |
|---|---|---|---|
| Audio server | [`services/audio-server`](services/audio-server/README.md) | Rust, PipeWire, BlueZ | ✅ Working (milestones M1–M2) |
| Gateway (API, accounts) | [`apps/gateway`](apps/gateway/README.md) | Node.js, TypeScript, Fastify, PostgreSQL | ✅ Working |
| Web interface | [`apps/web`](apps/web/README.md) | Next.js, React, Tailwind | ✅ Working |
| Installer and scripts | [`scripts`](scripts/README.md) | bash | ✅ Working |
| Brain (LLM agent) | [`services/brain`](services/brain/README.md) | Python, Strands, Bedrock | 📝 Planned |
| Voice (wake word, STT/TTS) | [`services/voice`](services/voice/README.md) | Python | 📝 Planned |
| Vision (faces, presence) | [`services/vision`](services/vision/README.md) | Python | 📝 Planned |
| Camera | [`services/camera`](services/camera/README.md) | Rust | 📝 Planned |
| Supervisor / CLI | [`services/supervisor`](services/supervisor/README.md) | Rust | 📝 Planned |
| Mobile app | [`apps/mobile`](apps/mobile/README.md) | React Native | 📝 Planned |
| Skills (MCP servers) | [`skills/`](skills/README.md) | any | 📝 Planned |
| Shared contracts | [`packages/contracts`](packages/contracts/README.md) | protobuf / JSON Schema | 📝 Planned |
| Infrastructure | [`infra/`](infra/README.md) | AWS CDK, Docker | 📝 Planned |

## How it fits together (today)
```
 iPhone / Mac ──Bluetooth A2DP──►┌──────────────── Debian machine ─────────────────┐
                                 │ BlueZ + PipeWire                                 │
                                 │   └─► audio-server (Rust) ── mix ──► speaker /   │──► Bluetooth headphones
                                 │        local API 127.0.0.1:8080                  │
 Browser ──HTTP :3000──────────► │ web (Next.js) ──/api──► gateway (Node) :4000     │
                                 │                           └─► PostgreSQL (Docker)│
                                 └──────────────────────────────────────────────────┘
```
Only the web interface listens on the network. The gateway and the audio-server API listen on `127.0.0.1`.
The full target architecture (brain, voice, vision, skills, AWS) is in [`docs/architecture.md`](docs/architecture.md).

## Quick start (Debian, x86_64)
Requirements: Debian testing/unstable (or recent Debian-based), a Bluetooth adapter, Docker with a PostgreSQL
container, and a user in the `docker` group.

```bash
git clone git@github.com:jeansamist/Charlotte.git ~/Charlotte
cd ~/Charlotte
bash scripts/install.sh --bt-name "Charlotte"     # asks for sudo only when needed
```
Then open `http://<server-ip>:3000` and sign in with the administrator created during the install.
Details and options: [`scripts/README.md`](scripts/README.md). Day-to-day operation: [`docs/OPERATIONS.md`](docs/OPERATIONS.md).

## Repository layout
```
apps/        gateway/ (API, accounts)   web/ (Next.js GUI)   mobile/ (planned)
services/    audio-server/ (Rust mixer)  brain/ voice/ vision/ camera/ supervisor/ (planned)
skills/      MCP servers that give the assistant its capabilities (planned)
packages/    contracts/ shared schemas (planned)
infra/       aws/ (CDK, planned)  docker/ (dev stack, planned)
scripts/     install.sh — full Debian install / update
docs/        architecture, developer guide, operations
```

## Documentation
| Document | For |
|---|---|
| [`docs/architecture.md`](docs/architecture.md) | Target architecture, decisions, roadmap |
| [`docs/DEVELOPER_GUIDE.md`](docs/DEVELOPER_GUIDE.md) | Working on the code: setup, workflow, conventions, tests |
| [`docs/OPERATIONS.md`](docs/OPERATIONS.md) | Running Charlotte: services, logs, updates, backups, troubleshooting |
| [`services/audio-server/README.md`](services/audio-server/README.md) | Audio server: features, configuration, CLI |
| [`services/audio-server/docs/API.md`](services/audio-server/docs/API.md) | Audio server HTTP API |
| [`services/audio-server/docs/PLAN.md`](services/audio-server/docs/PLAN.md) | Audio server design, milestones, findings |
| [`apps/gateway/README.md`](apps/gateway/README.md) · [`docs/API.md`](apps/gateway/docs/API.md) · [`docs/DATABASE.md`](apps/gateway/docs/DATABASE.md) | Gateway |
| [`apps/web/README.md`](apps/web/README.md) | Web interface |
| [`CLAUDE.md`](CLAUDE.md) | Project knowledge file for AI-assisted development |

## License
Private project. All rights reserved (the `audio-server` crate is marked MIT in its manifest).
