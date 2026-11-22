//! voiced: "Hey Luna" -> Whisper -> Hermes -> tiny-voca, with a follow-up window so you
//! don't have to repeat the wake word.
mod audio;
mod events;
mod hermes;
mod onnx;
mod vad;
mod wake;
mod wyoming;

use anyhow::{Context, Result};
use serde_json::json;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

struct Config {
    models: String,
    wake_model: String,
    wake_threshold: f32,
    whisper: String,
    tts: String,
    voice: String,
    hermes: String,
    hermes_key: String,
    follow_up: Duration,
    history_ttl: Duration,
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

fn main() -> Result<()> {
    let home = std::env::var("HOME")?;
    let get = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
    let env_file = std::fs::read_to_string(format!("{home}/.hermes/.env")).unwrap_or_default();
    let cfg = Config {
        models: get("VOICED_MODELS", &format!("{home}/homelab/voiced/models")),
        wake_model: get("VOICED_WAKE_MODEL", "hey_luna"),
        wake_threshold: get("VOICED_WAKE_THRESHOLD", "0.5").parse()?,
        whisper: get("VOICED_WHISPER", "127.0.0.1:10300"),
        tts: get("VOICED_TTS", "http://127.0.0.1:10210/speak"),
        voice: get("VOICED_VOICE", "F2"),
        hermes: get("VOICED_HERMES", "http://127.0.0.1:8642/v1/responses"),
        hermes_key: env_file.lines().find_map(|l| l.strip_prefix("API_SERVER_KEY=")).unwrap_or("").to_string(),
        follow_up: Duration::from_secs(get("VOICED_FOLLOW_UP_SECS", "8").parse()?),
        history_ttl: Duration::from_secs(300),
    };
    let ort_env = onnx::env()?;
    let mut wake = wake::Wake::new(&ort_env, &cfg.models, &cfg.wake_model).context("loading wake word models")?;
    let mut vad = vad::Vad::new(&ort_env, &format!("{}/silero_vad.onnx", cfg.models)).context("loading VAD")?;

    // `voiced --test-wake file.raw` (16 kHz s16le mono): print the best wake score and VAD activity, then exit.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--test-wake") {
        let bytes = std::fs::read(&args[2])?;
        let pcm: Vec<i16> = bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
        let (mut best, mut speech) = (0f32, 0usize);
        for (i, c) in pcm.chunks_exact(audio::CHUNK).enumerate() {
            let s = wake.process(c)?;
            if vad.process(c)? > 0.5 {
                speech += 1;
            }
            if s > best {
                best = s;
                eprintln!("t={:.2}s score={s:.3}", i as f32 * 0.08);
            }
        }
        println!("best wake score {best:.3}, speech chunks {speech}");
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
            };
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || hermes::stream(req, tx));
            for chunk in rx {
                match chunk {
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
    let mut previous: Option<String> = None;
    let mut last_turn = Instant::now();
    ev.emit("idle", json!({}));
    eprintln!("voiced ready: say \"hey luna\"");

    loop {
        let chunk = mic.recv()?;
        let score = wake.process(&chunk)?;
        if score < cfg.wake_threshold {
            continue;
        }
        ev.emit("wake", json!({"score": score}));
        if last_turn.elapsed() > cfg.history_ttl {
            previous = None;
        }
        audio::play(&audio::chime(true), 24_000)?;
        drain(&mic);
        vad.reset();
        converse(&cfg, &ev, &mic, &mut vad, &mut previous, Duration::from_secs(6));
        last_turn = Instant::now();
        drain(&mic);
        wake.reset();
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

/// One conversation: keeps listening for follow-ups until silence, a stop phrase or an error.
fn converse(
    cfg: &Config,
    ev: &events::Events,
    mic: &Receiver<Vec<i16>>,
    vad: &mut vad::Vad,
    previous: &mut Option<String>,
    mut start_timeout: Duration,
) {
    loop {
        ev.emit("listening", json!({}));
        let Some(pcm) = record(mic, vad, start_timeout) else {
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
        let clean: String = text.to_lowercase().chars().filter(|c| !",.!?".contains(*c)).collect();
        let clean = clean.trim();
        if clean.is_empty() || clean == "you" {
            continue; // whisper hallucinates "you" on noise
        }
        ev.emit("heard", json!({"text": text}));
        if STOP.iter().any(|s| clean == *s || clean.starts_with(&format!("{s} "))) {
            say(cfg, ev, "Okay.");
            return;
        }
        let req = hermes::Request {
            url: cfg.hermes.clone(),
            key: cfg.hermes_key.clone(),
            instructions: format!("{SYSTEM}\n\nRight now the screen shows: {}", screen_state()),
            input: text,
            previous: previous.clone(),
        };
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || hermes::stream(req, tx));
        for chunk in rx {
            match chunk {
                hermes::Chunk::Tool(name) => ev.emit("thinking", json!({"tool": name})),
                hermes::Chunk::Sentence(s) => say(cfg, ev, &s),
                hermes::Chunk::Done(_, id) => {
                    if id.is_some() {
                        *previous = id;
                    }
                }
                hermes::Chunk::Failed(e) => {
                    eprintln!("hermes error: {e}");
                    say(cfg, ev, "Sorry, I could not reach my brain just now.");
                    return;
                }
            }
        }
        drain(mic);
        vad.reset();
    }
}

/// Waits up to `start_timeout` for speech, then records until ~0.8 s of silence (max 20 s).
fn record(mic: &Receiver<Vec<i16>>, vad: &mut vad::Vad, start_timeout: Duration) -> Option<Vec<i16>> {
    let begin = Instant::now();
    let mut pre: Vec<Vec<i16>> = Vec::new();
    let mut pcm: Vec<i16> = Vec::new();
    let (mut speaking, mut silent) = (false, 0u32);
    loop {
        let chunk = mic.recv_timeout(Duration::from_millis(500)).ok()?;
        let p = vad.process(&chunk).unwrap_or(0.0);
        if !speaking {
            pre.push(chunk);
            if pre.len() > 4 {
                pre.remove(0);
            }
            if p > 0.5 {
                speaking = true;
                pcm = pre.concat();
            } else if begin.elapsed() > start_timeout {
                return None;
            }
            continue;
        }
        pcm.extend_from_slice(&chunk);
        silent = if p < 0.35 { silent + 1 } else { 0 };
        if silent >= 10 || pcm.len() > 16_000 * 20 {
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
