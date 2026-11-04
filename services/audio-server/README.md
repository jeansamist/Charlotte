# audio-server

Charlotte's audio engine. It turns a Debian machine into a **Bluetooth speaker that several phones and computers
can stream to at the same time**. Their audio is mixed in real time and played on one output: the laptop speaker,
or any Bluetooth headphones or speaker.

Written in Rust. BlueZ and PipeWire handle Bluetooth and audio routing. Mixing, control, the pairing agent and the
API are in this crate.

- Design, milestones and device findings: [`docs/PLAN.md`](docs/PLAN.md)
- HTTP API reference: [`docs/API.md`](docs/API.md)
- Scripts: [`scripts/README.md`](scripts/README.md) · Packaging: [`packaging/README.md`](packaging/README.md)

## Features
| Feature | Details |
|---|---|
| Multi-source A2DP sink | Every connected phone or computer becomes a separate mixer input |
| Real-time mixer | Per-input volume (0–200%) and mute, master volume and mute, peak meters, smoothed gain changes (no clicks) |
| Limiter | Peak limiter (instant attack, 80 ms release) and a final hard clip, so the output never distorts |
| **Smart mixing (ducking)** | The device that most recently started playing takes the foreground; the others fade to `level_db` and come back after `hold_ms` of silence |
| Any output | Plays on PipeWire's default sink. With `follow_bluetooth`, any Bluetooth headphones or speaker that connects becomes the output |
| Music profiles only | Captures only A2DP or BAP streams, never hands-free (HFP/HSP) call streams or WirePlumber headset-microphone proxies |
| Pairing agent | Built-in BlueZ agent ("Just Works", like a speaker). New devices are accepted only while the pairing window is open, then trusted |
| Bluetooth identity | Custom name (adapter alias). Warns if the device class is not Audio/Video, which would hide it from iPhones |
| Local HTTP API | Mixer, outputs and Bluetooth control for the gateway, on `127.0.0.1:8080` |
| Test mode | `--simulate N` adds N sine-wave inputs, so the mixer can be tested without devices |

## How it works
```
phone ─A2DP─► BlueZ ─► PipeWire bluez5 node (Audio/Source) ─► capture stream ─► ring buffer ─┐
phone ─A2DP─► BlueZ ─► PipeWire bluez5 node (Audio/Source) ─► capture stream ─► ring buffer ─┤
                                                                                              ▼
                     output stream ◄─ limiter ◄─ master ◄─ Σ (volume × mute × ducking) ◄── engine
                          │
                          └─► PipeWire default sink (speaker or Bluetooth headphones)
```
Threads:
- **Main thread:** the PipeWire main loop (device discovery and hot-plug, streams, status log).
- **PipeWire real-time thread:** capture and output callbacks. They never allocate, lock or block.
- **`api`:** axum on a current-thread Tokio runtime.
- **`bt-agent`:** zbus, the BlueZ pairing agent.

| Module | Role |
|---|---|
| `config` | TOML loading and validation (reports all errors at once) |
| `audio::buffer` | Lock-free per-input ring buffer (`rtrb`), latency prefill, underrun/overrun counters, clock-drift correction |
| `audio::mixer` | Mixing primitives, atomic per-channel controls |
| `audio::limiter` | Peak limiter and hard clip |
| `audio::engine` | Real-time mixer: inputs, smoothing, ducking, master, limiter. `EngineHandle` is the control side |
| `audio::input` / `output` / `format` | PipeWire capture and playback streams, F32 / 48 kHz / stereo format negotiation |
| `audio::sim` | Simulated sine inputs |
| `devices` | Classifies PipeWire nodes (Bluetooth source, output, own nodes…) |
| `bluetooth` | BlueZ D-Bus: identity, status, pairing window, forget device. `bluetooth::agent` is the pairing agent |
| `outputs` | Reads and changes the default sink (`wpctl`) |
| `api` / `shared` | HTTP API and the state it shares with the main thread |
| `monitoring` | Process CPU and memory |
| `app` | Wires everything together |

## Requirements (Debian, x86_64)
- Runtime: `bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth`
- Build: `libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential`, Rust ≥ 1.85
- The WirePlumber rule [`packaging/wireplumber/51-charlotte-bluez.conf`](packaging/wireplumber/51-charlotte-bluez.conf)
  (A2DP only, phones as sources). Without it, phones play straight to the speaker or connect in call mode.
- For iPhone visibility: `scripts/setup-bluetooth.sh` (sudo, once).

Check with `scripts/check-dependencies.sh --build`. The repository's `scripts/install.sh` does all of this.

## Build, test, run
```bash
cargo test  -p audio-server                    # from the repository root
cargo build -p audio-server --release          # → target/release/audio-server

audio-server run                               # default command
audio-server run --simulate 2 --no-bluetooth   # two test tones
audio-server run --output <node.name>          # fixed output instead of the default sink
audio-server list-nodes                        # audio nodes and their classification
audio-server check-config                      # validate and print the configuration
```
Global options: `--config <file>`, `--log-level <level>` (`RUST_LOG` also works).
Options of `run`: `--simulate N`, `--no-bluetooth`, `--output NAME`, `--status-interval SECS` (default 5).

Without `--config`, the first existing file is used from: `/etc/charlotte/audio-server.toml`, `config/default.toml`,
`services/audio-server/config/default.toml`. If none exists, built-in defaults apply. The installed service uses
`~/.config/charlotte/audio-server.toml`.

## Configuration
Reference file: [`config/default.toml`](config/default.toml). Unknown keys are rejected.

| Key | Default | Meaning |
|---|---|---|
| `server.api_enabled` | `true` | Serve the local HTTP API |
| `server.host` / `server.port` | `127.0.0.1` / `8080` | API address. **No authentication:** keep it local |
| `audio.sample_rate` / `channels` / `format` | `48000` / `2` / `"f32"` | Internal format (only these values for now). PipeWire converts every stream |
| `audio.buffer_size` | `512` | PipeWire quantum requested (frames, power of two, 64–8192) |
| `audio.input_latency_ms` | `60` | Audio buffered per input before mixing (absorbs Bluetooth jitter) |
| `audio.max_inputs` | `16` | Simultaneous inputs |
| `audio.master_volume` | `0.8` | Initial master volume (0–1) |
| `audio.limiter_enabled` / `limiter_threshold` | `true` / `0.95` | Output limiter |
| `bluetooth.enabled` | `true` | Bluetooth features (identity, agent, capture) |
| `bluetooth.name` | `"Charlotte"` | Name other devices see. The gateway overrides it from its settings |
| `bluetooth.adapter` | `"hci0"` | BlueZ adapter |
| `bluetooth.auto_accept_trusted_devices` | `true` | Trusted or paired devices may always connect |
| `bluetooth.auto_connect_sources` | `true` | Add every Bluetooth source to the mixer automatically |
| `output.device` | `"default"` | `"default"` follows PipeWire's default sink (changeable from the GUI), or a fixed `node.name` |
| `output.follow_bluetooth` | `true` | With `"default"`: switch to any Bluetooth headphones or speaker that connects |
| `ducking.enabled` | `true` | Smart mixing |
| `ducking.level_db` | `-18` | Level of the other inputs while one is in the foreground |
| `ducking.threshold_db` | `-50` | An input "plays" above this RMS level |
| `ducking.hold_ms` | `2000` | Silence before an input gives the foreground back |
| `ducking.attack_ms` / `release_ms` | `300` / `1500` | Fade-down and fade-up times |
| `logging.level` | `"info"` | `trace`, `debug`, `info`, `warn`, `error` |

## Limits (known)
- Only 48 kHz / stereo / f32 internally.
- Clock drift is corrected gently: when an input's *average* buffer level leaves the latency target ± 25%, one frame
  per block is removed (averaged with its neighbour) or repeated, a speed change of about 0.2% that cannot be heard.
  Only after a stall (more than 4× the target queued) does the buffer jump back, counted in `drift_skips`.
- Inputs are resampled by PipeWire (44.1 → 48 kHz) at `resample.quality = 10`.
- Codecs: SBC from phones and computers. AAC needs PipeWire built with libfdk-aac, which Debian does not ship.
  SBC-XQ is enabled towards headphones that accept it.
- Several A2DP sources at once depend on the Bluetooth adapter (Intel AX201: tested with a Mac, an iPhone and earbuds).
- Changing `output.device` to a fixed node needs a restart; the GUI changes the default sink instead.
