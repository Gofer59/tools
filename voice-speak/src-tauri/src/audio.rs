use anyhow::Result;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Arc<Mutex<Option<Sink>>>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, handle) =
            OutputStream::try_default().map_err(|e| anyhow::anyhow!("audio output: {e}"))?;
        Ok(Self {
            _stream,
            handle,
            sink: Arc::new(Mutex::new(None)),
        })
    }

    /// Play a list of PCM chunks. Replaces any currently playing audio.
    pub async fn play(&self, chunks: Vec<crate::piper::PcmChunk>) -> Result<()> {
        let sink = Sink::try_new(&self.handle)
            .map_err(|e| anyhow::anyhow!("sink: {e}"))?;

        for chunk in chunks {
            let samples: Vec<f32> = chunk.samples.iter().map(|&s| s as f32 / 32768.0).collect();
            let source = rodio::buffer::SamplesBuffer::new(1, chunk.sample_rate, samples);
            sink.append(source);
        }

        let mut guard = self.sink.lock().await;
        // Stop previous playback
        if let Some(old) = guard.take() {
            old.stop();
        }
        *guard = Some(sink);
        Ok(())
    }

    /// Stop current playback immediately.
    pub async fn stop(&self) {
        let mut guard = self.sink.lock().await;
        if let Some(sink) = guard.take() {
            sink.stop();
        }
    }

    /// Returns true if audio is currently playing.
    pub async fn is_playing(&self) -> bool {
        let guard = self.sink.lock().await;
        guard.as_ref().map(|s| !s.empty()).unwrap_or(false)
    }

    /// Resolves when the current sink drains naturally (or is already empty).
    pub async fn wait_until_done(&self) {
        loop {
            if !self.is_playing().await {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
