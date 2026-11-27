//! voiced: "Hey Luna" -> Whisper -> Hermes -> tiny-voca, with a follow-up window so you
//! don't have to repeat the wake word.
mod audio;
mod events;
mod hermes;
mod onnx;
mod speaker;
mod vad;
mod wake;
mod wyoming;

use anyhow::{Context, Result};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

struct Config {
    models: String,
    wake_model: String,
    wake_threshold: f32,
    barge_threshold: f32,
    whisper: String,
    /// Whisper used only for the wake check (a tiny model answers in a fraction of the time).
    wake_whisper: String,
    tts: String,
    voice: String,
    hermes: String,
    hermes_key: String,
    reasoning: String,
    follow_up: Duration,
    history_ttl: Duration,
    verify_wake: bool,
    max_request: Duration,
}

const SYSTEM: &str = "You are Luna, the family's voice assistant, running on a Lenovo Legion Go in the house. \
Everything you write is spoken aloud by a text-to-speech voice, so answer in one to three short, natural sentences \
with no markdown, lists, emoji, URLs or code unless asked.\n\
The Legion has a screen you control by running these commands with your terminal tool \
(run them; never write a command into your spoken reply):\n\
- screen-open <url>: show a web page full-screen. Use it for recipes, maps, videos, pictures, anything visual; \
find a good URL with web search first.\n\
- screen-close: close what you opened. Only when someone asks you to close it, put it away, clear the screen and so on; \
declining an offer (\"no thanks\") is not a request to close.\n\
- screen-text \"<text>\": show large text, for example a list or a timer.\n\
- screen-list: list what is open.\n\
Each turn you are told what the screen currently shows; trust that over your memory. \
When you show something, say briefly what is on screen instead of reading it all out. \
If you cannot do something, say plainly why and suggest how it could be fixed, so the family can improve you over time.";

const STOP: &[&str] = &[
    "stop", "never mind", "nevermind", "that's all", "thats all", "that's it", "thank you luna",
    "thanks luna", "goodbye", "bye luna", "go to sleep", "cancel",
];

/// Where Hermes' conversation continues from. `last` may be a cancelled turn (Hermes stores what it did
/// before it was interrupted); `completed` is the last turn that finished, the fallback if chaining fails.
#[derive(Default)]
struct Chain {
    last: Option<String>,
    completed: Option<String>,
}

/// How a turn ended.
enum Turn {
    /// Reply spoken in full: listen for a follow-up.
    Finished,
    /// Cancelled or failed: end the conversation.
    Ended,
}

fn main() -> Result<()> {
    let home = std::env::var("HOME")?;
    let get = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
    let env_file = std::fs::read_to_string(format!("{home}/.hermes/.env")).unwrap_or_default();
    let cfg = Config {
        models: get("VOICED_MODELS", &format!("{home}/homelab/voiced/models")),
        wake_model: get("VOICED_WAKE_MODEL", "hey_luna"),
        wake_threshold: get("VOICED_WAKE_THRESHOLD", "0.3").parse()?,
        barge_threshold: get("VOICED_BARGE_THRESHOLD", "0.5").parse()?,
        whisper: get("VOICED_WHISPER", "127.0.0.1:10300"),
        wake_whisper: get("VOICED_WAKE_WHISPER", &get("VOICED_WHISPER", "127.0.0.1:10300")),
        tts: get("VOICED_TTS", "http://127.0.0.1:10210/speak"),
        voice: get("VOICED_VOICE", "F2"),
        hermes: get("VOICED_HERMES", "http://127.0.0.1:8642/v1/responses"),
        // A little thinking makes her noticeably less dumb (multi-step questions) for about half a second
        // on simple ones; "medium" and up think longer than a spoken reply should wait.
        reasoning: get("VOICED_REASONING", "low"),
        hermes_key: env_file.lines().find_map(|l| l.strip_prefix("API_SERVER_KEY=")).unwrap_or("").to_string(),
        // Off by default, like Alexa: in a busy house the follow-up window mostly catches other people.
        follow_up: Duration::from_secs(get("VOICED_FOLLOW_UP_SECS", "0").parse()?),
        history_ttl: Duration::from_secs(300),
        verify_wake: get("VOICED_VERIFY_WAKE", "1") != "0",
        max_request: Duration::from_secs(get("VOICED_MAX_REQUEST_SECS", "8").parse()?),
    };
    let ort_env = onnx::env()?;
    let mut wake = wake::Wake::new(&ort_env, &cfg.models, &cfg.wake_model).context("loading wake word models")?;
    let mut vad = vad::Vad::new(&ort_env, &format!("{}/silero_vad.onnx", cfg.models)).context("loading VAD")?;

    // `voiced --test-wake file.raw` (16 kHz s16le mono): print the best wake score and VAD activity, then exit.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--test-wake") {
        let bytes = std::fs::read(&args[2])?;
        let pcm: Vec<i16> = bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
        let (mut best, mut speech, mut best_at) = (0f32, 0usize, 0usize);
        for (i, c) in pcm.chunks_exact(audio::CHUNK).enumerate() {
            let s = wake.process(c)?;
            if vad.process(c)? > 0.5 {
                speech += 1;
            }
            if s > best {
                best = s;
                best_at = (i + 1) * audio::CHUNK;
                eprintln!("t={:.2}s score={s:.3}", i as f32 * 0.08);
            }
        }
        println!("best wake score {best:.3}, speech chunks {speech}");
        if best >= cfg.wake_threshold {
            let from = best_at.saturating_sub(RECENT_CHUNKS * audio::CHUNK);
            let (ok, heard) = wake_confirmed(&cfg, &pcm[from..best_at]);
            println!("whisper check: {} (heard {heard:?})", if ok { "accepted" } else { "rejected" });
        }
        return Ok(());
    }

    // `voiced --test-record file.raw`: where recording a request would stop (speech start, end, why).
    if args.get(1).map(String::as_str) == Some("--test-record") {
        let bytes = std::fs::read(&args[2])?;
        let pcm: Vec<i16> = bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
        let (tx, rx) = mpsc::channel();
        for c in pcm.chunks_exact(audio::CHUNK) {
            tx.send(c.to_vec())?;
        }
        for _ in 0..25 {
            tx.send(vec![0; audio::CHUNK])?; // trailing silence
        }
        drop(tx);
        let voice: f32 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        match record(&rx, &mut vad, Duration::from_secs(6), cfg.max_request, voice) {
            Some(r) => println!("recorded {:.2} s of {:.2} s", r.len() as f32 / 16_000.0, pcm.len() as f32 / 16_000.0),
            None => println!("nothing recorded"),
        }
        return Ok(());
    }

    // `voiced --test-chat "turn one" "turn two" ...`: run typed turns through Hermes exactly as voice would,
    // printing what would be spoken and what the screen shows after each turn.
    if args.get(1).map(String::as_str) == Some("--test-chat") {
        let mut previous: Option<String> = None;
        for turn in &args[2..] {
            println!("you:  {turn}");
            let req = hermes::Request {
                url: cfg.hermes.clone(),
                key: cfg.hermes_key.clone(),
                instructions: format!("{SYSTEM}\n\nRight now the screen shows: {}", screen_state()),
                input: turn.clone(),
                previous: previous.clone(),
                reasoning: cfg.reasoning.clone(),
            };
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || hermes::stream(req, tx, Arc::default()));
            for chunk in rx {
                match chunk {
                    hermes::Chunk::Started(_) => {}
                    hermes::Chunk::Tool(name) => println!("      [tool: {name}]"),
                    hermes::Chunk::Sentence(s) => println!("luna: {s}"),
                    hermes::Chunk::Done(_, id) => previous = id.or(previous.take()),
                    hermes::Chunk::Failed(e) => println!("      [error: {e}]"),
                }
            }
            println!("      [screen: {}]", screen_state());
        }
        return Ok(());
    }

    let ev = events::Events::serve("127.0.0.1:7071");
    let mic = audio::start_mic()?;
    // Hermes keeps the conversation (with its tool calls); we only remember where to chain from.
    let mut chain = Chain::default();
    let mut last_turn = Instant::now();
    ev.emit("idle", json!({}));
    eprintln!("voiced ready: say \"hey luna\"");

    let mut recent = Recent::default();
    let mut gate = WakeGate::default();
    loop {
        let chunk = mic.recv()?;
        recent.push(&chunk);
        let score = wake.process(&chunk)?;
        let Some(score) = gate.feed(score, cfg.wake_threshold) else { continue };
        if !confirm(&cfg, &recent, score) {
            gate.rejected();
            continue;
        }
        ev.emit("wake", json!({"score": score}));
        if last_turn.elapsed() > cfg.history_ttl {
            chain = Chain::default();
        }
        audio::play(&audio::chime(true), 24_000)?;
        drain(&mic);
        vad.reset();
        let voice = recent.level();
        eprintln!("your level {voice:.0}");
        converse(&cfg, &ev, &mic, &mut wake, &mut vad, &mut chain, Duration::from_secs(6), voice);
        last_turn = Instant::now();
        drain(&mic);
        wake.reset();
        gate = WakeGate::default();
        ev.emit("idle", json!({}));
    }
}

fn drain(mic: &Receiver<Vec<i16>>) {
    while mic.try_recv().is_ok() {}
}

/// What Luna currently has open on the display, from `screen-list` (titles of her own windows).
fn screen_state() -> String {
    let out = std::process::Command::new("screen-list").output();
    let titles: Vec<String> = out
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.strip_prefix("[luna] "))
        .map(|l| l.split_once(": ").map_or(l, |(_, title)| title).to_string())
        .collect();
    if titles.is_empty() {
        "nothing (none of your windows are open)".to_string()
    } else {
        format!("your window \"{}\" (opened with screen-open; close it with screen-close)", titles.join("\", \""))
    }
}

/// One conversation: keeps listening for follow-ups until silence, a stop phrase, an interrupt or an error.
fn converse(
    cfg: &Config,
    ev: &events::Events,
    mic: &Receiver<Vec<i16>>,
    wake: &mut wake::Wake,
    vad: &mut vad::Vad,
    chain: &mut Chain,
    mut start_timeout: Duration,
    voice: f32,
) {
    loop {
        ev.emit("listening", json!({}));
        let Some(pcm) = record(mic, vad, start_timeout, cfg.max_request, voice) else {
            let _ = audio::play(&audio::chime(false), 24_000);
            return;
        };
        start_timeout = cfg.follow_up;
        ev.emit("thinking", json!({}));
        let text = match wyoming::transcribe(&cfg.whisper, &pcm) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("whisper error: {e}");
                say(cfg, ev, "Sorry, my speech recognition is not responding.");
                return;
            }
        };
        let clean = normalize(&text);
        if clean.is_empty() || clean == "you" {
            continue; // whisper hallucinates "you" on noise
        }
        ev.emit("heard", json!({"text": text}));
        if is_one_of(&clean, STOP) {
            say(cfg, ev, "Okay.");
            return;
        }
        match turn(cfg, ev, mic, wake, chain, &text) {
            Turn::Finished if cfg.follow_up.is_zero() => return,
            Turn::Finished => {}
            Turn::Ended => return,
        }
        drain(mic);
        vad.reset();
    }
}

/// Sends one request to Hermes and speaks the reply, while listening for "Hey Luna" to cut her off.
fn turn(
    cfg: &Config,
    ev: &events::Events,
    mic: &Receiver<Vec<i16>>,
    wake: &mut wake::Wake,
    chain: &mut Chain,
    input: &str,
) -> Turn {
    let mut retries = 0;
    'request: loop {
        let cancel = Arc::new(AtomicBool::new(false));
        let req = hermes::Request {
            url: cfg.hermes.clone(),
            key: cfg.hermes_key.clone(),
            instructions: format!("{SYSTEM}\n\nRight now the screen shows: {}", screen_state()),
            input: input.to_string(),
            previous: chain.last.clone(),
            reasoning: cfg.reasoning.clone(),
        };
        let (tx, rx) = mpsc::channel();
        let c = cancel.clone();
        thread::spawn(move || hermes::stream(req, tx, c));
        let mut recent = Recent::default();
        let mut gate = WakeGate::default();
        let started_at = Instant::now();
        let mut speaker = speaker::Speaker::start(cfg.tts.clone(), cfg.voice.clone(), ev.clone(), cancel.clone());
        let (mut queued, mut started, mut finished, mut failed, mut output) = (0, None, false, false, false);
        wake.reset();
        loop {
            loop {
                match rx.try_recv() {
                    Ok(hermes::Chunk::Started(id)) => started = Some(id),
                    Ok(hermes::Chunk::Tool(name)) => {
                        output = true;
                        ev.emit("thinking", json!({"tool": name}));
                    }
                    Ok(hermes::Chunk::Sentence(s)) => {
                        output = true;
                        queued += 1;
                        speaker.say(s);
                    }
                    Ok(hermes::Chunk::Done(_, id)) => {
                        if id.is_some() {
                            chain.last = id.clone();
                            chain.completed = id;
                        }
                        finished = true;
                    }
                    Ok(hermes::Chunk::Failed(e)) => {
                        eprintln!("hermes error: {e}");
                        // A rejected request with nothing said yet is usually the conversation, not the
                        // question: chaining to a cancelled turn, or a stored history DeepSeek refuses (an
                        // image Hermes' secret redaction corrupted: "Invalid base64 data"). Retry from the
                        // last finished turn, then in a fresh conversation, so she recovers by herself.
                        if !output && retries < 2 && chain.last.is_some() {
                            retries += 1;
                            if chain.last != chain.completed {
                                chain.last = chain.completed.clone();
                            } else {
                                eprintln!("starting a fresh conversation");
                                *chain = Chain::default();
                            }
                            speaker.stop();
                            continue 'request;
                        }
                        queued += 1;
                        speaker.say("Sorry, I could not reach my brain just now.".into());
                        (finished, failed) = (true, true);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
            if finished && speaker.spoken() >= queued {
                return if failed { Turn::Ended } else { Turn::Finished };
            }
            let Ok(chunk) = mic.recv_timeout(Duration::from_millis(100)) else { continue };
            recent.push(&chunk);
            let score = wake.process(&chunk).unwrap_or(0.0);
            let threshold = speaker.wake_threshold(cfg.wake_threshold, cfg.barge_threshold);
            let Some(score) = gate.feed(score, threshold) else { continue };
            // Saying "Hey Luna" twice in a row shouldn't cancel the request you just made: interrupts only
            // count once she has started answering, or after 4 s of thinking.
            if queued == 0 && started_at.elapsed() < Duration::from_secs(4) {
                continue;
            }
            if !confirm(cfg, &recent, score) {
                gate.rejected();
                continue;
            }
            // "Hey Luna" while she is thinking or talking: stop everything and go back to sleep.
            // Whatever follows ("cancel", or the TV) is ignored; a new request needs a new "Hey Luna".
            speaker.stop();
            cancel.store(true, Ordering::SeqCst); // the Hermes thread drops the stream, which interrupts the agent
            if !finished && let Some(id) = started {
                chain.last = Some(id);
            }
            eprintln!("interrupted (wake score {score:.3})");
            let _ = audio::play(&audio::chime(false), 24_000);
            return Turn::Ended;
        }
    }
}

/// The last ~2 s of mic audio, for checking a wake with Whisper.
const RECENT_CHUNKS: usize = 25;

#[derive(Default)]
struct Recent(std::collections::VecDeque<Vec<i16>>);

impl Recent {
    fn push(&mut self, chunk: &[i16]) {
        self.0.push_back(chunk.to_vec());
        if self.0.len() > RECENT_CHUNKS {
            self.0.pop_front();
        }
    }

    /// How loud the wake word was (mean of the three loudest 80 ms chunks): the speaker's level.
    fn level(&self) -> f32 {
        let mut r: Vec<f32> = self.0.iter().map(|c| rms(c)).collect();
        r.sort_by(|a, b| b.total_cmp(a));
        let top = &r[..r.len().min(3)];
        if top.is_empty() { 0.0 } else { top.iter().sum::<f32>() / top.len() as f32 }
    }
}

fn rms(chunk: &[i16]) -> f32 {
    (chunk.iter().map(|&v| (v as f32).powi(2)).sum::<f32>() / chunk.len().max(1) as f32).sqrt()
}

/// Decides when a rising wake score gets the Whisper check. It waits for the score to peak (at most
/// ~320 ms), so Whisper hears the whole "Luna" and not "Hey Lu...". After a rejection it ignores the
/// same sound for 1.5 s instead of resetting the wake model (which would throw away a real "Hey Luna"
/// that was still being said). It also logs near misses (peaks of 0.2 up to the threshold).
#[derive(Default)]
struct WakeGate {
    best: Option<f32>,
    waited: u32,
    quiet_until: Option<Instant>,
    near: f32,
}

impl WakeGate {
    /// Feed every chunk's score; returns the peak score when it is time to check it.
    fn feed(&mut self, score: f32, threshold: f32) -> Option<f32> {
        if score >= 0.2 {
            self.near = self.near.max(score);
        } else if self.near > 0.0 {
            if self.near < threshold {
                eprintln!("near miss: wake score {:.2}", self.near);
            }
            self.near = 0.0;
        }
        if self.quiet_until.is_some_and(|t| Instant::now() < t) {
            return None;
        }
        match self.best {
            None if score >= threshold => {
                self.best = Some(score);
                self.waited = 0;
                None
            }
            None => None,
            Some(best) => {
                self.waited += 1;
                if score > best && self.waited < 4 {
                    self.best = Some(score);
                    return None;
                }
                self.best = None;
                self.near = 0.0;
                Some(best.max(score))
            }
        }
    }

    fn rejected(&mut self) {
        self.quiet_until = Some(Instant::now() + Duration::from_millis(1500));
    }
}

/// Second-stage wake check (Alexa confirms its wake word in the cloud; we ask Whisper): the wake model
/// fires on TV and chatter, so only wake if Whisper also hears "Luna" in the last ~2 s.
fn confirm(cfg: &Config, recent: &Recent, score: f32) -> bool {
    if !cfg.verify_wake {
        return true;
    }
    let pcm: Vec<i16> = recent.0.iter().flatten().copied().collect();
    let (ok, heard) = wake_confirmed(cfg, &pcm);
    if !ok {
        // Keep the last rejected clip (16 kHz s16le mono) for checking: voiced --test-wake <file>
        let path = "/tmp/voiced-rejected-wake.raw";
        let bytes: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
        let _ = std::fs::write(path, bytes);
        eprintln!("wake rejected (score {score:.2}, whisper heard {heard:?}; audio in {path})");
    }
    ok
}

fn wake_confirmed(cfg: &Config, pcm: &[i16]) -> (bool, String) {
    match wyoming::transcribe(&cfg.wake_whisper, pcm) {
        Ok(text) => {
            let ok = normalize(&text).split_whitespace().any(|w| {
                // Whisper's usual spellings of "Luna" (lunar, loona, looner, luna's...)
                w.starts_with("lun") || w.starts_with("loon") || w.starts_with("luen")
            });
            (ok, text)
        }
        Err(e) => {
            eprintln!("whisper error during wake check: {e}");
            (true, String::new()) // don't go deaf if Whisper is down; the turn will report it
        }
    }
}

/// Lower case without the punctuation Whisper adds.
fn normalize(text: &str) -> String {
    let clean: String = text.to_lowercase().chars().filter(|c| !",.!?".contains(*c)).collect();
    clean.trim().to_string()
}

/// `clean` is one of `phrases`, or starts with one ("stop please").
fn is_one_of(clean: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|s| clean == *s || clean.starts_with(&format!("{s} ")))
}

/// Waits up to `start_timeout` for speech, then records until the speaker pauses (max `max_len`).
///
/// In a busy house the room is never silent, so "you" are told apart by loudness: you are close to the
/// mic, the TV and the people across the room aren't. `voice` is how loud your "Hey Luna" was (0 if
/// unknown). Recording starts only on speech at least -9 dB of that, and a chunk counts as a pause when
/// the VAD hears no speech or it is 14 dB quieter than you. Without `voice`, your level is taken from
/// your first 1.5 s.
fn record(
    mic: &Receiver<Vec<i16>>,
    vad: &mut vad::Vad,
    start_timeout: Duration,
    max_len: Duration,
    voice: f32,
) -> Option<Vec<i16>> {
    let begin = Instant::now();
    let max_samples = (max_len.as_secs_f32() * 16_000.0) as usize;
    let mut pre: Vec<Vec<i16>> = Vec::new();
    let mut pcm: Vec<i16> = Vec::new();
    let (mut speaking, mut silent, mut level, mut waited) = (false, 0u32, voice, 0usize);
    loop {
        let chunk = mic.recv_timeout(Duration::from_millis(500)).ok()?;
        let p = vad.process(&chunk).unwrap_or(0.0);
        let loud = rms(&chunk);
        if !speaking {
            waited += chunk.len();
            pre.push(chunk);
            if pre.len() > 4 {
                pre.remove(0);
            }
            if p > 0.5 && loud >= voice * 0.35 {
                speaking = true;
                pcm = pre.concat();
                level = level.max(loud);
            } else if begin.elapsed() > start_timeout || waited as f32 / 16_000.0 > start_timeout.as_secs_f32() {
                return None;
            }
            continue;
        }
        let into = pcm.len();
        pcm.extend_from_slice(&chunk);
        if voice == 0.0 && into < 24_000 && p > 0.5 {
            level = level.max(loud); // no wake word to go by: your level from the first 1.5 s
        }
        let pause = p < 0.35 || loud < level * 0.2;
        silent = if pause { silent + 1 } else { 0 };
        if silent >= 10 || pcm.len() >= max_samples {
            return Some(pcm);
        }
    }
}

/// Speaks one piece of text with tiny-voca.
fn say(cfg: &Config, ev: &events::Events, text: &str) {
    ev.emit("speaking", json!({"text": text}));
    let wav = ureq::post(&cfg.tts)
        .send_json(json!({"text": text, "voice": cfg.voice}))
        .and_then(|mut r| r.body_mut().read_to_vec());
    match wav {
        Ok(w) if w.len() > 44 => {
            let _ = audio::play(&w[44..], 24_000);
        }
        Ok(_) => {}
        Err(e) => eprintln!("tts error: {e}"),
    }
}
