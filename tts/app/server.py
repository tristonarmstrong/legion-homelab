"""tiny-voca TTS service: POST /speak {"text", "voice": "F2"|"M2", "speed"} -> audio/wav (24 kHz mono, 16-bit)."""
import io, json, os, re, wave
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import numpy as np, onnxruntime as ort
os.environ.setdefault("TINY_VOCA_FRONTEND", "own")
import tv_text
tv_text.set_frontend("own")

W = os.environ.get("WEIGHTS", "/weights")
CFG = json.load(open(f"{W}/config.json"))
RATE = CFG["sample_rate"]
VOICES = {v: i for i, v in enumerate(CFG["voices"])}
opts = ort.SessionOptions(); opts.intra_op_num_threads = int(os.environ.get("THREADS", "4"))
SESSION = ort.InferenceSession(f"{W}/onnx/model.onnx", opts, providers=["CPUExecutionProvider"])
SENTENCE = re.compile(r"(?<=[.!?…])\s+")

def synth(text, voice="F2", speed=1.0):
    chunks = []
    for piece in SENTENCE.split(re.sub(r"\s+", " ", text).strip()):
        if not piece: continue
        try: ph = tv_text.phonemize(piece)
        except tv_text.NothingToPronounce: continue
        audio, _ = SESSION.run(None, {
            "tokens": np.array([ph.ids], np.int64), "stress": np.array([ph.stress], np.int64),
            "voice": np.array([VOICES.get(voice, 0)], np.int64), "speed": np.array([speed], np.float32),
            "pitch_shift": np.array([0.0], np.float32), "pitch_range": np.array([1.0], np.float32)})
        chunks += [audio[0], np.zeros(int(RATE * 0.12), np.float32)]
    wav = np.concatenate(chunks) if chunks else np.zeros(1, np.float32)
    wav = wav / max(1.0, float(np.abs(wav).max()) / 0.99)
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
        w.writeframes((wav * 32767).astype(np.int16).tobytes())
    return buf.getvalue()

class H(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200); self.end_headers(); self.wfile.write(b"ok")
    def do_POST(self):
        try:
            req = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
            body = synth(req["text"], req.get("voice", "F2"), float(req.get("speed", 1.0)))
            self.send_response(200); self.send_header("Content-Type", "audio/wav")
            self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)
        except Exception as e:
            msg = str(e).encode(); self.send_response(400); self.end_headers(); self.wfile.write(msg)
    def log_message(self, *a): pass

if __name__ == "__main__":
    synth("Warming up.")
    ThreadingHTTPServer(("127.0.0.1", int(os.environ.get("PORT", "10210"))), H).serve_forever()
