---
name: household-display-control
description: Show or clear the Legion Go screen; keep it full screen.
---

# Household display control (Luna on the Legion Go)

Use when showing anything on the household display — URLs, big text, lists, timers, recipes,
photos — or when the screen looks wrong (not full screen, stale page, window in a corner).

One physical display: the Legion Go panel, 800x1280 portrait, Hyprland scale 1.25, so the
logical desktop is **640x1024**. Everything Luna shows replaces what was there before.

## Procedure

1. Pick the verb:
   - `screen-open <url>` — full-screen web page (Chromium app window, its own profile at
     `$HOME/.local/share/jarvis-screen`, so it never touches the user's own browser).
   - `screen-text "<text>"` — large text page; each line becomes a row. Multi-line input works:
     `screen-text "$(cat /tmp/list.txt)"`.
   - `screen-list` — what is open now. `screen-close` — put it away.
2. Generate content into a file first, then pass it, so you can check it before showing and
   re-show without regenerating:
   `python3 - <<'EOF' > /tmp/list.txt ... EOF; screen-text "$(cat /tmp/list.txt)"`
3. For `screen-text` with long lists, keep rows short (about 20 numbers or 3-4 words per row) —
   the font is `clamp(28px,7vw,96px)`, so wide rows overflow the panel.
4. Verify, then speak. Do not describe a page from command output alone — `screen-open` and
   `screen-text` only report that they launched. Confirm with a screenshot (below).
5. Report in one short sentence: what is on the screen, not a recitation of it.

## This display must always be full screen

Standing household preference. Two independent layers implement it; keep both intact.

- Pages: `screen-open` launches Chromium with `--start-fullscreen`, so web pages fill the monitor.
- Luna's own window: `/usr/bin/luna` (Hyprland class `luna`) is what fills the screen when nothing
  is open. Tiled dwindle layout leaves it gapped, which is what "you're not full screen anymore"
  looks like.
  - Window rule in `~/.config/hypr/hyprland.lua`: `hl.window_rule({ name = "luna-fullscreen",
    match = { class = "^luna$" }, fullscreen = true })` — applies when the window maps.
  - `~/.local/bin/screen-luna-fullscreen` re-fullscreens the live window; `screen-close` calls it,
    so closing a page restores a full-screen Luna instead of a gapped tile.
  - The `luna` window is the assistant UI process — never kill it to "test" a window rule; test
    the helper on the live window instead.

Apply Hyprland config changes with `hyprctl reload`, then **always** `hyprctl configerrors` — an
invalid rule property is otherwise silently dropped.

## Verifying what is really on screen

```bash
source ~/.local/bin/screen-env   # HYPRLAND_INSTANCE_SIGNATURE, WAYLAND_DISPLAY, JARVIS_PROFILE
hyprctl clients -j | python3 -c "import json,sys;[print(c['class'],'fs',c['fullscreen'],c['at'],c['size']) for c in json.load(sys.stdin)]"
grim -o eDP-1 /tmp/screen-check.png   # then look at the PNG with vision
```

- `fullscreen: 2` = fullscreen on the monitor; `0` = not fullscreen. Window `size` is logical
  pixels, so a correct full-screen window reads `[640, 1024]`, not `[800, 1280]` — that is the
  1.25 scale, not a bug.
- A page is only confirmed by looking at the screenshot; a launched window can be behind another
  window or still showing the previous page.

## Pitfalls

- `hyprctl dispatch fullscreen 0 address:0x...` fails on this build (`')' expected near '0'`).
  Use `hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'fullscreen' })"`; it acts on the
  **active** window, so focus the target first with
  `hyprctl dispatch "hl.dsp.focus({ window = 'address:0x...' })"`.
- The screen browser's window class is `chrome-<host>__-<path>-Default`, i.e. per-URL. Never match
  on one site's class to identify "the screen", and never `pkill chrome`; kill by profile instead
  (`pkill -f -- "--user-data-dir=$JARVIS_PROFILE"`, which is what `screen-close` does).
- Closing a page is not "clearing the screen": the Luna face page comes back. Say the screen is
  clear only after confirming, and only when asked — declining an offer is not a request to close.
- Long content belongs on the screen, not in the voice reply: reading 1,000 numbers aloud takes
  about ten minutes, so offer the screen version instead of narrating.

## Related

Audio requests ("play that song") are a separate procedure — see `household-audio-playback`. The
screen only carries a `screen-text "Now playing…"` card; the sound comes from mpv, not the page.
