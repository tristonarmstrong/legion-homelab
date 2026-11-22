# Decisions

Why it is built this way, including what was tried and dropped. Newest first
within each topic.

## Voice assistant

- **Own voice loop instead of Home Assistant.** Home Assistant was set up
  first (Assist pipeline, voice satellite, a community Hermes bridge). It put
  Home Assistant between the family and Hermes, there are no smart-home
  devices to justify it, and Home Assistant-style control can be built later
  in Rust if needed. Replaced by `voiced`; the old setup is archived in
  `archive/` (not committed).
- **Rust for voiced.** Always-on and tiny: about 50 MB of RAM, a few percent of
  one core. Mic and speaker go through `pacat`, so no audio library is linked;
  models run in-process with ONNX Runtime.
- **Hermes Agent as the brain.** A full agent with memory, skills, web search
  and a terminal, so Luna can act (show pages, fix her own display problems)
  and improve over time, not just chat.
- **DeepSeek V4 Flash.** The family already pays for DeepSeek, and Flash is
  fast enough for voice (about 2 s to a first answer). Tried first: Claude Haiku
  5.5 with an Anthropic API key, but the API account had no credit, and a
  Claude Max plan only covers Hermes through paid extra usage.
- **Responses API with chaining, not Chat Completions.** With Chat Completions
  voiced kept only the spoken text, so Hermes forgot its own tool calls and
  "corrected" itself about pages it had opened.
- **tiny-voca for speech.** Chosen by the family: 4.5M parameters, about 50 ms
  for four sentences on the CPU, Apache-2.0. Run through ONNX + dizione to avoid
  installing PyTorch.
- **Whisper `small-int8`** for speech to text: accurate enough, quick on 16
  threads.
- **"Hey Luna" wake word** from the community collection. "Hey Jarvis" was the
  first, but didn't fit. A custom word ("moomoo" was floated) would need
  training a model; local CPU training takes hours and is possible later.

## Face

- **Own Qt Quick app (`luna`)** following Omarchy's app conventions: themed by
  the Omarchy accent, packaged, a user service. It only draws what voiced
  reports.
- **Glow from layered shapes, not a shader effect.** The first version used
  `MultiEffect`, which didn't render offscreen; plain shapes render anywhere and
  cost less.

## Home server

- **Rootless Podman** instead of Docker: no root daemon and no docker group.
  Ports bind to 127.0.0.1, so ufw is never bypassed.
- **Tailscale serve, not a reverse proxy.** Real HTTPS certificates and
  tailnet-only access with no extra software.
- **Homepage** as the dashboard: light, with live status per service.
- **Paperless** covers "documents"; no Nextcloud.

## System

- **80% battery cap, not 70%.** The firmware has no 70% setting; a software
  70% would mean force-discharge cycling, so the firmware's 80% won.
- **No lock screen.** It's a home device with no keyboard, and the family
  wants to see what Luna is doing.
- **uBlock Origin Lite by policy** for ads now; Pi-hole when the home internet
  is sorted.
