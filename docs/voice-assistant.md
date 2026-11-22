# Luna, the voice assistant

Luna is four small programs and an agent:

| Part | Job |
|---|---|
| `voiced` (Rust) | Hears "Hey Luna", records you, runs the conversation, speaks |
| Whisper | Turns your speech into text |
| Hermes Agent + DeepSeek V4 Flash | Understands, answers, uses tools, remembers |
| tiny-voca | Turns her words into speech |
| `luna` (Qt) | Her face on the screen |

## Talking to her

- "Hey Luna", then ask. You'll hear a rising chime when she's listening.
- Keep talking after she answers; no wake word needed for about 8 seconds.
- End with "thanks Luna", "bye Luna", "that's all", "never mind", or by
  staying quiet (falling chime).
- "Pull up a pancake recipe" puts a page on the screen; "close that" takes it
  away. "No thanks" to an offer leaves it up.

## voiced

Source: `voiced/src/`. Build with `cargo build --release` in `voiced/`; the
service runs `voiced/target/release/voiced`.

| File | What it does |
|---|---|
| `main.rs` | Config, Luna's instructions (`SYSTEM`), stop phrases, the conversation loop, test modes |
| `audio.rs` | Mic capture and playback through `pacat`; the chimes |
| `wake.rs` | openWakeWord streaming pipeline (reads any model's tensor names) |
| `vad.rs` | Silero VAD v5 |
| `wyoming.rs` | Minimal Wyoming client for Whisper |
| `hermes.rs` | Streams Hermes' Responses API; splits sentences; drops markdown, URLs and echoed shell commands |
| `events.rs` | State WebSocket on 127.0.0.1:7071 |
| `onnx.rs` | Shared ONNX Runtime environment |

Settings are environment variables (set them in `voiced.service`):

| Variable | Default |
|---|---|
| `VOICED_WAKE_MODEL` | `hey_luna` (any `.onnx` in `voiced/models/`) |
| `VOICED_WAKE_THRESHOLD` | `0.5` |
| `VOICED_VOICE` | `F2` (tiny-voca's other voice is `M2`) |
| `VOICED_FOLLOW_UP_SECS` | `8` |
| `VOICED_WHISPER` / `VOICED_TTS` / `VOICED_HERMES` | local defaults |

Test modes, no microphone needed:

```sh
# Wake word score for a 16 kHz s16le mono recording
voiced/target/release/voiced --test-wake clip.raw
# Typed turns through Hermes exactly as voice would, with what's on screen after each
PATH=~/.local/bin:$PATH voiced/target/release/voiced --test-chat "Pull up a pancake recipe." "Close that."
```

## Wake word

"Hey Luna" is a community openWakeWord model from
[fwartner/home-assistant-wakewords-collection](https://github.com/fwartner/home-assistant-wakewords-collection).
On synthetic speech it scored 0.87 (female voice) and 0.85 (male), with
unrelated speech and "the luna moth flew by" at 0.001; the threshold is 0.5.
To switch, download another model's `.onnx` into `voiced/models/`, set
`VOICED_WAKE_MODEL`, and restart. A custom word means training one (see
[decisions](decisions.md)).

## Hermes

Hermes Agent (Nous Research), installed with `omarchy-install-hermes-cli`, at
`~/.hermes/hermes-agent`, updated with `hermes update`.

- Model: `deepseek-v4-flash` via provider `deepseek`; key `DEEPSEEK_API_KEY`
  in `~/.hermes/.env`.
- API server on 127.0.0.1:8642 (`API_SERVER_ENABLED=true`,
  `API_SERVER_KEY` in `~/.hermes/.env`; `voiced` reads it from there).
- `live/hermes/config.yaml` is the config without secrets;
  `live/hermes/env-variables.txt` lists which variables are set.

Luna's personality and rules live in `SYSTEM` in `voiced/src/main.rs`: who she
is, short spoken answers, how to use the screen, to trust the "screen shows"
line over memory, and to say plainly why something failed and how to fix it.

## The screen

Hermes drives the display with four commands in `~/.local/bin` (copied to
`live/bin/`):

| Command | Does |
|---|---|
| `screen-open <url>` | Full-screen Chromium app window, in its own profile (`~/.local/share/jarvis-screen`) so your own browser is never touched; replaces whatever she had open |
| `screen-close` | Kills that profile's windows, then re-fullscreens the face |
| `screen-text "<text>"` | Big text page (each line a row) |
| `screen-list` | Windows on screen; Luna's are tagged `[luna]` |
| `screen-env` | Sourced by the others to reach the Hyprland session from a service |

uBlock Origin Lite is force-installed in Chromium by policy, so pages she
opens have no ads.

## The face

`luna` is an Omarchy-style Qt Quick app (`~/Work/luna`, its own git repo),
installed as the `luna` package with a launcher entry and `luna.service`.
Original character: two glowing capsule eyes in the Omarchy accent color,
soft cheeks, a small mouth.

| State | Face |
|---|---|
| idle | resting smile; after 20 s dozes: slit eyes, drifting z's, dimmed |
| wake | eyes pop open with a bounce |
| listening | wide eyes, small "o" mouth, pulsing ring |
| thinking | eyes glance around, thought bubbles, "Searching the web…", your words in quotes |
| speaking | mouth moves, each sentence as a caption |

If `voiced` isn't running, the face says so. Keys (with a keyboard): Space
previews moods, F full screen, ? shortcuts, Q quit.
`luna snapshot <state> out.png` renders a mood offscreen.

## How she improves herself

Hermes can edit files and write skills, and Luna does. So far she has:

- written `screen-luna-fullscreen` and called it from `screen-close`, because
  closing a page left her face un-fullscreened;
- added a Hyprland window rule (`luna-fullscreen`) to `~/.config/hypr/hyprland.lua`;
- written the skill `household-display-control` (`live/hermes/skills/`).

Run `./collect` and `git diff` to see what she has changed since the last
commit. If a change is bad, restore the file from `live/` and tell her why.
