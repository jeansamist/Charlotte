#!/usr/bin/env bash
# Installs the Charlotte audio-server for the current user on Debian.
# - installs missing apt packages (asks for sudo only if something is missing)
# - installs binary, config, WirePlumber rule, systemd user service
# Non-destructive: never overwrites an existing config file.
# Usage: scripts/install-debian.sh [--build-deps] [--no-service]
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)

[ "$(id -u)" -ne 0 ] || { echo "Run as your normal user (PipeWire is per user), not root."; exit 1; }
. /etc/os-release
case "${ID:-}${ID_LIKE:-}" in *debian*) ;; *) echo "Not a Debian-based system ($PRETTY_NAME)"; exit 1;; esac

want=(bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth)
[ "${1:-}" = "--build-deps" ] && want+=(libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential)
missing=()
for p in "${want[@]}"; do dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || missing+=("$p"); done
if [ ${#missing[@]} -gt 0 ]; then
  echo "Installing: ${missing[*]}"
  sudo apt-get install -y "${missing[@]}"
fi

systemctl is-active --quiet bluetooth || sudo systemctl enable --now bluetooth
systemctl --user enable --now pipewire wireplumber >/dev/null

BIN="$ROOT/target/release/audio-server"
[ -x "$BIN" ] || { echo "Build first: cargo build --release -p audio-server"; exit 1; }
install -Dm755 "$BIN" "$HOME/.local/bin/audio-server"

CFG="$HOME/.config/charlotte/audio-server.toml"
if [ -f "$CFG" ]; then echo "Keeping existing $CFG"; else install -Dm644 config/default.toml "$CFG"; fi

WP="$HOME/.config/wireplumber/wireplumber.conf.d/51-charlotte-bluez.conf"
install -Dm644 packaging/wireplumber/51-charlotte-bluez.conf "$WP"
systemctl --user restart wireplumber

if [ "${1:-}" != "--no-service" ] && [ "${2:-}" != "--no-service" ]; then
  install -Dm644 packaging/systemd/audio-server.service "$HOME/.config/systemd/user/audio-server.service"
  systemctl --user daemon-reload
  systemctl --user enable --now audio-server
  echo "Service started: journalctl --user -u audio-server -f"
fi

[ "$(loginctl show-user "$USER" -p Linger --value)" = yes ] || \
  echo "Tip: start at boot without login → loginctl enable-linger $USER"
scripts/check-dependencies.sh || true
