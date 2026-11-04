# gateway

Charlotte's backend: **accounts, login, settings**, and the authenticated entry point to the other services.
The web interface (and later the mobile app) talks only to the gateway.

- Node.js ≥ 20, TypeScript (ESM, strict), [Fastify 5](https://fastify.dev), `pg`, zod 4, `@node-rs/argon2`
- PostgreSQL: database and role `charlotte`. The schema is in [`docs/DATABASE.md`](docs/DATABASE.md)
- HTTP API reference: [`docs/API.md`](docs/API.md)

## Responsibilities
| Area | What |
|---|---|
| Authentication | Username + password. Argon2id hashing, random session token in an httpOnly cookie (`charlotte_session`), stored hashed in the DB, sliding 30-day expiry, 10 failed logins per 5 min per IP |
| Users | Roles `admin` and `member`. Admins manage users and settings |
| Settings | Assistant name and language, Bluetooth name. Stored in the `settings` table, which is the source of truth |
| Audio proxy | `/api/audio/*` → audio-server local API, behind login. Bluetooth pairing management is admin-only |
| Reconciler | Every 15 s, checks the audio-server. When it has (re)started, re-applies the Bluetooth name from the settings |
| Audit log | Logins, failed logins, user and settings changes → `audit_log` table |

## Source layout
```
src/
  index.ts          server start-up: config, migrations, cookie → user hook, routes, shutdown
  config.ts         environment variables (zod)
  db.ts             pg pool, migration runner, audit helper
  auth.ts           hashing, sessions, requireUser / requireAdmin, login rate limiter
  settings.ts       settings schema, read / update
  audio-client.ts   client for the audio-server API
  reconciler.ts     pushes settings to the audio-server after it restarts
  cli.ts            admin CLI (migrate, create-admin, reset-password, set, has-users)
  routes/           auth.ts, users.ts, settings.ts, audio.ts
migrations/         NNN_name.sql, applied in order, recorded in schema_migrations
```

## Configuration (environment)
| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | – (required) | `postgres://charlotte:…@127.0.0.1:5432/charlotte` |
| `HOST` / `PORT` | `127.0.0.1` / `4000` | Listen address. Keep it local: the web app proxies to it |
| `AUDIO_SERVER_URL` | `http://127.0.0.1:8080` | audio-server API |
| `COOKIE_SECURE` | `false` | `true` when the GUI is served over HTTPS |
| `SESSION_DAYS` | `30` | Session lifetime (renewed on every request) |
| `LOG_LEVEL` | `info` | Fastify / pino log level |

The installed service reads `~/.config/charlotte/gateway.env` (written by `scripts/install.sh`, chmod 600).
For development, copy [`.env.example`](.env.example) to `.env`.

## Commands
```bash
pnpm dev:gateway                                # from the repo root: tsx watch on :4000
pnpm --filter @charlotte/gateway build          # → dist/
pnpm --filter @charlotte/gateway typecheck

# admin CLI (loads DATABASE_URL from the environment)
node dist/cli.js migrate
node dist/cli.js create-admin <username>        # password from CHARLOTTE_ADMIN_PASSWORD or an interactive prompt
node dist/cli.js reset-password <username>      # also signs the user out everywhere
node dist/cli.js set <key> <value>              # e.g. set bluetooth.name "Living room"
node dist/cli.js has-users                      # exit 0 if a user exists
```

## Rules
- Validate every request body with zod. Return `{"error": "…"}` with a meaningful status code.
- Every route except `/api/auth/login`, `/api/auth/logout` and `/api/health` requires `requireUser` or `requireAdmin`.
- Add a new audio-server route to the allow-list in `routes/audio.ts`. Nothing is proxied by default.
- Never edit an applied migration: add a new file.
