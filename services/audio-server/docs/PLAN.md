# audio-server — Technical implementation plan

Spec: multi-source Bluetooth audio mixer in Rust (PipeWire + BlueZ), headless, Debian x86_64.
Work is incremental. Each milestone must work on the Debian box before the next starts.

## Target machine (verified 2026-11-04)
| Item | Value |
|---|---|
| OS | Debian forky/sid, kernel 7.1.3, glibc 2.42, x86_64 |
| Audio | PipeWire 1.6.7 + WirePlumber 0.5.15 (user services), Intel Alder Lake HDA (speaker, HDMI), snd_aloop loopback |
| Bluetooth | Intel AX201 (USB 8087:0026), BlueZ 5.85. Adapter advertises the **Audio Sink** UUID (A2DP sink provided by PipeWire's bluez5 plugin). |
| Rust | 1.96 |
| Missing for build | `libpipewire-0.3-dev`, `libspa-0.2-dev`, `libclang-dev` (bindgen) |

## Architecture
```
 phone A ──A2DP──┐                       PipeWire graph
 phone B ──A2DP──┤ BlueZ ─► bluez5 plugin ─► bluez_input.A (Audio/Source) ─► capture stream 0 ─┐
 laptop  ──A2DP──┘                          bluez_input.B (Audio/Source) ─► capture stream 1 ─┤
                                                                                              │ (PipeWire adapter converts
                                                                                              │  each to f32 / 48 kHz / stereo)
         ┌────────────────────── audio-server process ─────────────────────────────────────────┘
         │  RT data thread:  capture cb ─► rtrb ring buffer (per input, lock-free)
         │                   output cb  ─► Engine: read all buffers → gain/mute → sum
         │                                 → master gain → limiter → hard clip
         │  main thread:     registry listener (hot-plug), status timer, EngineHandle
         └──► playback stream "charlotte-mixer-out" ─► speaker / configured sink
```

Key decisions:
1. **BlueZ + PipeWire do Bluetooth.** We do not touch the A2DP protocol. PipeWire's bluez5 plugin already
   registers the A2DP sink endpoint. Each connected device becomes a PipeWire node.
2. **`bluez5.media-source-role = input`** (WirePlumber rule in `packaging/wireplumber/`). The default
   (`playback`) sends phone audio straight to the speakers. With `input`, each phone is an `Audio/Source`
   node the mixer captures. Documented in `pipewire-props(7)` on the target machine.
3. **Format conversion is PipeWire's job.** Each stream asks for F32LE / 48 kHz / stereo, so PipeWire's
   adapter resamples (SBC 44.1 kHz → 48 kHz) and remaps channels before data reaches us. `rubato` is
   only needed later if we do our own adaptive drift resampling.
4. **Real-time rules.** The output callback never allocates, locks or blocks:
   - per-input lock-free SPSC buffers (`rtrb`);
   - gain and mute are atomics;
   - adding or removing inputs goes through a command queue;
   - removed inputs are sent back to the main thread to be freed.
5. **Clock drift (first version).** Each input keeps a latency target (`input_latency_ms`). If the buffer
   grows past 2× the target, old samples are dropped. If it runs dry, the input re-buffers. Adaptive
   resampling is evaluated in M3 if drops are audible.
6. **Feedback-loop protection.** Our nodes (`charlotte-mixer-*`, `application.name`) are never captured.
   Only `bluez5` sources are captured automatically.

## Milestones
| # | Goal | Status |
|---|---|---|
| **M1** | **PoC**: A2DP sink, 2 simultaneous phones, each captured in Rust, mixed, played on the laptop speaker. Simulated inputs. | Code written, waiting for build deps + device test |
| M2 | HTTP + WebSocket API (axum/tokio on its own thread, talks to `EngineHandle` via channels). Output switching at runtime. | – |
| M3 | Robustness: stream recovery, backend restart (PipeWire restart → reconnect), adaptive drift correction, benchmarks 2/4/6/10 sources. | – |
| M4 | Bluetooth manager over D-Bus (`zbus`): discovery, pairing agent, trust list, auto-reconnect, adapter capability report. | – |
| M5 | Packaging: .deb, NATS events for Charlotte, `skills/audio` MCP server. | – |

## M1 — module responsibilities
| Module | Responsibility | Tech | Interfaces |
|---|---|---|---|
| `config` | Load and validate TOML. Reports all errors at once. | serde, toml | `config::load()` → `Config` |
| `audio::buffer` | Per-input ring buffer, prefill/latency, underrun/overrun/drift counters | rtrb | `InputProducer::push`, `InputConsumer::read` |
| `audio::mixer` | Sum, gain, peak, sanitize, silence detection, atomic controls | std atomics | `ChannelControl` |
| `audio::limiter` | Peak limiter (instant attack, exponential release) and final hard clip | – | `Limiter::process` |
| `audio::engine` | Real-time mixer + control handle | – | `new_engine()` → (`EngineHandle`, `Engine`) |
| `audio::input` / `output` | PipeWire capture and playback streams | pipewire 0.10 | `connect_capture`, `connect_playback` |
| `audio::sim` | Sine-wave inputs for tests and benchmarks | std thread | `SimSource::spawn` |
| `devices` | Classify PipeWire nodes (BT source, output, own…) | – | `classify()` |
| `app` | Main loop, hot-plug, status report, graceful shutdown (SIGINT/SIGTERM) | pipewire | `run()`, `list_nodes()` |

## M1 validation on Debian
```bash
# 0. one-time: build deps
sudo apt install libpipewire-0.3-dev libspa-0.2-dev libclang-dev
services/audio-server/scripts/check-dependencies.sh --build

# 1. unit tests + build
cargo test -p audio-server
cargo build --release -p audio-server

# 2. mixer without Bluetooth: 2 tones on the speaker
./target/release/audio-server list-nodes
./target/release/audio-server run --simulate 2 --no-bluetooth --output <alsa speaker node.name>

# 3. Bluetooth: install WirePlumber rule, pair 2 phones, play music on both
mkdir -p ~/.config/wireplumber/wireplumber.conf.d
cp services/audio-server/packaging/wireplumber/51-charlotte-bluez.conf ~/.config/wireplumber/wireplumber.conf.d/
systemctl --user restart wireplumber
services/audio-server/scripts/bt-sink.sh 180      # pair phone A, then phone B
./target/release/audio-server list-nodes          # expect 2× bluetooth-source
./target/release/audio-server run                 # expect "Bluetooth source connected → mixer input" ×2
```
Pass criteria: both phones audible at the same time on the speaker. Status shows 2 inputs `streaming=true`,
underruns not growing, CPU < 5% of one core.

## Findings from device tests (2026-11-04)
- MacBook → Debian (A2DP, SBC) → mixer → soundcore R50i earbuds: **works**. Audio is routed correctly.
- WirePlumber `bluez5.loopback` headset-mic proxies must be ignored (see `app.rs`).
- **Class of Device:** iOS hides devices whose class is Computer. `Class=` in main.conf is overwritten
  by BlueZ's *hostname* plugin (chassis = laptop). Fix: `bluetoothd --noplugin=hostname` (systemd drop-in in
  `scripts/setup-bluetooth.sh`).
- A process started with `&` from a non-interactive shell ignores SIGINT. Stop the mixer with SIGTERM
  (systemd does this).

## Risks / unknowns
- **Several A2DP sources on one AX201.** BlueZ allows several connected devices. Bandwidth for 2 SBC/AAC
  streams is usually fine on Intel combo chips, but Wi-Fi coexistence can cause dropouts. Must be measured.
  We will not claim more than we test.
- **WirePlumber rule scope.** The `media-source-role` node property should apply via
  `monitor.bluez.rules`. If not, fallback: per-device `bluez5.media-source-role` through `pw-metadata`, or a
  loopback per device.
- **Headless.** WirePlumber's `seat-monitoring` disables Bluetooth when the user has no active seat. For
  boot without login, linger + `monitor.bluez.seat-monitoring = disabled` may be needed (M5).
- **Disk.** The dev box has about 3 GB free. The dev profile uses `debug = "line-tables-only"`.
