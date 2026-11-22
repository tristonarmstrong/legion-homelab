//! Minimal Wyoming client for speech-to-text (faster-whisper container).
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn send(s: &mut TcpStream, kind: &str, data: Value, payload: &[u8]) -> Result<()> {
    let mut h = json!({"type": kind, "data": data});
    if !payload.is_empty() { h["payload_length"] = json!(payload.len()); }
    s.write_all(format!("{h}\n").as_bytes())?;
    s.write_all(payload)?;
    Ok(())
}

pub fn transcribe(addr: &str, pcm: &[i16]) -> Result<String> {
    let mut s = TcpStream::connect(addr)?;
    s.set_read_timeout(Some(Duration::from_secs(60)))?;
    let fmt = json!({"rate": 16000, "width": 2, "channels": 1});
    send(&mut s, "transcribe", json!({"language": "en"}), &[])?;
    send(&mut s, "audio-start", fmt.clone(), &[])?;
    let bytes: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
    for c in bytes.chunks(32_000) { send(&mut s, "audio-chunk", fmt.clone(), c)?; }
    send(&mut s, "audio-stop", json!({}), &[])?;
    let mut r = BufReader::new(s);
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 { bail!("whisper closed the connection"); }
        let h: Value = serde_json::from_str(&line)?;
        let mut data = h.get("data").cloned().unwrap_or(json!({}));
        if let Some(n) = h.get("data_length").and_then(Value::as_u64) {
            let mut b = vec![0; n as usize]; r.read_exact(&mut b)?; data = serde_json::from_slice(&b)?;
        }
        if let Some(n) = h.get("payload_length").and_then(Value::as_u64) {
            let mut b = vec![0; n as usize]; r.read_exact(&mut b)?;
        }
        if h["type"] == "transcript" {
            return Ok(data["text"].as_str().unwrap_or("").trim().to_string());
        }
    }
}
