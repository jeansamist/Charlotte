# web

Charlotte's web interface: configure and control the home assistant from a browser, on a computer or a phone.

- Next.js 16 (App Router, `output: "standalone"`), React 19, Tailwind CSS 4, TypeScript
- Talks only to same-origin `/api/*`. Next.js **rewrites** these requests to the gateway (`GATEWAY_URL`, default
  `http://127.0.0.1:4000`), so the session cookie stays first-party and the gateway is never exposed.
- Light and dark themes follow the system setting. Responsive layout (sidebar on desktop, tabs on mobile).

## Pages
| Route | Access | Content |
|---|---|---|
| `/login` | public | Username and password |
| `/` **Audio** | user | Output selector, master volume and mute, master meter and limiter indicator, smart mixing toggle and level, one row per input (status *playing / silent / lowered / muted*, meter, volume 0–200%, mute). Refreshes every 0.5 s |
| `/bluetooth` | user (actions: admin) | Name shown to other devices, address, type (warns if not Audio/Video), pairing window (open 3 min / stop), known devices (connected, trusted, forget) |
| `/settings` | user (edit: admin) | Assistant name and language, Bluetooth name |
| `/users` | admin | Accounts: create, change role, reset password, delete |
| `/account` | user | Profile, change password |

Every page under `app/(app)/` requires a session. The layout calls `/api/auth/me` and redirects to `/login` on 401.

## Source layout
```
app/
  layout.tsx, globals.css     root layout, theme colours (CSS variables → Tailwind tokens)
  login/page.tsx
  (app)/layout.tsx            session check, navigation, sign out
  (app)/page.tsx              Audio
  (app)/bluetooth|settings|users|account/page.tsx
components/ui.tsx             Card, Button, Input, Select, Badge, Alert, Meter, Slider, Toggle, Spinner
components/session.tsx        current-user context (useUser)
lib/api.ts                    fetch wrapper (JSON, errors, 401 → /login) and API types
lib/hooks.ts                  usePoll (fetch + refresh interval), useThrottle (sliders)
next.config.ts                standalone output, /api rewrite
```

## Commands
```bash
pnpm dev:web                                   # from the repo root, http://localhost:3000
pnpm --filter @charlotte/web build             # → .next/standalone
pnpm --filter @charlotte/web typecheck
```
`GATEWAY_URL` is read at **build time**, because rewrites are baked into the standalone build.

## Production
`scripts/install.sh` builds the app, copies `.next/static` into the standalone folder and runs the server as the user
service `charlotte-web`:
`node apps/web/.next/standalone/apps/web/server.js` with `PORT=3000`, `HOSTNAME=0.0.0.0`.

## Notes
- The UI text is English for now. A French translation is planned (the assistant language setting already exists).
- Plain HTTP on the LAN. Use a reverse proxy with HTTPS (and `COOKIE_SECURE=true` on the gateway) for remote access.
