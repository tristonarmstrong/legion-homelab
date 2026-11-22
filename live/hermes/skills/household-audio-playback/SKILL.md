---
name: household-audio-playback
description: Play songs or audio on the Legion Go speakers.
---

# Household audio playback (Luna on the Legion Go)

Use when someone asks Luna to play a song, album, or any audio ("put on X", "turn on that
song"). The output is the Legion's own speakers plus a now-playing card on the panel; there is
no streaming account, so playback means fetching the audio file and playing it locally.

## Procedure

1. **Pin down what they meant before fetching anything.** Voice input garbles artist names badly
   (LANY arrives as "Lanny", "Lena", "Lainey"). Search `web_search("<heard name> song")` and
   match on song titles/lyrics, not on the spelling. If two plausible artists survive, ask one
   short question that names both ("Lanny Wolfe, or the band LANY?") — never guess and play the
   wrong track. A remembered phrase works as well as a name: "super far by Lanny" is the song
   "Super Far" by LANY, so search the phrase before asking again.
2. **Skip the browser for music.** `screen-open`ing a YouTube watch page hits "Sign in to confirm
   you're not a bot", and the `youtube-nocookie.com/embed` variant returns "Video player
   configuration error 153". If the first YouTube attempt is blocked, do not iterate embed/client
   variants — go to step 3.
3. **Find the track on SoundCloud** (yt-dlp handles search, and this box is not walled there):
   ```bash
   yt-dlp --no-playlist --print "%(title)s | %(webpage_url)s" "scsearch5:<artist> <title>"
   ```
   Pick a plain full upload (or the official "Stripped"/acoustic cut) — not a remix or cover —
   and skip anything that fails with "This video is DRM protected".
4. **Download the audio** and check it is the whole song:
   ```bash
   yt-dlp -f bestaudio --audio-quality 0 -o "/tmp/<slug>.%(ext)s" "<soundcloud url>"
   ffprobe -v error -show_entries format=duration -of default=nw=1 /tmp/<slug>.*
   ```
   A full pop song is roughly 3-4 minutes; a ~30s result is a preview — try the next candidate.
5. **Play it** as a tracked background process (the terminal tool refuses `nohup`/`&`):
   `terminal(command="mpv --no-video --volume=80 /tmp/<slug>.m4a", background=true)`.
6. **Verify it is really playing**, not just launched: `pgrep -a mpv` shows the process, and
   `wpctl status` lists `mpv` as a client on the Ryzen analog-stereo sink.
7. **Show a now-playing card** instead of leaving the blocked page up:
   `screen-text "Now playing\n\n<Song>\n<Artist>"` (see household-display-control).
8. **Speak one short sentence**: what is playing and where the sound is coming from.

## Music must duck under Luna's voice (standing preference)

Anything playing has to drop in volume the moment Luna starts speaking — asked for directly
("quiet it so we can hear what you're saying"). It is installed and running; keep it working.

- `~/.local/bin/audio-duck` polls `pactl list sink-inputs` every 150 ms. While a sink input from
  `voiced` is live, every other input is set to 22% of **its own** volume, then restored when the
  speech ends. TTS reaches the sink as `pacat --client-name=voiced --stream-name=voice` (see
  `~/homelab/voiced/src/audio.rs`), which is what the watcher matches on.
- Runs as `audio-duck.service`; check `systemctl --user is-active audio-duck`. If it is missing,
  restore `scripts/audio-duck` to `~/.local/bin/audio-duck` (chmod +x) and
  `scripts/audio-duck.service` to `~/.config/systemd/user/`, then `systemctl --user enable --now
  audio-duck`.
- Test without waiting for a real reply: play a tone in a `background=true` call through
  `pacat --playback --raw --format=s16le --rate=24000 --channels=1 --client-name=voiced
  --stream-name=voice` and watch `pactl list sink-inputs | grep -E "application.name|Volume:
  front-left"`. mpv reads 22% while the tone plays and its old volume after.
- Duck per sink input, never on the sink (`wpctl set-volume @DEFAULT_AUDIO_SINK@`) — the sink
  carries Luna's voice too, so lowering it just makes the speech quiet as well.

## Pitfalls

- A SoundCloud hit is not the studio master just because the title looks right. Check the
  canonical length first (`api.deezer.com` search, see `references/sources.md`) and tell the
  family plainly when what came down is a fan upload or a different mix.

- `mpv` is independent of the screen browser: `screen-close` does **not** stop audio. To stop
  playback, kill the player (`pkill -f "mpv --no-video"`) and confirm with `pgrep -a mpv`.
- `yt-dlp` from `scsearch` returns the first hit even when it is DRM-protected — read the error
  and move to the next result rather than assuming the song is unavailable.
- Progress bars from `yt-dlp` flood stdout with `\r`; add `2>&1 | tail -4` (or `--quiet`) so the
  useful line (`Destination:` / error) is visible.
- Source options and their exact failure symptoms: `references/sources.md`.
