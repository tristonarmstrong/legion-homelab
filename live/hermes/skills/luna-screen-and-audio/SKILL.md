---
name: luna-screen-and-audio
description: Use when showing pages or playing music on the Legion.
---

# Luna screen and audio on the Legion

Voice assistant on a Lenovo Legion Go running Hyprland (800x1280 at scale 1.25, so logical 640x1024).

## Screen commands (in ~/.local/bin)

- `screen-open <url>` — fullscreen Chromium app window, dedicated profile `~/.local/share/jarvis-screen`.
- `screen-text "line\nline"` — renders text into `~/.local/share/jarvis-screen-pages/text.html`, then screen-open on it.
- `screen-close` — kills only the Jarvis Chromium profile; also restores Luna's own window to fullscreen.
- `screen-luna-fullscreen` — fullscreens the `luna` window if it isn't already.
- `screen-env` — exports HYPRLAND_INSTANCE_SIGNATURE / XDG_RUNTIME_DIR / JARVIS_PROFILE; **source it before any hyprctl call from a service**.

## Layout and fullscreen pitfalls

The panel is split: Luna's face keeps a strip at the top (~250 logical px) and anything she shows
fills the rest below. `screen-open` runs `~/.local/bin/screen-split` after launching; it is
idempotent, so re-run it if a freshly opened window still covers the face.

- Chromium app windows report `fullscreen: 2` and size 640x1024. **That is fullscreen on this
display** — do not "fix" it. It is also the state to leave when the page is meant to be shown
alone; the split is the default for working-with-content.
- There is no longer a Hyprland rule forcing class `luna` fullscreen — it was removed because it
fights the split. Idle Luna (nothing else open) fills the panel through the tiling layout itself,
and `screen-close` still calls `screen-luna-fullscreen` to hand the screen back to her face.
- `chrome-*` window classes are shared with the family's own Omarchy web apps; confirm a window is
Luna's by checking `/proc/<pid>/cmdline` for `$JARVIS_PROFILE` before resizing or fullscreening it.
- Toggle fullscreen for the active window with `hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'fullscreen' })"`. Plain `hyprctl dispatch fullscreen 0 <addr>` fails with a Lua parse error. Resize/move behave the same way (`hl.dsp.window.resize({ x = 616, y = 250 })`), always on the active window — focus by address first.
- Verify with a screenshot plus eyes, not assumptions: `grim -o eDP-1 /tmp/x.png`, then inspect it.
- Inspect windows: `hyprctl clients -j` (class, fullscreen, size); monitors: `hyprctl monitors -j`. Check config errors with `hyprctl configerrors` after editing lua.

## Playing a song out loud

YouTube is unusable from this machine — both the watch page and the nocookie embed hit "Sign in to confirm you're not a bot" (embed also throws Error 153), and yt-dlp fails with the same bot wall across `player_client` values (android, tv_embedded, default). Do not burn calls retrying YouTube.

Working path:

1. Find the track: `yt-dlp --no-playlist --print "%(title)s | %(webpage_url)s" "scsearch5:<artist> <title>"`. Skip any result erroring `This video is DRM protected`.
2. Download: `yt-dlp -f bestaudio -o /tmp/track.%(ext)s <soundcloud url>`.
3. Sanity-check: `ffprobe -v error -show_entries format=duration -of default=nw=1 /tmp/track.m4a` — full songs are ~200s+ and a few MB.
4. Show a now-playing card with `screen-text`, then play detached: `terminal(command="mpv --no-video --volume=80 /tmp/track.m4a", background=true)`.
5. Confirm it reached the sink: `wpctl status` lists mpv under clients, and `pgrep -a mpv` shows it alive.

Fan uploads from SoundCloud are often a different mix or cover than the studio master — say so rather than claiming it's the official version.
