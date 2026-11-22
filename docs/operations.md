# Operations

SSH in with `ssh legion@<legion-host>`. User services need
`XDG_RUNTIME_DIR=/run/user/1000` when run over SSH; most commands below work
as-is.

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
| Face | edit `~/Work/luna`, `bin/test`, `bin/install`, restart luna |

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
