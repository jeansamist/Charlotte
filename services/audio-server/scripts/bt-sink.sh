#!/usr/bin/env bash
# Makes this machine visible as a Bluetooth speaker for N seconds (default 180)
# so phones/laptops can pair and stream to it. Paired devices are trusted
# automatically, so they can reconnect later without confirmation.
# Usage: scripts/bt-sink.sh [seconds]
set -euo pipefail
SECS=${1:-180}
bluetoothctl power on >/dev/null
echo "Discoverable as '$(bluetoothctl show | awk -F': ' '/Alias/{print $2}')' for ${SECS}s. Pair from your phone now."

# Keep an agent registered (Just Works pairing) while we are discoverable.
{ echo "agent NoInputNoOutput"; echo "default-agent"; echo "pairable on";
  echo "discoverable-timeout $SECS"; echo "discoverable on"; sleep "$SECS"; } \
  | bluetoothctl >/dev/null 2>&1 &
AGENT=$!
trap 'kill $AGENT 2>/dev/null || true' EXIT

end=$((SECONDS + SECS))
while [ $SECONDS -lt $end ]; do
  for mac in $(bluetoothctl devices Paired | awk '{print $2}'); do
    if bluetoothctl info "$mac" | grep -q "Trusted: no"; then
      bluetoothctl trust "$mac" >/dev/null && echo "Trusted $(bluetoothctl info "$mac" | awk -F': ' '/Name/{print $2}') ($mac)"
    fi
  done
  sleep 2
done
bluetoothctl discoverable off >/dev/null
echo "Done. Paired devices:"; bluetoothctl devices Paired
