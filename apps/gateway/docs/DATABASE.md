# Gateway database

PostgreSQL (17 on the dev box), database **`charlotte`**, owned by the role **`charlotte`**.
On the Debian machine it runs inside the existing Docker container `postgres_database`. Other databases in that
container belong to other projects.

## Migrations
- Files: `apps/gateway/migrations/NNN_name.sql`, applied in name order, each in its own transaction.
- Applied files are recorded in `schema_migrations(name, applied_at)`.
- They run automatically when the gateway starts, or with `node dist/cli.js migrate`.
- **Never edit an applied migration**: add `002_…sql`, `003_…sql`, and so on.

## Schema (001_init.sql)
### `users`
| Column | Type | Notes |
|---|---|---|
| `id` | uuid PK | `gen_random_uuid()` |
| `username` | text, unique | `^[a-z0-9][a-z0-9_.-]{2,31}$` |
| `display_name` | text | default `''` |
| `password_hash` | text | Argon2id (m = 19 MiB, t = 2, p = 1) |
| `role` | text | `admin` or `member` |
| `created_at`, `updated_at` | timestamptz | |

### `sessions`
| Column | Type | Notes |
|---|---|---|
| `token_hash` | text PK | SHA-256 (hex) of the cookie token. The token itself is never stored |
| `user_id` | uuid → users, cascade | |
| `created_at`, `last_seen_at` | timestamptz | |
| `expires_at` | timestamptz | Pushed to now + `SESSION_DAYS` on each request. Expired rows are purged hourly |
| `user_agent`, `ip` | text | For auditing |

### `settings`
| Column | Type | Notes |
|---|---|---|
| `key` | text PK | e.g. `bluetooth.name` |
| `value` | jsonb | e.g. `"Charlotte"` |
| `updated_at` | timestamptz | |
| `updated_by` | uuid → users, set null | |

Defaults inserted by the migration: `assistant.name = "Charlotte"`, `assistant.language = "en"`,
`bluetooth.name = "Charlotte"`. Valid keys and values are defined by `SettingsSchema` in `src/settings.ts`.

### `audit_log`
| Column | Type | Notes |
|---|---|---|
| `id` | bigserial PK | |
| `user_id` | uuid → users, set null | null for failed logins |
| `action` | text | `login`, `login.failed`, `password.changed`, `user.created`, `user.updated`, `user.deleted`, `settings.updated` |
| `details` | jsonb | |
| `created_at` | timestamptz | |

## Useful queries
```bash
docker exec -it postgres_database psql -U root -d charlotte
```
```sql
SELECT username, role, created_at FROM users;
SELECT u.username, s.last_seen_at, s.ip FROM sessions s JOIN users u ON u.id = s.user_id ORDER BY s.last_seen_at DESC;
SELECT created_at, action, details FROM audit_log ORDER BY id DESC LIMIT 20;
```
