use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex as PMutex;
use tempfile::NamedTempFile;

pub struct RingBuffer {
    buf: Vec<f32>,
    capacity: usize,
    write: usize,
    filled: usize,
    pub sample_rate: u32,
    pub channels: u16,
}

impl RingBuffer {
    pub const RING_SECONDS: f32 = 8.0;

    pub fn new(sample_rate: u32, channels: u16) -> Self {
        let capacity = (sample_rate as f32 * Self::RING_SECONDS) as usize * channels as usize;
        Self { buf: vec![0.0; capacity], capacity, write: 0, filled: 0, sample_rate, channels }
    }

    pub fn extend(&mut self, samples: &[f32]) {
        for &s in samples {
            self.buf[self.write] = s;
            self.write = (self.write + 1) % self.capacity;
            if self.filled < self.capacity { self.filled += 1; }
        }
    }

    pub fn snapshot_mono(&self, seconds: f32) -> Vec<f32> {
        let want_frames  = ((self.sample_rate as f32) * seconds) as usize;
        let want_samples = want_frames * self.channels as usize;
        let take = want_samples.min(self.filled);
        if take == 0 { return Vec::new(); }
        let start = (self.write + self.capacity - take) % self.capacity;
        let mut interleaved = Vec::with_capacity(take);
        if start + take <= self.capacity {
            interleaved.extend_from_slice(&self.buf[start..start + take]);
        } else {
            let head = self.capacity - start;
            interleaved.extend_from_slice(&self.buf[start..]);
            interleaved.extend_from_slice(&self.buf[..take - head]);
        }
        let ch = self.channels as usize;
        if ch == 1 { return interleaved; }
        let frames = interleaved.len() / ch;
        let mut mono = Vec::with_capacity(frames);
        for f in 0..frames {
            let mut acc = 0.0f32;
            for c in 0..ch { acc += interleaved[f * ch + c]; }
            mono.push(acc / ch as f32);
        }
        mono
    }
}

pub struct RecordHandle {
    pub ring: Arc<PMutex<RingBuffer>>,
    pub full: Arc<std::sync::Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    pub channels: u16,
    join: std::thread::JoinHandle<Result<NamedTempFile>>,
}

pub fn start_recording(stop: Arc<AtomicBool>) -> Result<RecordHandle> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("no input device")?;
    let config = device.default_input_config().context("default input config")?;
    let sample_rate = config.sample_rate().0;
    let channels    = config.channels();

    let ring = Arc::new(PMutex::new(RingBuffer::new(sample_rate, channels)));
    let full: Arc<std::sync::Mutex<Vec<f32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

    let r = ring.clone(); let f = full.clone(); let s = stop.clone();
    let join = std::thread::spawn(move || -> Result<NamedTempFile> {
        record_until_stop(s, r, f, sample_rate, channels)
    });

    Ok(RecordHandle { ring, full, sample_rate, channels, join })
}

pub fn finish_recording(h: RecordHandle) -> Result<NamedTempFile> {
    h.join.join().map_err(|_| anyhow::anyhow!("record thread panicked"))?
}

pub fn write_wav(path: &std::path::Path, samples: &[f32], sample_rate: u32, channels: u16) -> Result<()> {
    let spec = hound::WavSpec {
        channels, sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).context("wav writer")?;
    for &s in samples {
        let clamped = (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        writer.write_sample(clamped).context("write sample")?;
    }
    writer.finalize().context("wav finalize")?;
    Ok(())
}

fn record_until_stop(
    stop: Arc<AtomicBool>,
    ring: Arc<PMutex<RingBuffer>>,
    full: Arc<std::sync::Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
) -> Result<NamedTempFile> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("no input device")?;
    let config = device.default_input_config().context("default input config")?;

    let r2 = ring.clone(); let r3 = ring.clone(); let r4 = ring.clone();
    let f2 = full.clone(); let f3 = full.clone(); let f4 = full.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                r2.lock().extend(data);
                f2.lock().unwrap().extend_from_slice(data);
            },
            |e| eprintln!("[audio_in] stream error: {e}"),
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _| {
                let v: Vec<f32> = data.iter().map(|&x| x as f32 / i16::MAX as f32).collect();
                r3.lock().extend(&v);
                f3.lock().unwrap().extend_from_slice(&v);
            },
            |e| eprintln!("[audio_in] stream error: {e}"),
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _| {
                let v: Vec<f32> = data.iter()
                    .map(|&x| (x as f32 / u16::MAX as f32) * 2.0 - 1.0)
                    .collect();
                r4.lock().extend(&v);
                f4.lock().unwrap().extend_from_slice(&v);
            },
            |e| eprintln!("[audio_in] stream error: {e}"),
            None,
        )?,
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play().context("start stream")?;

    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(stream);

    let collected = full.lock().unwrap().clone();
    let tmp = NamedTempFile::new().context("tempfile")?;
    write_wav(tmp.path(), &collected, sample_rate, channels)?;
    Ok(tmp)
}
