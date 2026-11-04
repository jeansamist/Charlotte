# CLAUDE.md — Charlotte

Living knowledge file for this project. Update it whenever something new is learned
about the project (purpose, stack, structure, commands, conventions, decisions).

## Project
- **Name:** Charlotte
- **Purpose:** Self-hosted, installable AI home assistant for Linux (like Alexa / Google Home), built by and for the owner.
  - Assistant **name** and **language** are configurable (name also used as the wake word).
  - It can talk with the user (voice + text) and act (home automation, cameras, audio, custom skills).
- **Initial capabilities:** an audio server that mixes several audio sources, a camera program (video stream), a web GUI for configuration,
  multi-account management, person-at-home detection, face recognition, Home Assistant connection,
  and a custom-feature/plugin system for capabilities added later.
- **Tech mix:** Rust, Python, Node.js, React, React Native, PostgreSQL, AWS, MCP, Home Assistant, AI voice providers, LLMs.
  It is a monorepo with many sub-projects.
- **Status:** Architecture proposal stage (2026-11-04). See `docs/architecture.md` (draft v0.3. Main choices validated, see Decisions log).

## Environment
- Code lives on a Debian machine, accessed over SSH from the owner's Mac.
  - Host `192.168.2.32`, user `jeansamist`, SSH alias `debian` (key `~/.ssh/id_ed25519_debian` on the Mac).
  - Project path: `/home/jeansamist/Charlotte`.
- Debian testing (forky/sid), kernel 7.1.x, 12 cores, 15 GB RAM.
- **Root disk is ~94% full (≈3 GB free)** — watch out with large installs, Docker images, `node_modules`.
- Tooling seen in home dir (not yet confirmed for this project): bun, cargo, AdonisJS v6 history.

## Decisions log
- 2026-11-04 — Target hardware: x86_64 Debian, like the dev box. No ARM for now.
- 2026-11-04 — LLMs: cloud, several models/providers.
- 2026-11-04 — Voice: cloud providers, several models.
- 2026-11-04 — AWS: use managed services (AI, auth, storage…) to avoid boilerplate.
- 2026-11-04 — Architecture style: keep **microservices on one box** (v0.2+). "Edge + cloud" and "core + plugins" were proposed and rejected.
- 2026-11-04 — Event bus: **NATS**.
- 2026-11-04 — **Basic commands must work offline**: local STT/TTS fallback (faster-whisper, Piper) and a local intent engine in `brain`.
- 2026-11-04 — Remote access through **our own mobile app** via AWS IoT Core relay + Cognito.
- 2026-11-04 — AWS region: **US (default `us-east-1`), configurable**.
- Pending — first languages to support, default wake word name.

## Sync workflow (Mac <-> GitHub <-> Debian)
- GitHub remote: `git@github-jeansamist:jeansamist/Charlotte.git` (branch `main`), same on both machines.
- Local copy (Mac): `/Users/jeansamist/lab/me/Charlotte`. Remote copy (Debian): `/home/jeansamist/Charlotte`.
- **Change made on the Mac** -> commit + push from the Mac -> `ssh debian 'cd ~/Charlotte && git pull'`.
- **Change made on Debian** -> commit + push on Debian -> `git pull` on the Mac.
- Always pull on the target side before editing, so the two copies never drift.

## Stack / Architecture
Proposed, pending owner validation. Full details are in `docs/architecture.md`.
- Every capability is a **skill = MCP server**. The `brain` (Python) is the MCP client.
- Rust: `supervisor` (CLI/installer), `audio-server`, `camera`.
- Python: `brain` (LLM agent), `voice` (wake word, STT, TTS), `vision` (faces/presence).
- Node/TS: `gateway` API. React `web`, React Native `mobile`.
- Offline: local intent engine + local STT/TTS fallback. Remote: mobile app ↔ AWS IoT Core ↔ box (outbound only).
- NATS event bus + typed contracts (`packages/contracts`). PostgreSQL + pgvector.
- Cloud-powered: LLMs on **Amazon Bedrock** (several models), agent built with **Strands Agents**.
  Cloud STT/TTS (Transcribe, Polly, ElevenLabs…). Wake word + VAD stay local.
- AWS managed services to avoid boilerplate: Cognito (auth), S3 (media/backups), SNS (push). CDK (TS) for infra.
- Face recognition: local by default (InsightFace). Rekognition is an optional adapter.
- Packaging: .deb + systemd. Docker only for the dev stack.

## Commands
_TBD (install, dev, build, test, lint)_

## Conventions
_TBD_

## Documentation to write (pending owner input)
- [ ] `README.md`
- [ ] Developer guide
