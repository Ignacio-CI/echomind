use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
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
/// Sends 5-second 16 kHz mono f32 PCM chunks to the transcription pipeline.
pub static CHUNK_TX: Mutex<Option<mpsc::SyncSender<Vec<f32>>>> = Mutex::new(None);

pub fn list_devices() -> Result<Vec<AudioDevice>, AppError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| AppError::Audio(e.to_string()))?;
    Ok(devices
        .filter_map(|d| d.name().ok().map(|name| AudioDevice { name }))
        .collect())
}

/// Starts audio capture. Returns a receiver for 5-second PCM chunks (16 kHz mono).
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

    let sample_rate = config.sample_rate.0 as f32;
    let frame_size = (sample_rate * 0.05) as usize; // 50 ms window for RMS

    // Target: 5 s at the device's native sample rate, downsampled to 16 kHz later
    let chunk_samples = (sample_rate * 5.0) as usize;

    let rb = HeapRb::<f32>::new(chunk_samples * 2);
    let (mut prod, mut cons) = rb.split();

    let stream = build_stream(&device, &config, move |data: &[f32]| {
        for &s in data {
            let _ = prod.try_push(s);
        }
    })
    .map_err(|e| AppError::Audio(e.to_string()))?;

    stream
        .play()
        .map_err(|e| AppError::Audio(e.to_string()))?;

    let (chunk_tx, chunk_rx) = mpsc::sync_channel::<Vec<f32>>(4);
    *CHUNK_TX.lock().unwrap() = Some(chunk_tx);

    // Background thread: emits RMS level events + sends 5-second chunks
    std::thread::spawn(move || {
        let mut level_buf = vec![0f32; frame_size];
        let mut accumulator: Vec<f32> = Vec::with_capacity(chunk_samples);
        let target_sr = 16000.0_f32;
        let downsample_ratio = (sample_rate / target_sr).round() as usize;

        loop {
            std::thread::sleep(Duration::from_millis(50));
            let n = cons.pop_slice(&mut level_buf);
            if n == 0 {
                continue;
            }

            // RMS level event
            let rms = (level_buf[..n].iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt();
            let _ = app.emit(AUDIO_LEVEL, rms);

            // Accumulate downsampled samples into the 5-second chunk buffer
            let downsampled = level_buf[..n]
                .chunks(downsample_ratio.max(1))
                .map(|c| c[0]);
            accumulator.extend(downsampled);

            // When we have 5 s of 16 kHz audio, send the chunk
            let target_chunk_len = (target_sr * 5.0) as usize;
            if accumulator.len() >= target_chunk_len {
                let chunk: Vec<f32> = accumulator.drain(..target_chunk_len).collect();
                // Keep 0.5-second overlap for next window
                let overlap = (target_sr * 0.5) as usize;
                accumulator.splice(0..0, chunk[target_chunk_len - overlap..].iter().copied());
                if let Ok(tx) = CHUNK_TX.lock() {
                    if let Some(ref s) = *tx {
                        let _ = s.try_send(chunk);
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
    *RECORDER.lock().unwrap() = None; // drops Stream
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
