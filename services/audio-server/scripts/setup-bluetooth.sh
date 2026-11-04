#!/usr/bin/env bash
# One-time Bluetooth setup (needs sudo): make this machine appear as a SPEAKER
# (Class of Device = Audio/Video → Loudspeaker) instead of a laptop, so Macs and
# phones offer it as a sound output.
#
# - backs up /etc/bluetooth/main.conf once (main.conf.charlotte.bak)
# - sets "Class = 0x240414" in [General] (BlueZ keeps only the major/minor bits;
#   the Audio/Rendering service bits come from the A2DP sink profile)
# - restarts bluetooth (connected devices reconnect)
# The device NAME is set by audio-server itself from bluetooth.name in its config.
set -euo pipefail
CONF=/etc/bluetooth/main.conf
CLASS=0x240414

[ -f "$CONF" ] || { echo "$CONF not found"; exit 1; }
[ -f "$CONF.charlotte.bak" ] || sudo cp -p "$CONF" "$CONF.charlotte.bak"

if grep -qE '^\s*Class\s*=' "$CONF"; then
  sudo sed -i -E "s|^\s*Class\s*=.*|Class = $CLASS|" "$CONF"
elif grep -qE '^\s*#\s*Class\s*=' "$CONF"; then
  sudo sed -i -E "0,/^\s*#\s*Class\s*=.*/s||Class = $CLASS|" "$CONF"
else
  sudo sed -i "/^\[General\]/a Class = $CLASS" "$CONF"
fi
echo "main.conf: $(grep -E '^Class' "$CONF")"

sudo systemctl restart bluetooth
sleep 2
bluetoothctl power on >/dev/null
bluetoothctl show | grep -E "Alias|Class"
echo "Done. On the Mac: if this machine was already listed, remove it (Forget) and scan again."
