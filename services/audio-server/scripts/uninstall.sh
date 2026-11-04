#!/usr/bin/env bash
# Removes the audio-server user installation. Keeps your config unless --purge.
set -euo pipefail
systemctl --user disable --now audio-server 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/audio-server.service" "$HOME/.local/bin/audio-server"
systemctl --user daemon-reload
rm -f "$HOME/.config/wireplumber/wireplumber.conf.d/51-charlotte-bluez.conf"
systemctl --user restart wireplumber || true
[ "${1:-}" = "--purge" ] && rm -rf "$HOME/.config/charlotte/audio-server.toml"
echo "Bluetooth system setup (if done) is kept. To undo it:"
echo "  sudo rm /etc/systemd/system/bluetooth.service.d/charlotte.conf && sudo cp /etc/bluetooth/main.conf.charlotte.bak /etc/bluetooth/main.conf && sudo systemctl daemon-reload && sudo systemctl restart bluetooth"
echo "audio-server removed (system packages were left installed)."
