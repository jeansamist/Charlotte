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
- Tooling: Rust 1.96 (~/.cargo/bin). Also seen: bun, AdonisJS v6 history.
- Audio/BT on the box: PipeWire 1.6.7, WirePlumber 0.5.15, BlueZ 5.85, Intel AX201 Bluetooth, Alder Lake HDA speaker.
  The default sink is an snd_aloop "Loopback" device, and the speaker is muted at volume 0. Don't change the user's audio settings without asking.
- No passwordless sudo. Ask the user to run `sudo` commands themselves.

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

## Implemented components (2026-11-04)
| Path | Tech | Role | Runs as |
|---|---|---|---|
| `services/audio-server` | Rust (PipeWire, BlueZ/zbus 6, axum) | Bluetooth multi-source mixer, smart mixing (ducking), pairing agent, local API on 127.0.0.1:8080 (**no auth**) | `audio-server.service` (user) |
| `apps/gateway` | Node 24, TypeScript, Fastify 5, pg, argon2id, zod 4 | Login (username + password), sessions (DB, httpOnly cookie), users/roles, settings, authenticated proxy to audio-server | `charlotte-gateway.service` on 127.0.0.1:4000 |
| `apps/web` | Next.js 16 (App Router, standalone), React 19, Tailwind 4 | GUI: login, Audio, Bluetooth, Settings, Users, Account. Proxies `/api/*` → gateway (same origin) | `charlotte-web.service` on 0.0.0.0:3000 |
| `scripts/install.sh` | bash | Full Debian install/update (idempotent): packages, toolchains, builds, audio-server, BT class, Postgres role+DB, migrations, admin, user services, linger | – |

- **Database:** existing Docker container `postgres_database` (postgres:17, port 5432, superuser `root`, restart unless-stopped).
  Charlotte uses its own role and database `charlotte`. The password is in `~/.config/charlotte/gateway.env` (chmod 600) on Debian.
  Other databases in that container belong to other projects: never touch them.
- **Settings source of truth = DB** (`settings` table). The gateway reconciler pushes the Bluetooth name to audio-server whenever it (re)starts.
- **Auth for now = local username/password** (owner decision 2026-11-04). Cognito/AWS comes later. Roles: admin, member.
- Admin CLI: `(set -a; . ~/.config/charlotte/gateway.env; set +a; node apps/gateway/dist/cli.js create-admin|reset-password <user>|set <key> <value>|migrate)`.
- Node on Debian comes from **nvm** (non-interactive shells must `source ~/.nvm/nvm.sh`). pnpm 10 workspace (`apps/gateway`, `apps/web`).
- Update the Debian install after pulling: `bash scripts/install.sh` (safe to re-run).
- **Web UI rules (owner, 2026-11-04): shadcn/ui components (all installed), Outfit font, Hugeicons icons only, no emojis.**
  Details in `apps/web/README.md` → "UI rules". The `cn` npm package is shadcn's own (not a mistake).
- UI text is English for now. The owner speaks **French**, so answer in French.

## Current focus (2026-11-04)
**`services/audio-server`** (Rust) is the first component built. Everything else in the monorepo is a
**mock/placeholder** (README only). Spec: multi-source Bluetooth audio mixer (BlueZ + PipeWire).
Plan and milestones: `services/audio-server/docs/PLAN.md`. M1 = PoC: 2 phones over A2DP → mixed → laptop speaker.
- Real-time rule: never allocate, lock or block in PipeWire `process` callbacks (rtrb buffers, atomics, command queue).
- `bluez5.media-source-role = input` (WirePlumber rule) is required. Otherwise phones play straight to the speakers.
- PipeWire runs **per user** → the service is a systemd **user** unit, and the scripts must not run as root.
- Owner requirements (2026-11-04):
  - **Any Bluetooth output device must work** (AirPods, earbuds, speakers…). Planned: an automatic
    "bluetooth" output mode that follows whichever BT sink is connected and switches on connect/disconnect.
  - **Custom Bluetooth name** (`bluetooth.name`, default "Charlotte", set as the adapter alias over D-Bus).
  - Debian must appear on the Mac as an **audio output device** → Class of Device must be Audio/Video
    (`scripts/setup-bluetooth.sh` sets `Class = 0x240414` in /etc/bluetooth/main.conf, needs sudo).
- WirePlumber creates `bluez_input.<addr with colons>` nodes with `bluez5.loopback=true` for headset mics.
  Never capture them (it switches the headset to HFP). The mixer inspects full node info before capturing.
- BlueZ's *hostname* plugin overwrites `Class` from main.conf with Computer/Laptop (chassis=laptop).
  iOS then hides the device. setup-bluetooth.sh adds a systemd drop-in `bluetoothd --noplugin=hostname`.
- Stop a background mixer with `pkill -TERM -x audio-server` (SIGINT is ignored by `&` jobs; never `pkill -f "audio-server run"`, which matches the ssh shell itself).
- Mac sounded choppy because it connected in **HFP (mSBC 16 kHz)**. The WirePlumber config now limits roles to A2DP/BAP
  (`bluez5.roles`, `hfphsp-backend = none`, no autoswitch), and the mixer captures only `api.bluez5.profile` a2dp*/bap*.
- **Smart mixing (ducking)** is required: the newest device that starts playing takes the foreground, the others fade to
  `ducking.level_db`, and are restored after `hold_ms` of silence. It can be toggled and tuned from the GUI.
- Pairing: audio-server's own agent (NoInputNoOutput). New devices can pair only while the pairing window is open
  (Discoverable + Pairable share a timeout). Paired devices are trusted.
- Performance baseline (2026-11-04): audio-server < 1% CPU, ~15 MB RSS, 0 xruns. The PipeWire data-loop runs at RR 20.
  Clock drift from the Mac caused audible "drift_skips" → replaced by gentle 1-frame slips (`slips` counter).
  Capture streams use `resample.quality=10`. SBC-XQ is enabled, but the soundcore R50i negotiates plain SBC.
  Debian's PipeWire has no AAC (no fdk-aac), so phones and Macs use SBC.
- The Debian box is on **Wi-Fi** (wlo1, Intel AX201 combo chip shares the antenna with Bluetooth). Ethernet is recommended for audio stability.
- After a restart of wireplumber/bluetooth, the Bluetooth headphones do not reconnect by themselves (`bluetoothctl connect <addr>`).
  Auto-reconnect of the last output is a candidate feature.
- Test setup: MacBook → (A2DP) → Debian mixer → soundcore R50i earbuds (41:42:9A:86:8B:AA).
- Spec rules: work incrementally; no HTTP API or frontend until the M1 audio pipeline works; never claim
  Bluetooth capabilities that have not been tested.

## Commands
Rust code is edited on the Mac but **built and run on Debian** (PipeWire headers are Linux-only):
```bash
ssh debian 'cd ~/Charlotte && git pull && cargo test -p audio-server && cargo build --release -p audio-server'
ssh debian '~/Charlotte/target/release/audio-server list-nodes'
```
Pure DSP tests (no PipeWire) can also run on the Mac through a scratch crate that includes `src/audio/{buffer,mixer,limiter,engine}.rs`.
Root `Cargo.toml` is a workspace. Dev profile uses `debug = "line-tables-only"` (low disk on the Debian box).

## Conventions
_TBD_

## Documentation
- Root `README.md`, `docs/DEVELOPER_GUIDE.md`, `docs/OPERATIONS.md`, `docs/architecture.md`.
- Each component has a README (planned ones state "planned, not implemented"). API references:
  `services/audio-server/docs/API.md`, `apps/gateway/docs/API.md`. DB: `apps/gateway/docs/DATABASE.md`.
- **Keep the docs in sync with the code**: update the matching README/API doc in the same commit as the change.
