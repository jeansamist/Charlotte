#!/usr/bin/env bash
# Checks runtime (and optionally build) dependencies. Changes nothing.
# Usage: scripts/check-dependencies.sh [--build]
set -u
ok=0
pass() { printf '  \e[32m✔\e[0m %s\n' "$1"; }
fail() { printf '  \e[31m✘\e[0m %s\n' "$1"; ok=1; }
warn() { printf '  \e[33m!\e[0m %s\n' "$1"; }
pkg()  { dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q "install ok installed"; }

echo "System"
[ "$(uname -m)" = x86_64 ] && pass "arch x86_64" || warn "arch $(uname -m) (only x86_64 is tested)"
if [ -r /etc/os-release ]; then . /etc/os-release; pass "OS: $PRETTY_NAME"; fi
pass "glibc $(ldd --version 2>/dev/null | head -1 | awk '{print $NF}')"

echo "Runtime packages"
for p in bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth; do
  pkg "$p" && pass "$p" || fail "$p missing → sudo apt install $p"
done

if [ "${1:-}" = "--build" ]; then
  echo "Build packages"
  for p in libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential; do
    pkg "$p" && pass "$p" || fail "$p missing → sudo apt install $p"
  done
  command -v cargo >/dev/null && pass "cargo $(cargo --version | awk '{print $2}')" || fail "cargo missing → https://rustup.rs"
fi

echo "Services"
systemctl is-active --quiet bluetooth && pass "bluetooth.service active" || fail "bluetooth.service not active → sudo systemctl enable --now bluetooth"
for s in pipewire wireplumber; do
  systemctl --user is-active --quiet "$s" && pass "$s (user) active" || fail "$s (user) not active → systemctl --user enable --now $s"
done

echo "Bluetooth adapter"
if [ -d /sys/class/bluetooth ] && ls /sys/class/bluetooth | grep -q hci; then
  for r in /sys/class/rfkill/rfkill*; do
    [ "$(cat "$r/type")" = bluetooth ] || continue
    [ "$(cat "$r/soft")" = 0 ] && [ "$(cat "$r/hard")" = 0 ] && pass "not blocked (rfkill)" || fail "blocked by rfkill → sudo rfkill unblock bluetooth"
  done
  bluetoothctl show 2>/dev/null | grep -q "Audio Sink" && pass "A2DP Audio Sink profile registered" || fail "A2DP sink profile not registered (is libspa-0.2-bluetooth installed and wireplumber running?)"
else
  fail "no Bluetooth adapter found"
fi

echo "Configuration"
f="$HOME/.config/wireplumber/wireplumber.conf.d/51-charlotte-bluez.conf"
[ -f "$f" ] && pass "WirePlumber rule installed" || warn "WirePlumber rule not installed: phones will play directly to speakers ($f)"
[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" = yes ] && pass "linger enabled (starts at boot)" || warn "linger disabled: service starts only after login → loginctl enable-linger $USER"

exit $ok
