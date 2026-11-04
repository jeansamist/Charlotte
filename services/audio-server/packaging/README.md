# audio-server packaging

Files installed on the target machine, usually by `scripts/install-debian.sh` or the repository's `scripts/install.sh`.

## `systemd/audio-server.service`
A **user** service, because PipeWire runs per user and the mixer must be in the same session.
- Installed to `~/.config/systemd/user/audio-server.service`.
- Runs `~/.local/bin/audio-server --config ~/.config/charlotte/audio-server.toml run`.
- Starts after `pipewire` and `wireplumber`, and restarts on failure (2 s delay).
- For start-up at boot without a login: `loginctl enable-linger $USER` (the installer does it).
- Real-time priority for the audio callbacks comes from PipeWire (RTKit / rlimits), not from the unit.

```bash
systemctl --user daemon-reload && systemctl --user enable --now audio-server
journalctl --user -u audio-server -f
```

## `wireplumber/51-charlotte-bluez.conf`
WirePlumber 0.5+ configuration, installed to `~/.config/wireplumber/wireplumber.conf.d/`. It has three parts:

1. **Music roles only:** `bluez5.roles = [ a2dp_sink a2dp_source bap_sink bap_source ]`, `bluez5.hfphsp-backend = "none"`.
   Without this, Debian also offers the hands-free and headset roles. A Mac or iPhone may then pick the call profile
   (mSBC, 16 kHz mono), and the sound becomes choppy.
2. **No automatic switch to the headset profile:** `bluetooth.autoswitch-to-headset-profile = false`.
3. **Phones as sources:** `bluez5.media-source-role = "input"` for `bluez_input.*` nodes. By default PipeWire plays
   a phone's audio straight to the speaker. With `input`, each phone becomes an `Audio/Source` that the mixer captures.

Apply it with `systemctl --user restart wireplumber`, then reconnect the Bluetooth devices.

Note that this disables the microphone of Bluetooth headsets (HFP) on this machine. That is intended for a music
speaker.
