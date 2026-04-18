use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use rubato::{FftFixedIn, Resampler};
use serde::Serialize;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::error::AppError;
use crate::events::AUDIO_LEVEL;

#[derive(Debug, Serialize, Clone)]
pub struct AudioDevice {
    pub name: String,
}

pub struct Recorder {
    _stream: Stream,
}

// SAFETY: cpal Stream contains a raw pointer but is only used from the owning thread.
unsafe impl Send for Recorder {}

static RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);
pub static CHUNK_TX: Mutex<Option<mpsc::SyncSender<Vec<f32>>>> = Mutex::new(None);

pub fn is_recording() -> bool {
    RECORDER.lock().unwrap().is_some()
}

pub fn list_devices() -> Result<Vec<AudioDevice>, AppError> {
    let host = cpal::default_host();
    let devices = host.input_devices().map_err(|e| AppError::Audio(e.to_string()))?;
    Ok(devices
        .filter_map(|d| d.name().ok().map(|name| AudioDevice { name }))
        .collect())
}

pub fn start(app: AppHandle) -> Result<mpsc::Receiver<Vec<f32>>, AppError> {
    let mut guard = RECORDER.lock().unwrap();
    if guard.is_some() {
        return Err(AppError::Audio("Already recording".into()));
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AppError::Audio("No default input device".into()))?;

    let config: StreamConfig = device
        .default_input_config()
        .map_err(|e| AppError::Audio(e.to_string()))?
        .into();

    let sample_rate = config.sample_rate.0 as usize;
    let frame_size = sample_rate / 20; // 50 ms at device rate

    let rb = HeapRb::<f32>::new(sample_rate * 10); // 10 s buffer
    let (mut prod, mut cons) = rb.split();

    let stream = build_stream(&device, &config, move |data: &[f32]| {
        for &s in data {
            let _ = prod.try_push(s);
        }
    })
    .map_err(|e| AppError::Audio(e.to_string()))?;

    stream.play().map_err(|e| AppError::Audio(e.to_string()))?;

    let (chunk_tx, chunk_rx) = mpsc::sync_channel::<Vec<f32>>(4);
    *CHUNK_TX.lock().unwrap() = Some(chunk_tx);

    std::thread::spawn(move || {
        const TARGET_SR: usize = 16000;
        const CHUNK_SECS: usize = 5;
        const OVERLAP_SECS: f32 = 0.5;

        let mut resampler = FftFixedIn::<f32>::new(sample_rate, TARGET_SR, frame_size, 2, 1)
            .expect("Failed to create resampler");

        let mut read_buf = vec![0f32; frame_size];
        let mut input_spill: Vec<f32> = Vec::new();
        let mut accumulator: Vec<f32> = Vec::new();
        let target_len = TARGET_SR * CHUNK_SECS;
        let overlap_len = (TARGET_SR as f32 * OVERLAP_SECS) as usize;

        loop {
            std::thread::sleep(Duration::from_millis(50));
            let n = cons.pop_slice(&mut read_buf);
            if n == 0 {
                continue;
            }

            // RMS level event
            let rms = (read_buf[..n].iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
            let _ = app.emit(AUDIO_LEVEL, rms);

            // Buffer samples until we have a full frame for the resampler
            input_spill.extend_from_slice(&read_buf[..n]);
            while input_spill.len() >= frame_size {
                let chunk: Vec<f32> = input_spill.drain(..frame_size).collect();
                match resampler.process(&[chunk], None) {
                    Ok(out) => accumulator.extend_from_slice(&out[0]),
                    Err(e) => eprintln!("resample error: {e}"),
                }
            }

            if accumulator.len() >= target_len {
                let outchunk: Vec<f32> = accumulator.drain(..target_len).collect();
                // Keep overlap for next window
                let tail = outchunk[target_len - overlap_len..].to_vec();
                accumulator.splice(0..0, tail);
                if let Ok(tx) = CHUNK_TX.lock() {
                    if let Some(ref s) = *tx {
                        let _ = s.try_send(outchunk);
                    }
                }
            }
        }
    });

    *guard = Some(Recorder { _stream: stream });
    Ok(chunk_rx)
}

pub fn stop() -> Result<(), AppError> {
    *CHUNK_TX.lock().unwrap() = None;
    *RECORDER.lock().unwrap() = None;
    Ok(())
}

fn build_stream(
    device: &Device,
    config: &StreamConfig,
    mut data_cb: impl FnMut(&[f32]) + Send + 'static,
) -> Result<Stream, cpal::BuildStreamError> {
    let cfg = config.clone();
    device.build_input_stream(
        &cfg,
        move |data: &[f32], _| data_cb(data),
        |e| eprintln!("audio stream error: {e}"),
        None,
    )
}
