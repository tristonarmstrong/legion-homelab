# Legion home server and Luna

A Lenovo Legion Go running Omarchy, set up as the house's always-on box. It does
two jobs:

1. **Home server.** Photos, documents and passwords for the family, reachable
   only over Tailscale, with a dashboard at the root address.
2. **Luna, the voice assistant.** Say "Hey Luna" and talk to her. She answers
   out loud, can look things up, can put pages on the screen and take them
   away, and shows an animated face while she works.

Everything runs as the `legion` user: rootless Podman containers and systemd
user services. Nothing listens on the local network; services are bound to
`127.0.0.1` and published over Tailscale HTTPS.

## At a glance

| What | Where |
|---|---|
| Dashboard | https://legion.<tailnet>.ts.net |
| Immich (photos) | https://legion.<tailnet>.ts.net:2283 |
| Paperless-ngx (documents) | https://legion.<tailnet>.ts.net:8000 |
| Vaultwarden (passwords) | https://legion.<tailnet>.ts.net:8222 |
| Luna | the Legion itself: "Hey Luna" |
| SSH | `ssh legion@<legion-host>` (Tailscale) |
| Logins | `~/homelab/CREDENTIALS.txt` on the Legion (never committed) |

```text
   "Hey Luna" ─▶ voiced (Rust) ─▶ Whisper ─▶ Hermes Agent ─▶ DeepSeek V4 Flash
                   │  wake word, VAD          │  web search, terminal, memory, skills
                   │                          └─▶ screen-open / screen-close ─▶ the display
                   ├─▶ tiny-voca (speech) ─▶ speaker
                   └─▶ luna (face app) ◀── state over ws://127.0.0.1:7071

   Immich · Paperless · Vaultwarden · Homepage ─▶ 127.0.0.1 ─▶ tailscale serve (HTTPS) ─▶ tailnet
```

## Docs

- [Architecture](docs/architecture.md): every piece, the ports, how they connect.
- [Luna, the voice assistant](docs/voice-assistant.md): voiced, the wake word,
  Hermes, speech, the screen commands, the face, and how she improves herself.
- [Services](docs/services.md): Immich, Paperless, Vaultwarden, the dashboard, Tailscale.
- [System setup](docs/system.md): battery cap, never sleeping, no lock screen,
  microphone levels, ad blocking, Podman, networking.
- [Operations](docs/operations.md): logs, restarting, updating, backups, and
  fixes for the problems we have actually hit.
- [Decisions](docs/decisions.md): why it is built this way, and what was tried
  and dropped.

## This repo

`~/homelab` on the Legion is the repo. It holds the compose files, the voice
loop's source and the tiny-voca service; Luna's face is its own app in
`~/Work/luna`. Files that live elsewhere on the system (screen commands,
systemd units, root-owned config, Luna's self-written skills) are copied into
`live/` by `./collect`:

```sh
cd ~/homelab
./collect                # snapshot the live config into live/
git status && git diff   # see what changed, including anything Luna changed
git add -A && git commit -m "Describe the change"
```

The `.gitignore` is an allow-list: only the files it names can be committed, so
`.env` files, `CREDENTIALS.txt`, API keys, photos, databases and models stay
out. Models and weights are downloaded by `voiced/models/fetch` and `tts/fetch`.
