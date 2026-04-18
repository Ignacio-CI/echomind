use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{audio, model, Config, HOP_LENGTH, N_FFT, SAMPLE_RATE};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use once_cell::sync::OnceCell;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;

use crate::error::AppError;

const MODEL_REPO: &str = "openai/whisper-small";
const MODEL_REVISION: &str = "main";

struct WhisperModel {
    model: model::Whisper,
    tokenizer: Tokenizer,
    config: Config,
    mel_filters: Vec<f32>,
    device: Device,
}

// candle Tensors are Send; the Metal device is per-thread but OnceCell ensures single init.
unsafe impl Send for WhisperModel {}
unsafe impl Sync for WhisperModel {}

static MODEL: OnceCell<Mutex<WhisperModel>> = OnceCell::new();

/// Mel triangular filterbank — shape [n_mels, n_fft].
/// The candle audio loop indexes as `filters[j * n_fft + k]` where k < n_fft.
fn compute_mel_filters(n_mels: usize, n_fft: usize, sample_rate: f32) -> Vec<f32> {
    let fmax = sample_rate / 2.0;
    let hz_to_mel = |f: f32| 2595.0 * (1.0 + f / 700.0).log10();
    let mel_to_hz = |m: f32| 700.0 * (10.0_f32.powf(m / 2595.0) - 1.0);

    let mel_min = 0.0_f32;
    let mel_max = hz_to_mel(fmax);
    // n_mels + 2 evenly spaced mel points
    let mel_pts: Vec<f32> = (0..=(n_mels + 1))
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();
    let hz_pts: Vec<f32> = mel_pts.iter().map(|&m| mel_to_hz(m)).collect();
    let bins: Vec<usize> = hz_pts
        .iter()
        .map(|&f| ((n_fft as f32 + 1.0) * f / sample_rate).floor() as usize)
        .collect();

    let mut filters = vec![0.0_f32; n_mels * n_fft];
    for m in 0..n_mels {
        let (fl, fc, fr) = (bins[m], bins[m + 1], bins[m + 2]);
        for k in fl..fc {
            if fc > fl && k < n_fft {
                filters[m * n_fft + k] = (k - fl) as f32 / (fc - fl) as f32;
            }
        }
        for k in fc..fr {
            if fr > fc && k < n_fft {
                filters[m * n_fft + k] = (fr - k) as f32 / (fr - fc) as f32;
            }
        }
    }
    filters
}

pub async fn load() -> Result<(), AppError> {
    if MODEL.get().is_some() {
        return Ok(());
    }

    let api = Api::new().map_err(|e| AppError::Ai(e.to_string()))?;
    let repo = api.repo(Repo::with_revision(
        MODEL_REPO.to_string(),
        RepoType::Model,
        MODEL_REVISION.to_string(),
    ));

    let config_path = repo
        .get("config.json")
        .await
        .map_err(|e| AppError::Ai(e.to_string()))?;
    let model_path = repo
        .get("model.safetensors")
        .await
        .map_err(|e| AppError::Ai(e.to_string()))?;
    let tok_path = repo
        .get("tokenizer.json")
        .await
        .map_err(|e| AppError::Ai(e.to_string()))?;

    let config: Config = serde_json::from_str(
        &std::fs::read_to_string(&config_path).map_err(|e| AppError::Ai(e.to_string()))?,
    )
    .map_err(|e| AppError::Ai(e.to_string()))?;

    let device = Device::new_metal(0).unwrap_or(Device::Cpu);

    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[&model_path], DType::F32, &device)
            .map_err(|e| AppError::Ai(e.to_string()))?
    };

    let whisper = model::Whisper::load(&vb, config.clone())
        .map_err(|e| AppError::Ai(e.to_string()))?;

    let tokenizer =
        Tokenizer::from_file(&tok_path).map_err(|e| AppError::Ai(e.to_string()))?;

    let mel_filters = compute_mel_filters(config.num_mel_bins, N_FFT, SAMPLE_RATE as f32);

    MODEL
        .set(Mutex::new(WhisperModel {
            model: whisper,
            tokenizer,
            config,
            mel_filters,
            device,
        }))
        .ok();

    Ok(())
}

/// Transcribe a 16 kHz mono f32 PCM slice. Returns the recognised text.
pub async fn transcribe(pcm: Vec<f32>) -> Result<String, AppError> {
    let mut guard = MODEL
        .get()
        .ok_or_else(|| AppError::Ai("Whisper not loaded".into()))?
        .lock()
        .await;

    let mel = audio::pcm_to_mel(&guard.config, &pcm, &guard.mel_filters);
    let n_mels = guard.config.num_mel_bins;
    let n_frames = mel.len() / n_mels;

    let mel_t =
        Tensor::from_vec(mel, (1, n_mels, n_frames), &guard.device)
            .map_err(|e| AppError::Ai(e.to_string()))?;

    let sot = guard
        .tokenizer
        .token_to_id("<|startoftranscript|>")
        .unwrap_or(50258);
    let eot = guard
        .tokenizer
        .token_to_id("<|endoftext|>")
        .unwrap_or(50256);

    let features = guard
        .model
        .encoder
        .forward(&mel_t, true)
        .map_err(|e| AppError::Ai(e.to_string()))?;

    let mut tokens = vec![sot];

    for _ in 0..224usize {
        let t = Tensor::new(tokens.as_slice(), &guard.device)
            .map_err(|e| AppError::Ai(e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| AppError::Ai(e.to_string()))?;

        let logits = guard
            .model
            .decoder
            .forward(&t, &features, true)
            .map_err(|e| AppError::Ai(e.to_string()))?;

        // logits shape: [1, seq, vocab] — pick the last position
        let last = logits
            .i((0, logits.dims()[1] - 1))
            .map_err(|e| AppError::Ai(e.to_string()))?;

        let next = last
            .argmax(D::Minus1)
            .map_err(|e| AppError::Ai(e.to_string()))?
            .to_scalar::<u32>()
            .map_err(|e| AppError::Ai(e.to_string()))?;

        if next == eot {
            break;
        }
        tokens.push(next);
    }

    let text = guard
        .tokenizer
        .decode(&tokens[1..], true)
        .map_err(|e| AppError::Ai(e.to_string()))?;

    Ok(text.trim().to_string())
}
