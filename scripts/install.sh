#!/usr/bin/env bash
# Charlotte — installs / updates everything on a Debian machine, for the current user.
#
#   scripts/install.sh [--bt-name NAME] [--admin USERNAME] [--pg-container NAME]
#                      [--web-port 3000] [--skip-bluetooth-setup]
#
# What it does (each step is skipped when already done; safe to re-run to update):
#   1. system packages (apt, asks for sudo only if something is missing)
#   2. Rust + Node.js/pnpm toolchains
#   3. builds audio-server (Rust), gateway (Node/TS) and web (Next.js)
#   4. audio-server: binary, config, WirePlumber rule, user service; Bluetooth name
#   5. Bluetooth: announces this machine as a speaker (sudo, once)
#   6. PostgreSQL: starts the Docker container, creates the "charlotte" role + database
#   7. gateway: config (~/.config/charlotte/gateway.env), migrations, first administrator
#   8. systemd user services: audio-server, charlotte-gateway, charlotte-web (+ linger)
set -euo pipefail

BT_NAME="Charlotte"
ADMIN_USER="admin"
PG_CONTAINER="postgres_database"
WEB_PORT=3000
GATEWAY_PORT=4000
SKIP_BT_SETUP=0
while [ $# -gt 0 ]; do
  case "$1" in
    --bt-name) BT_NAME="$2"; shift 2;;
    --admin) ADMIN_USER="$2"; shift 2;;
    --pg-container) PG_CONTAINER="$2"; shift 2;;
    --web-port) WEB_PORT="$2"; shift 2;;
    --skip-bluetooth-setup) SKIP_BT_SETUP=1; shift;;
    -h|--help) sed -n '2,17p' "$0"; exit 0;;
    *) echo "unknown option: $1"; exit 2;;
  esac
done

ROOT=$(cd "$(dirname "$0")/.." && pwd)
CONF_DIR="$HOME/.config/charlotte"
UNIT_DIR="$HOME/.config/systemd/user"
step() { printf '\n\e[1;35m==> %s\e[0m\n' "$*"; }
ok()   { printf '  \e[32m✔\e[0m %s\n' "$*"; }
warn() { printf '  \e[33m!\e[0m %s\n' "$*"; }
die()  { printf '  \e[31m✘ %s\e[0m\n' "$*"; exit 1; }

[ "$(id -u)" -ne 0 ] || die "run as your normal user (PipeWire runs per user), not root"
. /etc/os-release
case "${ID:-} ${ID_LIKE:-}" in *debian*) ;; *) die "not a Debian-based system ($PRETTY_NAME)";; esac
[ "${#BT_NAME}" -ge 1 ] && [ "${#BT_NAME}" -le 60 ] || die "--bt-name must be 1-60 characters"

# ---------------------------------------------------------------------------
step "1/8 System packages"
PKGS=(bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth
      libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential
      curl ca-certificates git python3)
missing=()
for p in "${PKGS[@]}"; do
  dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || missing+=("$p")
done
if [ ${#missing[@]} -gt 0 ]; then
  warn "installing: ${missing[*]}"
  sudo apt-get update
  sudo apt-get install -y "${missing[@]}"
fi
ok "packages present"
command -v docker >/dev/null || die "Docker is required (PostgreSQL runs in a container)"
docker info >/dev/null 2>&1 || die "cannot talk to Docker: add $USER to the docker group (sudo usermod -aG docker $USER) and log in again"
systemctl is-active --quiet bluetooth || sudo systemctl enable --now bluetooth
systemctl --user enable --now pipewire wireplumber >/dev/null 2>&1 || true
ok "docker, bluetooth, pipewire ready"

# ---------------------------------------------------------------------------
step "2/8 Toolchains"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
if ! command -v cargo >/dev/null; then
  warn "installing Rust (rustup)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  . "$HOME/.cargo/env"
fi
ok "$(cargo --version)"

if [ -s "$HOME/.nvm/nvm.sh" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.nvm/nvm.sh"
fi
node_major() { node -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo 0; }
if [ "$(node_major)" -lt 20 ]; then
  if command -v nvm >/dev/null; then nvm install 22; else sudo apt-get install -y nodejs npm; fi
fi
[ "$(node_major)" -ge 20 ] || die "Node.js >= 20 is required"
NODE=$(command -v node)
if command -v pnpm >/dev/null; then PNPM=(pnpm); else PNPM=(npx --yes pnpm@10); fi
ok "node $(node --version) ($NODE), pnpm $("${PNPM[@]}" --version)"

# ---------------------------------------------------------------------------
step "3/8 Build"
cd "$ROOT"
cargo build --release -p audio-server
ok "audio-server built"
"${PNPM[@]}" install
"${PNPM[@]}" --filter @charlotte/gateway build
"${PNPM[@]}" --filter @charlotte/web build
# Next.js standalone output needs its static files next to the server.
WEB_STANDALONE="$ROOT/apps/web/.next/standalone"
rm -rf "$WEB_STANDALONE/apps/web/.next/static"
cp -r "$ROOT/apps/web/.next/static" "$WEB_STANDALONE/apps/web/.next/static"
[ -d "$ROOT/apps/web/public" ] && cp -r "$ROOT/apps/web/public" "$WEB_STANDALONE/apps/web/public"
ok "gateway and web built"

# ---------------------------------------------------------------------------
step "4/8 audio-server"
systemctl --user stop audio-server 2>/dev/null || true
"$ROOT/services/audio-server/scripts/install-debian.sh" --no-service >/dev/null
AS_CONF="$CONF_DIR/audio-server.toml"
# Set [bluetooth] name (section-aware, keeps comments).
python3 - "$AS_CONF" "$BT_NAME" <<'PY'
import json, re, sys
path, name = sys.argv[1], sys.argv[2]
lines, out, section, done = open(path).read().splitlines(), [], "", False
for l in lines:
    m = re.match(r"\s*\[([^\]]+)\]", l)
    if m: section = m.group(1).strip()
    if section == "bluetooth" and re.match(r"\s*name\s*=", l):
        l, done = f"name = {json.dumps(name)}", True
    out.append(l)
if not done:
    out = [x for x in out]; idx = next(i for i, x in enumerate(out) if x.strip() == "[bluetooth]")
    out.insert(idx + 1, f"name = {json.dumps(name)}")
open(path, "w").write("\n".join(out) + "\n")
PY
ok "config $AS_CONF (Bluetooth name: $BT_NAME)"

# ---------------------------------------------------------------------------
step "5/8 Bluetooth identity"
CLASS=$(bluetoothctl show 2>/dev/null | awk '/Class:/{print $2; exit}')
if [ "$SKIP_BT_SETUP" = 1 ]; then
  warn "skipped (--skip-bluetooth-setup)"
elif [ -n "$CLASS" ] && [ $(( (CLASS >> 8) & 0x1f )) -eq 4 ] && [ -f /etc/systemd/system/bluetooth.service.d/charlotte.conf ]; then
  ok "already announced as a speaker ($CLASS)"
else
  "$ROOT/services/audio-server/scripts/setup-bluetooth.sh"
fi

# ---------------------------------------------------------------------------
step "6/8 PostgreSQL ($PG_CONTAINER)"
docker inspect "$PG_CONTAINER" >/dev/null 2>&1 || die "container $PG_CONTAINER not found (docker ps -a)"
docker update --restart unless-stopped "$PG_CONTAINER" >/dev/null
docker start "$PG_CONTAINER" >/dev/null
for _ in $(seq 1 30); do docker exec "$PG_CONTAINER" pg_isready -q && break; sleep 1; done
PG_SUPER=$(docker inspect "$PG_CONTAINER" --format '{{range .Config.Env}}{{println .}}{{end}}' | sed -n 's/^POSTGRES_USER=//p')
PG_SUPER=${PG_SUPER:-postgres}
PG_PORT=$(docker port "$PG_CONTAINER" 5432/tcp | head -1 | sed 's/.*://')
[ -n "$PG_PORT" ] || die "port 5432 of $PG_CONTAINER is not published on the host"
psql_super() { docker exec -i "$PG_CONTAINER" psql -v ON_ERROR_STOP=1 -qAt -U "$PG_SUPER" -d postgres "$@"; }

mkdir -p "$CONF_DIR"; chmod 700 "$CONF_DIR"
ENV_FILE="$CONF_DIR/gateway.env"
if [ -f "$ENV_FILE" ] && grep -q '^DATABASE_URL=' "$ENV_FILE"; then
  DB_PASS=$(sed -n 's|^DATABASE_URL=postgres://charlotte:\([^@]*\)@.*|\1|p' "$ENV_FILE")
fi
DB_PASS=${DB_PASS:-$(head -c 24 /dev/urandom | od -An -tx1 | tr -d ' \n')}
if [ "$(psql_super -c "SELECT 1 FROM pg_roles WHERE rolname='charlotte'")" = 1 ]; then
  psql_super -c "ALTER ROLE charlotte WITH LOGIN PASSWORD '$DB_PASS'"
else
  psql_super -c "CREATE ROLE charlotte WITH LOGIN PASSWORD '$DB_PASS'"
fi
if [ "$(psql_super -c "SELECT 1 FROM pg_database WHERE datname='charlotte'")" != 1 ]; then
  psql_super -c "CREATE DATABASE charlotte OWNER charlotte"
fi
ok "database charlotte on 127.0.0.1:$PG_PORT (role charlotte)"

# ---------------------------------------------------------------------------
step "7/8 Gateway"
umask 077
cat > "$ENV_FILE" <<ENV
# Charlotte gateway — generated by scripts/install.sh
DATABASE_URL=postgres://charlotte:$DB_PASS@127.0.0.1:$PG_PORT/charlotte
HOST=127.0.0.1
PORT=$GATEWAY_PORT
AUDIO_SERVER_URL=http://127.0.0.1:8080
COOKIE_SECURE=false
ENV
umask 022
gw_cli() { (set -a; . "$ENV_FILE"; set +a; "$NODE" "$ROOT/apps/gateway/dist/cli.js" "$@"); }
gw_cli migrate
gw_cli set bluetooth.name "$BT_NAME" >/dev/null
if gw_cli has-users; then
  ok "administrator already exists"
elif [ -n "${CHARLOTTE_ADMIN_PASSWORD:-}" ] || [ -t 0 ]; then
  echo "  Create the first administrator: $ADMIN_USER"
  gw_cli create-admin "$ADMIN_USER"
else
  warn "no administrator yet. Create one with:"
  warn "  (set -a; . $ENV_FILE; set +a; node $ROOT/apps/gateway/dist/cli.js create-admin $ADMIN_USER)"
fi

# ---------------------------------------------------------------------------
step "8/8 Services"
mkdir -p "$UNIT_DIR"
install -m644 "$ROOT/services/audio-server/packaging/systemd/audio-server.service" "$UNIT_DIR/audio-server.service"
cat > "$UNIT_DIR/charlotte-gateway.service" <<UNIT
[Unit]
Description=Charlotte gateway (API, accounts)
After=audio-server.service
Wants=audio-server.service

[Service]
EnvironmentFile=%h/.config/charlotte/gateway.env
ExecStart=$NODE $ROOT/apps/gateway/dist/index.js
WorkingDirectory=$ROOT/apps/gateway
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
UNIT
cat > "$UNIT_DIR/charlotte-web.service" <<UNIT
[Unit]
Description=Charlotte web interface
After=charlotte-gateway.service
Wants=charlotte-gateway.service

[Service]
Environment=PORT=$WEB_PORT
Environment=HOSTNAME=0.0.0.0
Environment=NODE_ENV=production
ExecStart=$NODE $WEB_STANDALONE/apps/web/server.js
WorkingDirectory=$WEB_STANDALONE/apps/web
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
UNIT
systemctl --user daemon-reload
# Stop manual runs started outside systemd.
pgrep -x audio-server >/dev/null && pkill -TERM -x audio-server && sleep 1 || true
systemctl --user enable audio-server charlotte-gateway charlotte-web >/dev/null 2>&1
systemctl --user restart audio-server charlotte-gateway charlotte-web
if [ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" != yes ]; then
  loginctl enable-linger "$USER" 2>/dev/null || sudo loginctl enable-linger "$USER" || warn "could not enable linger: services start only after login"
fi
sleep 3
for s in audio-server charlotte-gateway charlotte-web; do
  systemctl --user is-active --quiet "$s" && ok "$s running" || warn "$s not running: journalctl --user -u $s -n 50"
done

IP=$(hostname -I 2>/dev/null | awk '{print $1}')
printf '\n\e[1;32mCharlotte is installed.\e[0m Open http://%s:%s\n' "${IP:-localhost}" "$WEB_PORT"
printf 'Logs: journalctl --user -u audio-server -u charlotte-gateway -u charlotte-web -f\n'
