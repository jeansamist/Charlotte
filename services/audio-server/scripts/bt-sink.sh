#!/usr/bin/env bash
# Opens the Bluetooth pairing window for N seconds (default 180): the machine
# becomes visible (as bluetooth.name, e.g. "Charlotte") and new phones/Macs can pair.
#
# Pairing requests are answered by audio-server's own agent (it must be running):
# devices that pair during the window are trusted and can reconnect any time.
# Usage: scripts/bt-sink.sh [seconds]
set -euo pipefail
SECS=${1:-180}
pgrep -x audio-server >/dev/null || { echo "audio-server is not running: start it first (it answers pairing requests)."; exit 1; }
bluetoothctl power on >/dev/null
bluetoothctl pairable-timeout "$SECS" >/dev/null 2>&1 || true
bluetoothctl pairable on >/dev/null
bluetoothctl discoverable-timeout "$SECS" >/dev/null
bluetoothctl discoverable on >/dev/null
echo "Pairing window open for ${SECS}s: '$(bluetoothctl show | awk -F': ' '/Alias/{print $2}')' is visible. Pair from your phone or Mac now."
