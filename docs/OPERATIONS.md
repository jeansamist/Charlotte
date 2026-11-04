# Operations

Running Charlotte on the Debian machine: services, logs, updates, backups and troubleshooting.

## Services
Everything runs as **systemd user services** of the installing user, because PipeWire runs per user.
*Linger* is enabled, so the services start at boot without a login.

| Service | What | Listens on | Config |
|---|---|---|---|
| `audio-server` | Bluetooth mixer + local API | `127.0.0.1:8080` (no auth) | `~/.config/charlotte/audio-server.toml` |
| `charlotte-gateway` | API, accounts, settings | `127.0.0.1:4000` | `~/.config/charlotte/gateway.env` (secret, chmod 600) |
| `charlotte-web` | Web interface | `0.0.0.0:3000` (LAN) | unit file environment |
| PostgreSQL | Docker container `postgres_database` | `127.0.0.1`/host `:5432` | database and role `charlotte` |

Unit files: `~/.config/systemd/user/{audio-server,charlotte-gateway,charlotte-web}.service`.
The WirePlumber rule is in `~/.config/wireplumber/wireplumber.conf.d/51-charlotte-bluez.conf`.
The system-level Bluetooth setup (sudo) is in `/etc/bluetooth/main.conf` (`Class = 0x240414`) and
`/etc/systemd/system/bluetooth.service.d/charlotte.conf` (`--noplugin=hostname`).

```bash
systemctl --user status audio-server charlotte-gateway charlotte-web
systemctl --user restart charlotte-gateway
journalctl --user -u audio-server -u charlotte-gateway -u charlotte-web -f
```

## Updating
```bash
cd ~/Charlotte && git pull && bash scripts/install.sh
```
The installer is idempotent. It rebuilds everything, applies new migrations, keeps existing configs and the database
password, and restarts the services.

## Accounts
```bash
cd ~/Charlotte && source ~/.nvm/nvm.sh
gw() { (set -a; . ~/.config/charlotte/gateway.env; set +a; node apps/gateway/dist/cli.js "$@"); }
gw create-admin admin            # create (or promote and reset) an administrator
gw reset-password alice          # also signs the user out everywhere
gw set bluetooth.name "Salon"    # change a setting from the command line
```

## Backups
- **Database:**
  `docker exec postgres_database pg_dump -U root -d charlotte -Fc > charlotte-$(date +%F).dump`
  Restore with `docker exec -i postgres_database pg_restore -U root -d charlotte --clean < file.dump`.
- **Configuration:** `~/.config/charlotte/` (contains the database password).
- Bluetooth pairings are stored by BlueZ in `/var/lib/bluetooth/` (root).

## Security notes
- Only the web interface (port 3000) is reachable from the network. It is plain **HTTP**, so use it only on a trusted LAN.
  For remote access, put it behind HTTPS (reverse proxy) and set `COOKIE_SECURE=true` in `gateway.env`.
- Passwords are hashed with Argon2id. Sessions are random tokens, stored hashed (SHA-256), and expire after 30 days
  of inactivity. Logins are limited to 10 failed attempts per 5 minutes per IP.
- The audio-server API has **no authentication**: keep `server.host = "127.0.0.1"`.
- New Bluetooth devices can pair only while the pairing window is open (GUI → Bluetooth → *Open pairing*).

## Troubleshooting
| Symptom | Cause | Fix |
|---|---|---|
| iPhone does not list "Charlotte" | Bluetooth class is *Computer* (iOS hides computers), or pairing window closed | GUI → Bluetooth shows a warning if the class is wrong: run `services/audio-server/scripts/setup-bluetooth.sh`. Then open pairing. |
| iPhone paired but Charlotte is not an audio output | Device not trusted (fixed in code); old pairing | Forget the device on both sides, then pair again during the pairing window |
| Sound from the Mac is choppy / sounds like a phone call | Device connected with the hands-free profile (HFP, mSBC 16 kHz) | Make sure the WirePlumber rule is installed (A2DP only), restart wireplumber, disconnect and reconnect the device |
| Phone audio plays on the speaker but not in the mixer | WirePlumber rule missing (`media-source-role = input`) | `bash scripts/install.sh` or copy the rule, `systemctl --user restart wireplumber` |
| Log: "ignoring non-music Bluetooth stream (call profile)" | Same as the HFP row | Same as the HFP row |
| GUI says "the audio server is not running" | `audio-server` stopped or crashed | `journalctl --user -u audio-server -n 100` |
| Log: "output is not running (no audio cycles)" | Output device gone or suspended | Choose another output in the GUI, or reconnect the headphones |
| Login always fails | No administrator yet | `gw create-admin admin` (see Accounts) |
| Gateway does not start | Database unreachable | `docker start postgres_database`, then check `DATABASE_URL` in `gateway.env` |
| Services stop when you log out | Linger disabled | `loginctl enable-linger $USER` |

## Uninstall
```bash
systemctl --user disable --now charlotte-web charlotte-gateway audio-server
rm ~/.config/systemd/user/charlotte-{web,gateway}.service
services/audio-server/scripts/uninstall.sh            # audio-server + WirePlumber rule (prints how to undo the BT system setup)
docker exec postgres_database psql -U root -d postgres -c "DROP DATABASE charlotte" -c "DROP ROLE charlotte"   # optional
rm -rf ~/.config/charlotte
```
