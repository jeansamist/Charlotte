# audio-server HTTP API

Local API used by the Charlotte gateway. It is served by axum on its own thread, isolated from real-time audio.

- **Base URL:** `http://127.0.0.1:8080` (`server.host`, `server.port`)
- **No authentication.** Keep it bound to `127.0.0.1`. Clients go through the gateway, which requires login and
  proxies these routes under `/api/audio/*` (see [`apps/gateway/docs/API.md`](../../../apps/gateway/docs/API.md)).
- JSON in, JSON out. Errors: `{"error": "message"}` with status 400 (invalid input), 404 (unknown channel or output),
  409 (Bluetooth disabled) or 500.

## Health and metrics
### `GET /api/health`
```json
{ "status": "ok", "version": "0.1.0", "started_at": 1793810575, "uptime_s": 42,
  "cycles": 2800, "xruns": 0, "inputs": 1 }
```
`started_at` (Unix seconds) changes at every restart. The gateway uses it to re-apply settings.

### `GET /api/metrics`
```json
{ "cpu_percent": 1.2, "rss_kib": 10444, "cycles": 2800, "xruns": 0 }
```

## Mixer
### `GET /api/mixer` (alias `GET /api/mixer/channels`)
```json
{
  "master": { "volume": 0.8, "muted": false, "peak": 0.42, "limiter_gain": 1.0, "limiter_enabled": true },
  "ducking": { "enabled": true, "level_db": -18.0 },
  "channels": [
    { "slot": 0, "name": "mac’s MacBook Pro", "node": "bluez_input.DC_A9_04_71_E0_B2.2", "kind": "bluetooth",
      "volume": 1.0, "muted": false, "peak": 0.38, "streaming": true, "active": true,
      "ducked": false, "duck_gain": 1.0,
      "buffered_ms": 64.0, "underruns": 0, "overruns": 0, "drift_skips": 0 }
  ]
}
```
| Field | Meaning |
|---|---|
| `slot` | Mixer input id (stable while the device stays connected) |
| `kind` | `bluetooth` or `simulated` |
| `peak` | Linear peak of the last block, before volume (0–1+) |
| `streaming` | The input delivers audio (not buffering or under-running) |
| `active` | The input is "playing" (above `ducking.threshold_db`) |
| `ducked` / `duck_gain` | Lowered because another input is in the foreground; current ducking gain (1 = full level) |
| `buffered_ms`, `underruns`, `overruns`, `drift_skips` | Buffer health |

Every mixer `PUT` below returns the same document as `GET /api/mixer`.

| Method and path | Body | Rules |
|---|---|---|
| `PUT /api/mixer/channels/{slot}/volume` | `{"volume": 1.2}` | 0–2 |
| `PUT /api/mixer/channels/{slot}/mute` | `{"muted": true}` | |
| `PUT /api/mixer/master/volume` | `{"volume": 0.7}` | 0–1 |
| `PUT /api/mixer/master/mute` | `{"muted": false}` | |
| `PUT /api/mixer/ducking` | `{"enabled": true, "level_db": -20}` | both optional; `level_db` −60–0 |

Volume, mute and ducking changes made through the API are not saved: they reset to the configuration on restart.

## Outputs
The mixer plays on PipeWire's **default sink**. Choosing an output changes the default sink.

### `GET /api/outputs` (alias `GET /api/outputs/current`)
```json
{ "current": "bluez_output.41_42_9A_86_8B_AA.1",
  "outputs": [ { "id": 99, "name": "bluez_output.41_42_9A_86_8B_AA.1", "description": "soundcore R50i", "kind": "bluetooth" },
               { "id": 59, "name": "alsa_output.pci-…HiFi__Speaker__sink", "description": "… Speaker", "kind": "output" } ] }
```
`kind` is `bluetooth`, `output` (physical) or `virtual-output`.

### `PUT /api/outputs/current`
Body `{"name": "<node.name>"}`. Returns the same document as `GET /api/outputs`, and 404 if the name is unknown.

## Bluetooth
All Bluetooth routes return **409** when `bluetooth.enabled = false`.

### `GET /api/bluetooth`
```json
{ "adapter": "hci0", "address": "E4:FD:45:4A:24:54", "name": "Charlotte",
  "class": 1836052, "class_name": "Audio/Video", "audio_class": true,
  "powered": true, "discoverable": false, "discoverable_timeout": 180, "pairable": false,
  "devices": [ { "address": "08:8E:DC:DB:BA:9B", "name": "Ephraim’s iPhone", "icon": "phone",
                 "paired": true, "trusted": true, "connected": true } ] }
```
`audio_class = false` means iPhones will not list the machine (run `scripts/setup-bluetooth.sh`).
Devices are sorted with connected devices first.

| Method and path | Body | Effect |
|---|---|---|
| `PUT /api/bluetooth/name` | `{"name": "Living room"}` | Sets the name other devices see (1–248 bytes) |
| `POST /api/bluetooth/pairing` | `{"seconds": 180}` (30–900, default 180) | Opens the pairing window: discoverable and pairable for that long |
| `DELETE /api/bluetooth/pairing` | – | Closes the pairing window |
| `DELETE /api/bluetooth/devices/{address}` | – | Unpairs and forgets a device (`AA:BB:CC:DD:EE:FF`) |

All four return the same document as `GET /api/bluetooth`.

## Example
```bash
curl -s localhost:8080/api/mixer | python3 -m json.tool
curl -s -X PUT localhost:8080/api/mixer/master/volume -H 'content-type: application/json' -d '{"volume":0.6}'
curl -s -X POST localhost:8080/api/bluetooth/pairing -H 'content-type: application/json' -d '{"seconds":120}'
```
