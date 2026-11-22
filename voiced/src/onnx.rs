//! Small helpers around `ort`: one shared environment, single-threaded sessions, anyhow errors.
use anyhow::{anyhow, Result};
use ort::{environment::Environment, session::Session};

/// `ort` errors carry raw pointers (not Send/Sync), so convert them to plain messages.
pub trait OrtResult<T> {
    fn o(self) -> Result<T>;
}

impl<T, R> OrtResult<T> for std::result::Result<T, ort::Error<R>> {
    fn o(self) -> Result<T> {
        self.map_err(|e| anyhow!("onnx: {e}"))
    }
}

pub fn env() -> Result<Environment> {
    ort::init().with_name("voiced").build().o()
}

pub fn session(env: &Environment, path: &str) -> Result<Session> {
    Session::builder(env)
        .o()?
        .with_intra_threads(1)
        .o()?
        .commit_from_file(path)
        .o()
        .map_err(|e| anyhow!("{path}: {e}"))
}
