# audio-server

Charlotte's audio mixer. Turns a Debian machine into a Bluetooth speaker that several phones or
laptops can stream to **at the same time**. Their audio is mixed in real time and played on one output.

Written in Rust. Bluetooth is handled by BlueZ and PipeWire. Mixing, control and monitoring are in Rust.
See [`docs/PLAN.md`](docs/PLAN.md) for the design and milestones.

**Status:** milestone 1 (proof of concept). There is no HTTP API yet.

## Requirements (Debian, x86_64)
- Runtime: `bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth`
- Build: `libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential` and Rust ≥ 1.85

Check with `scripts/check-dependencies.sh --build`.

## Build and test
```bash
cargo test  -p audio-server
cargo build -p audio-server --release      # → target/release/audio-server (repo root)
```

## Use
```bash
audio-server list-nodes                                  # audio nodes and how they are classified
audio-server run --simulate 2 --no-bluetooth             # two test tones, no Bluetooth
audio-server run                                         # mix every connected Bluetooth source
audio-server run --output alsa_output.pci-…analog-stereo # choose the output
audio-server check-config
```

Pair devices: `scripts/bt-sink.sh 180` makes the machine discoverable for 180 s and trusts what pairs.

Phone audio goes to the mixer only if the WirePlumber rule is installed
(`packaging/wireplumber/51-charlotte-bluez.conf`). Without it, PipeWire plays each phone directly to
the speakers, and the mixer logs a warning.

## Install as a service
```bash
scripts/install-debian.sh          # binary → ~/.local/bin, config → ~/.config/charlotte, user service
journalctl --user -u audio-server -f
scripts/uninstall.sh [--purge]
```

## Configuration
See [`config/default.toml`](config/default.toml). Milestone 1 supports only 48 kHz / stereo / f32
internally. PipeWire converts every input to that format.
