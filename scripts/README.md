# scripts

## `install.sh`: install or update Charlotte on Debian
Installs everything Charlotte needs for the **current user**, and is safe to re-run: each step is skipped when it
is already done. Run it again after `git pull` to update.

```bash
bash scripts/install.sh [--bt-name NAME] [--admin USERNAME] [--pg-container NAME]
                        [--web-port PORT] [--skip-bluetooth-setup]
```
| Option | Default | Meaning |
|---|---|---|
| `--bt-name` | `Charlotte` | Bluetooth name shown to phones and computers (1–60 characters). Saved in the audio-server config and in the database |
| `--admin` | `admin` | Username of the first administrator |
| `--pg-container` | `postgres_database` | Existing PostgreSQL Docker container to use |
| `--web-port` | `3000` | Port of the web interface |
| `--skip-bluetooth-setup` | – | Do not touch `/etc/bluetooth` (speaker class) |

Environment: `CHARLOTTE_ADMIN_PASSWORD` creates the administrator without a prompt. Otherwise the script asks for the
password if a terminal is available, or prints the command to run later.

### Steps
| # | Step | Details |
|---|---|---|
| 1 | System packages | Installs any missing package from: `bluez pipewire pipewire-audio wireplumber libspa-0.2-bluetooth libpipewire-0.3-dev libspa-0.2-dev libclang-dev pkgconf build-essential curl ca-certificates git python3` (sudo). Checks Docker access, enables bluetooth, pipewire and wireplumber |
| 2 | Toolchains | Rust via rustup if `cargo` is missing. Node ≥ 20 (uses nvm if present, otherwise apt). pnpm (or `npx pnpm@10`) |
| 3 | Build | `cargo build --release -p audio-server`, `pnpm install`, gateway and web builds, copies Next.js static files into the standalone output |
| 4 | audio-server | Runs `services/audio-server/scripts/install-debian.sh --no-service` (binary, config, WirePlumber rule), then writes `[bluetooth] name` in `~/.config/charlotte/audio-server.toml` |
| 5 | Bluetooth identity | If the adapter class is not Audio/Video or the systemd drop-in is missing, runs `setup-bluetooth.sh` (sudo) |
| 6 | PostgreSQL | Starts the container and sets `--restart unless-stopped`. Creates (or updates the password of) the role `charlotte` and creates the database `charlotte`. The password is generated once and kept |
| 7 | Gateway | Writes `~/.config/charlotte/gateway.env` (chmod 600), runs migrations, saves the Bluetooth name setting, creates the first administrator if there is none |
| 8 | Services | Writes and enables the user units `audio-server`, `charlotte-gateway`, `charlotte-web`, restarts them, enables linger (start at boot), prints the URL |

Requirements: Debian-based system, a normal user (not root) with sudo, Docker with an existing PostgreSQL container
whose port 5432 is published, and a Bluetooth adapter.

Component-specific scripts are documented in [`services/audio-server/scripts/README.md`](../services/audio-server/scripts/README.md).
