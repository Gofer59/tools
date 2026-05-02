use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tempfile::NamedTempFile;

pub struct RecordHandle {
    join: std::thread::JoinHandle<Result<NamedTempFile>>,
}

pub fn start_recording(stop: Arc<AtomicBool>) -> Result<RecordHandle> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("no input device")?;
    let config = device.default_input_config().context("default input config")?;
    let sample_rate = config.sample_rate().0;
    let channels    = config.channels();

    let full: Arc<std::sync::Mutex<Vec<f32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = stop.clone();
    let join = std::thread::spawn(move || -> Result<NamedTempFile> {
        record_until_stop(s, full, sample_rate, channels)
    });
    Ok(RecordHandle { join })
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
    full: Arc<std::sync::Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
) -> Result<NamedTempFile> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("no input device")?;
    let config = device.default_input_config().context("default input config")?;

    let f2 = full.clone(); let f3 = full.clone(); let f4 = full.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                f2.lock().unwrap().extend_from_slice(data);
            },
            |e| eprintln!("[audio_in] stream error: {e}"),
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _| {
                let v: Vec<f32> = data.iter().map(|&x| x as f32 / i16::MAX as f32).collect();
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
