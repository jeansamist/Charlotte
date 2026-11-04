# Developer guide

How to work on Charlotte: machines, workflow, running each part, conventions and tests.

## 1. Machines and workflow
Charlotte is developed on a **Mac** and runs on a **Debian machine**. The PipeWire and BlueZ parts only build
and run on Linux.

| | Mac (editing) | Debian (build, run, test) |
|---|---|---|
| Path | `~/lab/me/Charlotte` | `~/Charlotte` |
| Access | – | `ssh debian` (alias for `jeansamist@192.168.2.32`, key `~/.ssh/id_ed25519_debian`) |
| Remote | `git@github-jeansamist:jeansamist/Charlotte.git` | same |

**Synchronisation goes through GitHub, in both directions:**
- Change made on the Mac → commit + push → on Debian: `cd ~/Charlotte && git pull`.
- Change made on Debian → commit + push there → on the Mac: `git pull`.
- Always pull before editing, so the two copies never drift.

Generated files that must be committed from Debian: `pnpm-lock.yaml`, `Cargo.lock`, `apps/web/next-env.d.ts`.

## 2. Prerequisites (Debian)
`scripts/install.sh` installs everything. Manually, you need:
- `bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth` (runtime)
- `libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential` (to build audio-server)
- Rust ≥ 1.85 (`rustup`), Node.js ≥ 20 (nvm on the dev box), pnpm 10
- Docker, with a PostgreSQL container (the dev box uses `postgres_database`, PostgreSQL 17)

Node on the dev box comes from **nvm**. In non-interactive shells (`ssh debian '…'`), run `source ~/.nvm/nvm.sh` first.

## 3. Repository structure
- **Cargo workspace** (root `Cargo.toml`): `services/audio-server`. The dev profile uses `debug = "line-tables-only"`
  because the dev box has little disk space.
- **pnpm workspace** (root `package.json`, `pnpm-workspace.yaml`): `apps/gateway`, `apps/web`.
- Planned components are folders with a README only. Read it before starting one.

## 4. Running each part in development
The installed services (systemd, see [OPERATIONS.md](OPERATIONS.md)) use the same ports as the dev commands.
Stop the one you are working on first, for example `systemctl --user stop charlotte-gateway`.

### audio-server (Rust)
```bash
cargo test -p audio-server                       # unit tests (DSP, buffers, ducking, config, …)
cargo build -p audio-server --release
./target/release/audio-server list-nodes         # PipeWire audio nodes and their classification
./target/release/audio-server run --simulate 2 --no-bluetooth   # two test tones, no devices needed
./target/release/audio-server run --log-level debug
```
Stop a background run with `pkill -TERM -x audio-server`. Never use `pkill -f "audio-server run"`: it also matches
your own ssh shell. SIGINT is ignored by jobs started with `&` from a script.
The pure DSP modules (`buffer`, `mixer`, `limiter`, `engine`) have no PipeWire dependency and can be tested on the Mac
with a scratch crate that includes them with `#[path]`.

### gateway (Node/TypeScript)
```bash
cp apps/gateway/.env.example apps/gateway/.env    # or reuse ~/.config/charlotte/gateway.env
pnpm dev:gateway                                  # tsx watch, http://127.0.0.1:4000
pnpm --filter @charlotte/gateway typecheck
pnpm --filter @charlotte/gateway build            # → apps/gateway/dist
```
Migrations run automatically at start-up.

### web (Next.js)
```bash
pnpm dev:web                                      # http://localhost:3000, /api/* → gateway (GATEWAY_URL)
pnpm --filter @charlotte/web build                # standalone output in apps/web/.next/standalone
```
To open the dev server from the Mac: `ssh -L 3000:localhost:3000 debian`, then open http://localhost:3000.

## 5. Conventions
### General
- **Incremental work.** Each step must build and run on Debian before the next one.
- **Never claim untested hardware behaviour.** This applies especially to Bluetooth: test on real devices and
  write findings in `services/audio-server/docs/PLAN.md`.
- **Keep `CLAUDE.md` up to date** with every decision and finding.
- Commits: `type(scope): summary` (`feat`, `fix`, `docs`, `chore`), with the scope being the component
  (`audio-server`, `gateway`, `web`).

### Rust (audio-server)
- **Real-time rule:** code that runs in PipeWire `process` callbacks never allocates, locks, blocks or logs.
  - Use the `rtrb` ring buffers, atomics (`AtomicF32` in `audio/mixer.rs`) and the engine command queue.
  - Removed inputs are sent back to the main thread to be freed there.
- The PipeWire main loop runs on the main thread. The HTTP API has its own thread and Tokio runtime, and
  shares state through `shared.rs` (atomics, plus `Mutex` for small lists).
- Every config key is validated in `Config::validate()`, which reports all errors at once.
- Tests sit next to the code (`#[cfg(test)]`).

### TypeScript (gateway, web)
- ES modules, `strict` TypeScript. Validate every request body with zod.
- The gateway is the only component with authentication. Anything it exposes must use `requireUser` or `requireAdmin`.
- The web app calls only same-origin `/api/*`. It never calls the audio-server directly.
- SQL lives in `apps/gateway/migrations/NNN_name.sql`. **Never edit an applied migration:** add a new file.

## 6. Recipes
**Add an audio-server API endpoint**
1. Add the handler and route in `services/audio-server/src/api.rs`, using `Shared` for state.
2. Allow it in the gateway proxy list, `apps/gateway/src/routes/audio.ts` (mark `admin: true` if needed).
3. Document it in `services/audio-server/docs/API.md` and `apps/gateway/docs/API.md`.

**Add a setting**
1. Add the key and its zod rule to `SettingsSchema` (`apps/gateway/src/settings.ts`).
2. Add a migration that inserts the default value.
3. If another service needs it, apply it in `reconciler.ts`.
4. Add the field to `Settings` in `apps/web/lib/api.ts` and to the Settings page.

**Add a database table:** create `apps/gateway/migrations/002_<name>.sql`. It is applied at the next start or by
`node dist/cli.js migrate`.

**Add a page to the GUI:** create `apps/web/app/(app)/<name>/page.tsx` (client component, `usePoll` + `api`), then add
it to `NAV` in `app/(app)/layout.tsx`.

## 7. Testing on real devices
Reference setup: MacBook → Debian (A2DP) → mixer → soundcore R50i earbuds, with an iPhone as the second source.
1. `curl -s localhost:8080/api/mixer | python3 -m json.tool` shows the inputs, levels and ducking state.
2. `curl -s localhost:8080/api/bluetooth` shows the adapter class (must be Audio/Video) and the devices.
3. Logs: `journalctl --user -u audio-server -f`.

Check that every Bluetooth stream uses an A2DP profile (`pw-dump`: `api.bluez5.profile` = `a2dp-*`). A `headset-*`
profile means the call mode, which sounds choppy (see [OPERATIONS.md](OPERATIONS.md#troubleshooting)).
