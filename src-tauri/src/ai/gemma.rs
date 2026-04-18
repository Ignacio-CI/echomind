use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::gemma3::{Config, Model};
use hf_hub::{Repo, RepoType};
use once_cell::sync::OnceCell;
use tokenizers::Tokenizer;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use crate::error::AppError;
use crate::events::AI_STATUS;

const MODEL_REPO: &str = "google/gemma-3-1b-it";
const MODEL_REVISION: &str = "main";

struct GemmaInner {
    model: Model,
    tokenizer: Tokenizer,
    device: Device,
    eos_token: u32,
}

pub struct GemmaEngine {
    inner: Option<Arc<Mutex<GemmaInner>>>,
    cancel_tx: Option<mpsc::Sender<()>>,
}

static ENGINE: OnceCell<Mutex<GemmaEngine>> = OnceCell::new();

impl GemmaEngine {
    pub fn new() -> Self {
        Self { inner: None, cancel_tx: None }
    }

    pub async fn load(&mut self, app: &AppHandle) -> Result<(), AppError> {
        if self.inner.is_some() {
            return Ok(());
        }

        let _ = app.emit(AI_STATUS, "Loading Gemma 3 model...");

        let api = match std::env::var("HF_TOKEN") {
            Ok(token) => hf_hub::api::tokio::ApiBuilder::new().with_token(Some(token)).build(),
            Err(_) => hf_hub::api::tokio::Api::new(),
        }
        .map_err(|e| AppError::Ai(format!("HF API error: {}", e)))?;

        let repo = api.repo(Repo::with_revision(
            MODEL_REPO.to_string(),
            RepoType::Model,
            MODEL_REVISION.to_string(),
        ));

        let config_path = repo.get("config.json").await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("403") {
                AppError::Ai(format!(
                    "Access denied (403) for {MODEL_REPO}. \
                     Accept the license at https://huggingface.co/{MODEL_REPO} \
                     and set HF_TOKEN in your .env file."
                ))
            } else {
                AppError::Ai(msg)
            }
        })?;
        let tokenizer_path = repo.get("tokenizer.json").await.map_err(|e| AppError::Ai(e.to_string()))?;
        let model_path = repo.get("model.safetensors").await.map_err(|e| AppError::Ai(e.to_string()))?;

        let config_str = std::fs::read_to_string(&config_path).map_err(|e| AppError::Ai(e.to_string()))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| AppError::Ai(format!("Config error: {}", e)))?;

        let device = Device::new_metal(0).unwrap_or(Device::Cpu);
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &device)
                .map_err(|e| AppError::Ai(e.to_string()))?
        };

        let model = Model::new(false, &config, vb).map_err(|e| AppError::Ai(e.to_string()))?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| AppError::Ai(e.to_string()))?;
        let eos_token = tokenizer.token_to_id("<eos>").unwrap_or(1);

        self.inner = Some(Arc::new(Mutex::new(GemmaInner { model, tokenizer, device, eos_token })));
        Ok(())
    }
}

pub async fn get_engine() -> &'static Mutex<GemmaEngine> {
    ENGINE.get_or_init(|| Mutex::new(GemmaEngine::new()))
}

pub fn is_loaded() -> bool {
    ENGINE.get()
        .and_then(|e| e.try_lock().ok())
        .map(|g| g.inner.is_some())
        .unwrap_or(false)
}

pub async fn load(app: &AppHandle) -> Result<(), AppError> {
    let mut engine = get_engine().await.lock().await;
    engine.load(app).await
}

pub async fn generate<F>(prompt: &str, on_token: F) -> Result<(), AppError>
where F: Fn(String) + Send + Sync + 'static {
    let (inner, mut cancel_rx) = {
        let mut engine = get_engine().await.lock().await;
        let inner = engine.inner.clone().ok_or_else(|| AppError::Ai("Gemma not loaded".into()))?;
        let (tx, rx) = mpsc::channel(1);
        engine.cancel_tx = Some(tx);
        (inner, rx)
    };

    let mut model_data = inner.lock().await;
    let eos_token = model_data.eos_token;

    let tokens = model_data.tokenizer.encode(prompt, true).map_err(|e| AppError::Ai(e.to_string()))?;
    let mut tokens = tokens.get_ids().to_vec();
    let mut next_token = *tokens.last().unwrap();

    for _ in 0..512usize {
        if cancel_rx.try_recv().is_ok() {
            break;
        }

        let input = Tensor::new(&[next_token], &model_data.device)
            .map_err(|e| AppError::Ai(e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| AppError::Ai(e.to_string()))?;

        let logits = model_data.model.forward(&input, tokens.len() - 1)
            .map_err(|e| AppError::Ai(e.to_string()))?
            .squeeze(0)
            .map_err(|e| AppError::Ai(e.to_string()))?;

        next_token = logits.argmax(0)
            .map_err(|e| AppError::Ai(e.to_string()))?
            .to_scalar::<u32>()
            .map_err(|e| AppError::Ai(e.to_string()))?;

        tokens.push(next_token);

        if let Some(text) = model_data.tokenizer.id_to_token(next_token) {
            // SentencePiece ▁ = space; skip special tokens (wrapped in <>)
            let text = text.replace('▁', " ").replace("<0x0A>", "\n");
            if !text.starts_with('<') || text == "<0x0A>" {
                on_token(text);
            }
        }

        if next_token == eos_token {
            break;
        }
    }

    get_engine().await.lock().await.cancel_tx = None;
    Ok(())
}

pub async fn cancel() -> Result<(), AppError> {
    let mut engine = get_engine().await.lock().await;
    if let Some(tx) = engine.cancel_tx.take() {
        let _ = tx.send(()).await;
    }
    Ok(())
}
