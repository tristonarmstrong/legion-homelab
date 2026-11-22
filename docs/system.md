# System setup

Changes made to the Legion beyond installing the services. Copies of the
root-owned files are in `live/system/`.

## Battery capped at about 80%

The firmware only offers `Standard` or `Long_Life` (about 80%); there is no
70% setting. Long_Life is on, and a udev rule keeps it on after reboots:

```text
/etc/udev/rules.d/90-battery-long-life.rules
ACTION=="add|change", SUBSYSTEM=="power_supply", KERNEL=="BATT", ATTR{charge_types}="Long_Life"
```

Check: `cat /sys/class/power_supply/BATT/charge_types` shows `[Long_Life]`.

## Never sleeps, never locks

- System sleep is off with Omarchy's own file,
  `/etc/systemd/sleep.conf.d/omarchy-sleep-disabled.conf`, so Omarchy's menu
  hides Suspend too.
- The screensaver is off (`omarchy-toggle screensaver-off`).
- The idle lock is off: `"idle": {"lock": 0}` in
  `~/.config/omarchy/shell.json`. On Omarchy the screen only turns off as part
  of locking, so the screen stays on, which suits the face. There is no
  keyboard attached, so an unlocked touchscreen exposes little.
- `loginctl enable-linger legion` keeps user services running without a login.

## Microphone levels

The built-in mic shipped with both boosts at maximum ("Internal Mic Boost" 3,
"Capture" 63), which turned its hiss into near-full-scale noise and broke wake
word detection. Now boost 1 and capture 45, saved with `sudo alsactl store 1`.
`live/system/mic-mixer.txt` has the current values. To check the noise floor:

```sh
timeout 3 parecord --raw --format=s16le --rate=16000 --channels=1 \
  -d alsa_input.pci-0000_c2_00.6.analog-stereo /tmp/mic.raw
# quiet room: RMS around a few hundred, not tens of thousands
```

## Ad blocking

uBlock Origin Lite is force-installed into every Chromium profile by policy
(`/etc/chromium/policies/managed/adblock.json`). Pi-hole is planned for the
whole network later.

## Containers: rootless Podman

`podman`, `podman-compose`, `passt`, `netavark`, `aardvark-dns`, `crun`.
Docker is installed (Omarchy ships it) but disabled. Containers restart after
boot through the user `podman-restart.service`.

## Display

The face (`luna.service`) starts with the desktop. Luna added a Hyprland window
rule so the face is always full screen (`live/hypr/luna-rules.txt`).

## Networking

- Tailscale with the operator set to `legion` (no sudo for `tailscale` commands).
- Exit node a VPS on the tailnet, LAN access off; see [architecture](architecture.md#network).
- ufw on.

## Packages added

`podman podman-compose passt netavark aardvark-dns crun rust qt6-websockets`,
`luna` (built locally), `hermes-desktop` (via `omarchy-install-hermes-cli`).
