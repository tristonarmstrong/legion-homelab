//! Silero VAD v5 (512-sample windows at 16 kHz with 64 samples of context).
use crate::onnx::{session, OrtResult};
use anyhow::Result;
use ort::{environment::Environment, session::Session, value::Tensor};

pub struct Vad {
    s: Session,
    state: Vec<f32>,
    ctx: Vec<f32>,
    pending: Vec<f32>,
}

impl Vad {
    pub fn new(env: &Environment, path: &str) -> Result<Self> {
        Ok(Self { s: session(env, path)?, state: vec![0.0; 2 * 128], ctx: vec![0.0; 64], pending: Vec::new() })
    }

    pub fn reset(&mut self) {
        self.state.fill(0.0);
        self.ctx.fill(0.0);
        self.pending.clear();
    }

    /// Highest speech probability among the complete windows in this chunk.
    pub fn process(&mut self, chunk: &[i16]) -> Result<f32> {
        self.pending.extend(chunk.iter().map(|&s| s as f32 / 32768.0));
        let mut best = 0f32;
        while self.pending.len() >= 512 {
            let win: Vec<f32> = self.pending.drain(..512).collect();
            let mut x = self.ctx.clone();
            x.extend_from_slice(&win);
            self.ctx.copy_from_slice(&win[512 - 64..]);
            let out = self
                .s
                .run(ort::inputs![
                    "input" => Tensor::from_array(([1usize, 576], x)).o()?,
                    "state" => Tensor::from_array(([2usize, 1, 128], self.state.clone())).o()?,
                    "sr" => Tensor::from_array((Vec::<usize>::new(), vec![16000i64])).o()?,
                ])
                .o()?;
            let (_, p) = out["output"].try_extract_tensor::<f32>().o()?;
            best = best.max(p[0]);
            let (_, st) = out["stateN"].try_extract_tensor::<f32>().o()?;
            self.state.copy_from_slice(st);
        }
        Ok(best)
    }
}
