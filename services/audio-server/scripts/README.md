# audio-server scripts

Run them from anywhere: they locate the crate themselves. None of them should be run as root; they call `sudo`
only for the commands that need it.

| Script | Sudo | What it does |
|---|---|---|
| [`check-dependencies.sh`](check-dependencies.sh) `[--build]` | no | Read-only report: OS, glibc, packages (and build packages with `--build`), services, Bluetooth adapter (rfkill, A2DP sink profile), WirePlumber rule, linger. Exit code 1 if something required is missing. |
| [`install-debian.sh`](install-debian.sh) `[--build-deps] [--no-service]` | if packages are missing | Installs the audio-server for the current user: missing apt packages, binary → `~/.local/bin/audio-server`, config → `~/.config/charlotte/audio-server.toml` (never overwritten), WirePlumber rule, user service. Needs `target/release/audio-server` to be built first. |
| [`setup-bluetooth.sh`](setup-bluetooth.sh) | yes | One-time system setup so Macs and iPhones see the machine as a **speaker**. Sets `Class = 0x240414` in `/etc/bluetooth/main.conf` (backup in `main.conf.charlotte.bak`), adds a systemd drop-in that starts `bluetoothd --noplugin=hostname` (that plugin would reset the class to *Computer/Laptop*), then restarts bluetooth. |
| [`bt-sink.sh`](bt-sink.sh) `[seconds]` | no | Opens the pairing window (default 180 s). It needs a running audio-server, whose agent answers pairing requests. Same as *Open pairing* in the GUI. |
| [`build-release.sh`](build-release.sh) | no | `cargo build --release`, then prints the binary size, its shared-library dependencies and the minimum glibc version. |
| [`uninstall.sh`](uninstall.sh) `[--purge]` | no | Removes the user service, the binary and the WirePlumber rule (`--purge` also deletes the config). Prints the commands that undo the `setup-bluetooth.sh` system changes. |

The repository-wide installer [`scripts/install.sh`](../../../scripts/README.md) calls `install-debian.sh` and
`setup-bluetooth.sh` for you.
