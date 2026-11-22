# Operations

SSH in with `ssh legion@<legion-host>`. User services need
`XDG_RUNTIME_DIR=/run/user/1000` when run over SSH; most commands below work
as-is.

## Turn Luna on and off

`~/homelab/lunactl` starts and stops all of Luna: Hermes (her brain), `voiced`
(wake word and voice loop), the face, and the Whisper and tiny-voca containers.
The home server is never touched. It sets up the session environment user
services need, so it works straight over SSH from any machine on the tailnet:

```sh
ssh legion@<legion-host> ~/homelab/lunactl start     # on now (about 5 s)
ssh legion@<legion-host> ~/homelab/lunactl stop      # off now, screen cleared, mic released
ssh legion@<legion-host> ~/homelab/lunactl status    # each part, and whether it starts at boot
ssh legion@<legion-host> ~/homelab/lunactl restart
ssh legion@<legion-host> ~/homelab/lunactl enable    # start at boot from now on (and now)
ssh legion@<legion-host> ~/homelab/lunactl disable   # don't start at boot (and stop now)
```

`start` and `stop` don't change what happens at boot; `enable` and `disable` do.
Right now Luna is **disabled**: off, and stays off after a reboot. If you
`start` her without `enable` and the Legion reboots, the speech containers come
back on their own but Luna stays off; `lunactl stop` tidies them away.

`status` looks like this when she's off:

```text
PART               NOW        AT-BOOT
hermes-gateway     inactive   disabled
voiced             inactive   disabled
luna               inactive   disabled
speech_whisper_1   exited     restart=unless-stopped
tts_tiny-voca_1    exited     restart=unless-stopped
Microphone: not in use
```

What it runs, if you want to do it by hand (over SSH, first
`export XDG_RUNTIME_DIR=/run/user/1000`):

| | Start | Stop |
|---|---|---|
| Speech containers | `podman start speech_whisper_1 tts_tiny-voca_1` | `podman stop speech_whisper_1 tts_tiny-voca_1` |
| Brain, voice loop, face | `systemctl --user start hermes-gateway voiced luna` | `systemctl --user stop luna voiced hermes-gateway` |
| At boot | `systemctl --user enable hermes-gateway voiced luna` | `systemctl --user disable hermes-gateway voiced luna` |

## Is everything up?

```sh
systemctl --user status voiced luna hermes-gateway
podman ps --format "{{.Names}}\t{{.Status}}"
tailscale serve status
```

The dashboard shows a green or red dot per service.

## Logs

```sh
journalctl --user -u voiced -f          # every wake, transcript, tool and sentence
journalctl --user -u hermes-gateway -f
journalctl --user -u luna -f
podman logs -f <container>
```

## Restart

```sh
systemctl --user restart voiced          # voice loop
systemctl --user restart hermes-gateway  # Hermes
systemctl --user restart luna            # face
cd ~/homelab/<service> && podman-compose restart
```

## Change things

| Change | How |
|---|---|
| Luna's personality or rules | edit `SYSTEM` in `voiced/src/main.rs`, `cargo build --release`, restart voiced |
| Wake word | model into `voiced/models/`, `Environment=VOICED_WAKE_MODEL=<name>` in `voiced.service`, `systemctl --user daemon-reload`, restart |
| Voice | `Environment=VOICED_VOICE=M2` |
| Too many or too few wake-ups | `Environment=VOICED_WAKE_THRESHOLD=0.4` (fewer misses) or `0.6` (fewer false triggers) |
| Brain model | `hermes config set model.default <model>`, restart hermes-gateway |
| Face | edit `luna/`, then `luna/bin/test`, `luna/bin/install`, restart luna |

## Update

```sh
hermes update                                     # Hermes
cd ~/homelab/<service> && podman-compose pull && podman-compose up -d
cd ~/homelab/tts && podman-compose build --pull && podman-compose up -d
omarchy-update                                    # the OS
```

Immich: read its release notes before pulling a new major version.

## Back up

Not set up yet. What matters, in order:

1. `vaultwarden/data/` (passwords)
2. `immich/library/` and a database dump
   (`podman exec immich_postgres pg_dumpall -U postgres > immich.sql`)
3. Paperless: `podman exec paperless_webserver_1 document_exporter ../export`,
   then copy the `export` volume
4. `~/homelab/CREDENTIALS.txt`, the `.env` files and `~/.hermes/.env`, kept
   somewhere safe (not in git)
5. `~/.hermes/` (Luna's memories and skills)

## Problems we have hit

| Symptom | Cause | Fix |
|---|---|---|
| "Hey Luna" (then Jarvis) stopped triggering | Mic boosts at maximum: constant near-clipping noise | Boost 1, capture 45, `sudo alsactl store 1` (see [system](system.md#microphone-levels)) |
| Luna said she never opened a page she had opened | Only her spoken text was sent back each turn, not her tool calls | voiced chains turns with Hermes' Responses API and tells her what is on screen |
| Luna read "screen-close" out loud | DeepSeek echoes commands into its reply | voiced drops reply lines that start with a command |
| "No thanks" closed the page | Over-eager instruction-following | Instructions say declining an offer is not a request to close |
| Face not full screen after a page closes | Tiled layout | Luna's own fix: window rule plus `screen-luna-fullscreen` |
| Home Assistant ignored its port settings | Newer HA keeps HTTP settings in `.storage/http`, not YAML | Historical: HA was removed |
| Sudo needed over SSH | No terminal for the password | `omarchy-sudo-passwordless <minutes>` grants a time-limited window |
