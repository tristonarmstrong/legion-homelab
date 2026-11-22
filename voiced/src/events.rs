//! Broadcasts state changes as JSON over a local WebSocket (ws://127.0.0.1:7071) for the face app.
use serde_json::{json, Value};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tungstenite::{accept, Message, WebSocket};

#[derive(Clone, Default)]
pub struct Events { clients: Arc<Mutex<Vec<WebSocket<TcpStream>>>>, last: Arc<Mutex<String>> }

impl Events {
    pub fn serve(addr: &str) -> Self {
        let ev = Events::default();
        let listener = TcpListener::bind(addr).expect("bind events socket");
        let e = ev.clone();
        thread::spawn(move || for stream in listener.incoming().flatten() {
            if let Ok(mut ws) = accept(stream) {
                let last = e.last.lock().unwrap().clone();
                if !last.is_empty() { let _ = ws.send(Message::text(last)); }
                e.clients.lock().unwrap().push(ws);
            }
        });
        ev
    }

    pub fn emit(&self, state: &str, extra: Value) {
        let mut msg = json!({"state": state});
        if let (Some(m), Some(x)) = (msg.as_object_mut(), extra.as_object()) {
            for (k, v) in x { m.insert(k.clone(), v.clone()); }
        }
        let text = msg.to_string();
        eprintln!("[{state}] {extra}");
        *self.last.lock().unwrap() = text.clone();
        self.clients.lock().unwrap().retain_mut(|ws| ws.send(Message::text(text.clone())).is_ok());
    }
}
