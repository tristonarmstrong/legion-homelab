//! Speaks a reply sentence by sentence on its own thread, so the main loop can keep listening for
//! "Hey Luna" and cut her off mid-word.
use crate::{audio, events::Events};
use serde_json::json;
use std::io::Write;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long after her voice stops the room can still echo it (and the wake model, which looks at
/// the last ~1.3 s of audio, can still hear it).
const ECHO_TAIL: Duration = Duration::from_millis(1500);

#[derive(Default)]
struct State {
    child: Option<Child>,
    playing: bool,
    last_sound: Option<Instant>,
    /// While she says her own name (and for ECHO_TAIL after), the wake word is ignored.
    name_until: Option<Instant>,
}

pub struct Speaker {
    tx: Option<Sender<String>>,
    spoken_rx: Receiver<()>,
    spoken: usize,
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
}

impl Speaker {
    pub fn start(tts: String, voice: String, ev: Events, cancel: Arc<AtomicBool>) -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (spoken_tx, spoken_rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(State::default()));
        let (st, c) = (state.clone(), cancel.clone());
        thread::spawn(move || {
            for text in rx {
                if c.load(Ordering::SeqCst) {
                    break;
                }
                ev.emit("speaking", json!({"text": text}));
                speak(&tts, &voice, &text, &st, &c);
                let _ = spoken_tx.send(());
            }
        });
        Self { tx: Some(tx), spoken_rx, spoken: 0, state, cancel }
    }

    /// Queues a sentence.
    pub fn say(&self, text: String) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(text);
        }
    }

    /// How many queued sentences have been spoken (or skipped) so far.
    pub fn spoken(&mut self) -> usize {
        while self.spoken_rx.try_recv().is_ok() {
            self.spoken += 1;
        }
        self.spoken
    }

    /// Stops the current sentence now and drops the rest.
    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.tx = None;
        if let Some(child) = self.state.lock().unwrap().child.as_mut() {
            let _ = child.kill();
        }
    }

    /// The wake threshold to use right now, so her own voice can't wake her:
    /// `normal` while she is quiet, `talking` while she speaks (and just after), never while she says "Luna".
    pub fn wake_threshold(&self, normal: f32, talking: f32) -> f32 {
        let st = self.state.lock().unwrap();
        let now = Instant::now();
        if st.name_until.is_some_and(|t| now < t) {
            f32::INFINITY
        } else if st.playing || st.last_sound.is_some_and(|t| now.duration_since(t) < ECHO_TAIL) {
            talking
        } else {
            normal
        }
    }
}

fn speak(tts: &str, voice: &str, text: &str, st: &Mutex<State>, cancel: &AtomicBool) {
    let wav = ureq::post(tts).send_json(json!({"text": text, "voice": voice})).and_then(|mut r| r.body_mut().read_to_vec());
    let pcm = match wav {
        Ok(w) if w.len() > 44 => w[44..].to_vec(),
        Ok(_) => return,
        Err(e) => {
            eprintln!("tts error: {e}");
            return;
        }
    };
    let says_name = text.to_lowercase().contains("luna");
    let stdin = {
        // Spawn under the lock, so a stop() either sees this child or makes us skip it.
        let mut s = st.lock().unwrap();
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        let mut child = match audio::player(24_000) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        let stdin = child.stdin.take();
        s.child = Some(child);
        s.playing = true;
        if says_name {
            s.name_until = Some(Instant::now() + Duration::from_secs(3600));
        }
        stdin
    };
    if let Some(mut stdin) = stdin {
        let _ = stdin.write_all(&pcm); // fails at once if stop() killed pacat
    }
    loop {
        let mut s = st.lock().unwrap();
        let done = s.child.as_mut().map_or(true, |c| !matches!(c.try_wait(), Ok(None)));
        if done {
            let now = Instant::now();
            s.child = None;
            s.playing = false;
            s.last_sound = Some(now);
            if says_name {
                s.name_until = Some(now + ECHO_TAIL);
            }
            return;
        }
        drop(s);
        thread::sleep(Duration::from_millis(20));
    }
}
