---
name: household-display-control
description: Show or clear the Legion Go screen; face on top, content below.
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

## Screen layout: Luna's face on top, content below

Standing household preference (it supersedes the earlier "everything full screen" rule): while
Luna is working on something, the family wants to keep seeing her face. So the panel is split —
the `luna` window keeps a strip across the top (~250 logical px) and the page fills the rest below.
Do not let a page take over the whole display.

- `~/.local/bin/screen-split` implements it: it leaves fullscreen if either window has it, resizes
  `luna` to 616x250 and the content window to 616x730. It is idempotent — re-run it if a window
  still covers the face.
- `screen-open` no longer passes `--start-fullscreen`; it launches the app window and then runs
  `screen-split`. Relayout can be needed a second time because a Chromium app window may already
  remember being fullscreen and the first pass can fire before the window finishes mapping.
- When nothing is open, `luna` alone in the dwindle layout fills the panel — that idle state is
  correct, and `screen-close` still ends with `screen-luna-fullscreen` so closing a page hands the
  whole screen back to her face.
- The old `luna-fullscreen` Hyprland window rule was removed from `~/.config/hypr/hyprland.lua`;
  a rule forcing `fullscreen = true` on class `luna` fights the split. Leave it out.

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
- `screen-open` passes `--password-store=basic` to the screen profile on purpose: without it the
  profile's cookies land in `v11` OS-keyring encryption that CLI tools cannot read. It also passes
  `--autoplay-policy=no-user-gesture-required`, so media pages start by themselves.
- The screen browser's window class is `chrome-<host>__-<path>-Default`, i.e. per-URL. Never match
  on one site's class to identify "the screen", and never `pkill chrome`; kill by profile instead
  (`pkill -f -- "--user-data-dir=$JARVIS_PROFILE"`, which is what `screen-close` does).
- `chrome-*` is **not** unique to the screen browser: the family's own Omarchy web apps (the YouTube
  PWA, for one) use the same `chrome-<host>__-Default` shape. Before acting on a window, prove it is
  Luna's by checking that `/proc/<pid>/cmdline` contains `$JARVIS_PROFILE` — `screen-split` does
  exactly that, and fullscreening the family's window by mistake is the failure it prevents.
- Resizing also acts on the **active** window: `hyprctl dispatch "hl.dsp.focus({ window =
  'address:0x...' })"` first, then `hyprctl dispatch "hl.dsp.window.resize({ x = 616, y = 250 })"`.
  Without the focus call the resize lands on whatever had focus.
- A third window on the workspace (the family's own app) squeezes both panes and the split degrades
  to three columns. That is expected — do not move or close their window to make room; say the
  content shares the bottom half.
- Closing a page is not "clearing the screen": the Luna face page comes back. Say the screen is
  clear only after confirming, and only when asked — declining an offer is not a request to close.
- Long content belongs on the screen, not in the voice reply: reading 1,000 numbers aloud takes
  about ten minutes, so offer the screen version instead of narrating.

## Related

Audio requests ("play that song") are a separate procedure — see `household-audio-playback`. The
screen only carries a `screen-text "Now playing…"` card; the sound comes from mpv, not the page.
