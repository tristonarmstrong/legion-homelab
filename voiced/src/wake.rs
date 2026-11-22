//! openWakeWord streaming pipeline: audio -> melspectrogram -> embeddings -> wake word score.
use crate::onnx::{session, OrtResult};
use anyhow::Result;
use ort::{environment::Environment, session::Session, value::Tensor};

pub struct Wake {
    mel: Session,
    emb: Session,
    ww: Session,
    // Community models are exported by different tools, so their tensor names vary.
    ww_in: String,
    ww_out: String,
    raw: Vec<f32>,
    mels: Vec<[f32; 32]>,
    embs: Vec<[f32; 96]>,
}

impl Wake {
    pub fn new(env: &Environment, dir: &str, model: &str) -> Result<Self> {
        let ww = session(env, &format!("{dir}/{model}.onnx"))?;
        let ww_in = ww.inputs()[0].name().to_string();
        let ww_out = ww.outputs()[0].name().to_string();
        Ok(Self {
            mel: session(env, &format!("{dir}/melspectrogram.onnx"))?,
            emb: session(env, &format!("{dir}/embedding_model.onnx"))?,
            ww,
            ww_in,
            ww_out,
            raw: vec![0.0; 1280 + 480],
            mels: vec![[1.0; 32]; 76],
            embs: Vec::new(),
        })
    }

    /// Feed one 1280-sample chunk; returns the wake word probability (0 until 16 embeddings exist).
    pub fn process(&mut self, chunk: &[i16]) -> Result<f32> {
        self.raw.drain(..chunk.len());
        self.raw.extend(chunk.iter().map(|&s| s as f32));

        let input = Tensor::from_array(([1usize, self.raw.len()], self.raw.clone())).o()?;
        let out = self.mel.run(ort::inputs!["input" => input]).o()?;
        let (_, m) = out["output"].try_extract_tensor::<f32>().o()?;
        for f in m.chunks_exact(32) {
            let mut row = [0f32; 32];
            for (r, v) in row.iter_mut().zip(f) {
                *r = v / 10.0 + 2.0;
            }
            self.mels.push(row);
        }
        drop(out);
        let excess = self.mels.len().saturating_sub(200);
        self.mels.drain(..excess);

        let window: Vec<f32> = self.mels[self.mels.len() - 76..].iter().flatten().copied().collect();
        let input = Tensor::from_array(([1usize, 76, 32, 1], window)).o()?;
        let out = self.emb.run(ort::inputs!["input_1" => input]).o()?;
        let (_, e) = out["conv2d_19"].try_extract_tensor::<f32>().o()?;
        let mut row = [0f32; 96];
        row.copy_from_slice(&e[..96]);
        drop(out);
        self.embs.push(row);
        if self.embs.len() > 16 {
            self.embs.remove(0);
        }
        if self.embs.len() < 16 {
            return Ok(0.0);
        }

        let feats: Vec<f32> = self.embs.iter().flatten().copied().collect();
        let input = Tensor::from_array(([1usize, 16, 96], feats)).o()?;
        let out = self.ww.run(ort::inputs![self.ww_in.as_str() => input]).o()?;
        let (_, s) = out[self.ww_out.as_str()].try_extract_tensor::<f32>().o()?;
        Ok(s[0])
    }

    /// Forget recent audio so the same utterance can't trigger twice.
    pub fn reset(&mut self) {
        self.embs.clear();
    }
}
