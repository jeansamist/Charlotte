# Gateway HTTP API

- **Base URL:** `http://127.0.0.1:4000`. Browsers use the web app's origin (`http://<server>:3000/api/...`), which
  forwards `/api/*` to the gateway.
- **Auth:** session cookie `charlotte_session` (httpOnly, SameSite=Lax), set by `POST /api/auth/login`.
- **Errors:** `{"error": "message"}` with 400 (invalid input), 401 (not signed in or wrong credentials),
  403 (administrator only), 404, 409 (conflict), 429 (too many login attempts, `retry-after` header),
  or 502 (audio-server unreachable).

Access column: **public** = no login, **user** = any signed-in user, **admin** = role `admin`.

## Health
| Method and path | Access | Response |
|---|---|---|
| `GET /api/health` | public | `{"status":"ok","audioServer":"up"\|"down"}` (checks the database) |

## Authentication
| Method and path | Access | Body | Response |
|---|---|---|---|
| `POST /api/auth/login` | public | `{"username","password"}` | `{"user":{id,username,displayName,role}}` and sets the cookie |
| `POST /api/auth/logout` | public | – | `{"ok":true}`, deletes the session |
| `GET /api/auth/me` | user | – | `{"user":{id,username,displayName,role}}` |
| `POST /api/auth/password` | user | `{"current","next"}` (next: 10–128 chars) | `{"ok":true}`, signs out the user's other sessions |

Usernames are lowercase, 3–32 characters: letters, digits, `.`, `_`, `-`.

## Users (admin)
| Method and path | Body | Response / rules |
|---|---|---|
| `GET /api/users` | – | `{"users":[{id,username,displayName,role,createdAt}]}` |
| `POST /api/users` | `{"username","password","displayName"?,"role"?:"admin"\|"member"}` | 201 `{"user"}`, 409 if the username exists |
| `PATCH /api/users/:id` | `{"displayName"?,"role"?,"password"?}` | `{"user"}`. A new password signs the user out. You cannot remove your own admin role |
| `DELETE /api/users/:id` | – | `{"ok":true}`. You cannot delete yourself |

## Settings
| Method and path | Access | Body | Response |
|---|---|---|---|
| `GET /api/settings` | user | – | `{"settings":{...}}` |
| `PUT /api/settings` | admin | any subset of the keys below | `{"settings":{...},"warning"?}` |

| Key | Type | Effect |
|---|---|---|
| `assistant.name` | string, 1–40 | Assistant name (future wake word) |
| `assistant.language` | `"en"` \| `"fr"` | Assistant language |
| `bluetooth.name` | string, 1–60 | Applied immediately to the audio-server, and again whenever it restarts. `warning` is set if it was unreachable |

## Audio (proxy to the audio-server)
Every route below forwards to the same path under `/api` on the audio-server, with the same body and response.
See [`services/audio-server/docs/API.md`](../../../services/audio-server/docs/API.md).

| Method and path | Access |
|---|---|
| `GET /api/audio/health`, `GET /api/audio/metrics` | user |
| `GET /api/audio/mixer` | user |
| `PUT /api/audio/mixer/channels/:slot/volume`, `PUT /api/audio/mixer/channels/:slot/mute` | user |
| `PUT /api/audio/mixer/master/volume`, `PUT /api/audio/mixer/master/mute` | user |
| `PUT /api/audio/mixer/ducking` | user |
| `GET /api/audio/outputs`, `PUT /api/audio/outputs/current` | user |
| `GET /api/audio/bluetooth` | user |
| `POST /api/audio/bluetooth/pairing`, `DELETE /api/audio/bluetooth/pairing` | admin |
| `DELETE /api/audio/bluetooth/devices/:address` | admin |

Change the Bluetooth name through `PUT /api/settings`, not through the proxy, so that it is saved.

## Example
```bash
curl -c jar -H 'content-type: application/json' -d '{"username":"admin","password":"…"}' localhost:3000/api/auth/login
curl -b jar localhost:3000/api/audio/mixer
curl -b jar -X PUT -H 'content-type: application/json' -d '{"bluetooth.name":"Salon"}' localhost:3000/api/settings
```
