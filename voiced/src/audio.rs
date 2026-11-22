//! Mic capture and playback through PipeWire's Pulse server (pacat), so no audio library is linked.
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub const MIC_RATE: u32 = 16_000;
pub const CHUNK: usize = 1280; // 80 ms, the openWakeWord step

/// Starts `pacat --record` and returns a channel of 80 ms, 16 kHz mono chunks.
pub fn start_mic() -> Result<Receiver<Vec<i16>>> {
    let mut child = Command::new("pacat")
        .args(["--record", "--raw", "--format=s16le", "--rate=16000", "--channels=1",
               "--latency-msec=40", "--client-name=voiced", "--stream-name=wake-word"])
        .stdout(Stdio::piped()).stderr(Stdio::null()).spawn().context("starting pacat --record")?;
    let mut out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buf = vec![0u8; CHUNK * 2];
        while out.read_exact(&mut buf).is_ok() {
            let s = buf.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
            if tx.send(s).is_err() { break; }
        }
        let _ = child.kill();
        eprintln!("mic stream ended");
        std::process::exit(1); // let systemd restart us
    });
    Ok(rx)
}

/// Plays 16-bit mono PCM at `rate` and blocks until it has finished.
pub fn play(pcm: &[u8], rate: u32) -> Result<()> {
    let mut child = Command::new("pacat")
        .args(["--playback", "--raw", "--format=s16le", &format!("--rate={rate}"), "--channels=1",
               "--client-name=voiced", "--stream-name=voice"])
        .stdin(Stdio::piped()).stderr(Stdio::null()).spawn().context("starting pacat --playback")?;
    child.stdin.take().unwrap().write_all(pcm)?;
    child.wait()?;
    Ok(())
}

/// A soft two-note chime (rising = listening, falling = going to sleep).
pub fn chime(rising: bool) -> Vec<u8> {
    let rate = 24_000.0;
    let notes = if rising { [660.0, 990.0] } else { [880.0, 587.0] };
    let mut out = Vec::new();
    for f in notes {
        let n = (rate * 0.09) as usize;
        for i in 0..n {
            let t = i as f32 / rate;
            let env = (std::f32::consts::PI * i as f32 / n as f32).sin();
            let v = (2.0 * std::f32::consts::PI * f * t).sin() * env * 0.25;
            out.extend_from_slice(&((v * 32767.0) as i16).to_le_bytes());
        }
    }
    out
}
