//! Streams a reply from Hermes' Responses API and hands back speakable sentences as they arrive.
//!
//! Turns are chained with `previous_response_id`, so Hermes keeps the whole conversation on its side,
//! including its own tool calls and results (otherwise it forgets that it opened something on screen).
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::sync::mpsc::Sender;

pub enum Chunk {
    Sentence(String),
    Tool(String),
    /// Full reply text and the response id to chain the next turn to.
    Done(String, Option<String>),
    Failed(String),
}

pub struct Request {
    pub url: String,
    pub key: String,
    pub instructions: String,
    pub input: String,
    pub previous: Option<String>,
}

pub fn stream(req: Request, tx: Sender<Chunk>) {
    if let Err(e) = run(&req, &tx) {
        let _ = tx.send(Chunk::Failed(e.to_string()));
    }
}

fn run(req: &Request, tx: &Sender<Chunk>) -> Result<()> {
    let mut body = json!({
        "model": "hermes-agent",
        "stream": true,
        "instructions": req.instructions,
        "input": req.input,
    });
    if let Some(prev) = &req.previous {
        body["previous_response_id"] = json!(prev);
    }
    let mut resp = ureq::post(&req.url).header("Authorization", &format!("Bearer {}", req.key)).send_json(body)?;
    let reader = BufReader::new(resp.body_mut().as_reader());
    let (mut pending, mut full, mut id) = (String::new(), String::new(), None);
    for line in reader.lines() {
        let line = line?;
        let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
        if data == "[DONE]" {
            break;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
        match v["type"].as_str().unwrap_or("") {
            "response.created" | "response.completed" => {
                if let Some(i) = v["response"]["id"].as_str() {
                    id = Some(i.to_string());
                }
            }
            "response.output_item.added" if v["item"]["type"] == "function_call" => {
                let name = v["item"]["name"].as_str().unwrap_or("working");
                let _ = tx.send(Chunk::Tool(name.to_string()));
            }
            "response.output_text.delta" => {
                let d = v["delta"].as_str().unwrap_or("");
                full.push_str(d);
                pending.push_str(d);
                while let Some(i) = sentence_end(&pending) {
                    let s: String = pending.drain(..=i).collect();
                    let s = speakable(&s);
                    if !s.is_empty() && !is_command(&s) {
                        let _ = tx.send(Chunk::Sentence(s));
                    }
                }
            }
            "response.failed" | "error" => {
                anyhow::bail!("hermes: {}", v);
            }
            _ => {}
        }
    }
    let rest = speakable(&pending);
    if !rest.is_empty() && !is_command(&rest) {
        let _ = tx.send(Chunk::Sentence(rest));
    }
    let _ = tx.send(Chunk::Done(full, id));
    Ok(())
}

/// The model sometimes echoes a shell command it ran (e.g. "screen-close") into its reply; never say those.
fn is_command(s: &str) -> bool {
    let first = s.split_whitespace().next().unwrap_or("");
    first.starts_with("screen-") || first == "sudo" || first == "hyprctl"
}

/// Byte index of the end of the first complete sentence (punctuation then whitespace, or a newline).
fn sentence_end(s: &str) -> Option<usize> {
    let b: Vec<(usize, char)> = s.char_indices().collect();
    for w in b.windows(2) {
        let ((i, c), (_, n)) = (w[0], w[1]);
        if c == '\n' || (matches!(c, '.' | '!' | '?') && n.is_whitespace()) {
            return Some(i + c.len_utf8() - 1);
        }
    }
    None
}

/// Strips markdown, links and symbols that sound bad when read aloud.
fn speakable(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '#' | '`' | '_' | '>' | '|' | '~' => {}
            '[' => {
                // [text](url) -> text
                let t: String = chars.by_ref().take_while(|&c| c != ']').collect();
                out.push_str(&t);
                if chars.peek() == Some(&'(') {
                    for c in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                    }
                }
            }
            _ => out.push(c),
        }
    }
    let out = out.split_whitespace().filter(|w| !w.starts_with("http")).collect::<Vec<_>>().join(" ");
    out.trim_start_matches(['-', '•', ' ']).trim().to_string()
}
