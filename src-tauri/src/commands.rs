use crate::ai::{gemma, meeting, prompts, whisper};
use crate::audio;
use crate::storage::{self, Meeting};
use crate::error::AppError;
use crate::events::{AI_STATUS, AI_TOKEN, SUMMARY_UPDATE, TRANSCRIPT_FINAL, TRANSCRIPT_PARTIAL};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub fn ping() -> Result<String, AppError> {
    Ok("pong".into())
}

// ── Model management ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn models_prepare(app: AppHandle) -> Result<(), AppError> {
    whisper::load(&app).await?;
    gemma::load(&app).await?;
    let _ = app.emit(AI_STATUS, "AI Ready");
    Ok(())
}

#[tauri::command]
pub fn models_status() -> bool {
    whisper::is_loaded() && gemma::is_loaded()
}

// ── Audio ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn audio_list_devices() -> Result<Vec<audio::AudioDevice>, AppError> {
    audio::list_devices()
}

#[tauri::command]
pub async fn audio_start(app: AppHandle) -> Result<(), AppError> {
    if !whisper::is_loaded() || !gemma::is_loaded() {
        return Err(AppError::Ai("Models not loaded. Call models_prepare first.".into()));
    }

    let chunk_rx = audio::start(app.clone())?;

    let _ = app.emit(SUMMARY_UPDATE, "EchoMind is active and listening. Summaries will appear here.");

    // Transcription task
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        for chunk in chunk_rx {
            let _ = app_clone.emit(TRANSCRIPT_PARTIAL, "…");
            let app2 = app_clone.clone();
            rt.spawn(async move {
                match whisper::transcribe(chunk).await {
                    Ok(text) if !text.is_empty() => {
                        meeting::append_transcript(text.clone());
                        let _ = app2.emit(TRANSCRIPT_FINAL, text);
                    }
                    _ => {}
                }
            });
        }
    });

    // Auto-summarization task (every 30 s of silence or 2 min)
    let app_clone = app.clone();
    tokio::spawn(async move {
        let mut last_summary_time = std::time::Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            // Stop loop when recording ends
            if !audio::is_recording() {
                break;
            }

            let transcript = meeting::get_full_transcript();
            if transcript.is_empty() {
                continue;
            }

            let silence = std::time::Instant::now().duration_since(meeting::get_last_activity());
            let since_last = std::time::Instant::now().duration_since(last_summary_time);

            if silence > Duration::from_secs(30) || since_last > Duration::from_secs(120) {
                let prompt = prompts::get_summary_prompt(&transcript, "en");
                let full_summary = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
                let summary_clone = full_summary.clone();
                let app_inner = app_clone.clone();

                let res = gemma::generate(&prompt, move |token| {
                    let mut guard = summary_clone.lock().unwrap();
                    guard.push_str(&token);
                    let _ = app_inner.emit(SUMMARY_UPDATE, guard.clone());
                }).await;

                if res.is_ok() {
                    last_summary_time = std::time::Instant::now();
                    if let Some(mid) = meeting::get_current_meeting_id() {
                        let text = full_summary.lock().unwrap().clone();
                        if !text.is_empty() {
                            let _ = storage::summary_save(&mid, &text);
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn audio_stop() -> Result<(), AppError> {
    audio::stop()?;
    if let Some(mid) = meeting::get_current_meeting_id() {
        let _ = storage::meeting_end(&mid);
    }
    meeting::set_current_meeting_id(None);
    Ok(())
}

// ── Meetings ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn meetings_list() -> Result<Vec<Meeting>, AppError> {
    storage::meetings_list()
}

#[tauri::command]
pub fn meeting_create(id: String, title: String, locale: String) -> Result<(), AppError> {
    storage::meeting_create(&Meeting {
        id: id.clone(),
        title,
        started_at: chrono::Utc::now(),
        ended_at: None,
        locale,
    })?;
    meeting::set_current_meeting_id(Some(id));
    meeting::clear_transcript();
    Ok(())
}

#[tauri::command]
pub fn meeting_get_transcript(id: String) -> Result<Vec<storage::TranscriptEntry>, AppError> {
    storage::meeting_get_transcript(&id)
}

#[tauri::command]
pub fn meeting_get_summary(id: String) -> Result<Option<String>, AppError> {
    storage::meeting_get_latest_summary(&id)
}

#[tauri::command]
pub async fn meeting_summarize(app: AppHandle, locale: String) -> Result<(), AppError> {
    let transcript = meeting::get_full_transcript();
    if transcript.is_empty() {
        return Ok(());
    }

    let meeting_id = meeting::get_current_meeting_id();
    let prompt = prompts::get_summary_prompt(&transcript, &locale);
    let full_summary = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let summary_clone = full_summary.clone();
    let app_clone = app.clone();

    gemma::generate(&prompt, move |token| {
        let mut guard = summary_clone.lock().unwrap();
        guard.push_str(&token);
        let _ = app_clone.emit(SUMMARY_UPDATE, guard.clone());
    }).await?;

    if let Some(mid) = meeting_id {
        let text = full_summary.lock().unwrap().clone();
        if !text.is_empty() {
            storage::summary_save(&mid, &text)?;
        }
    }
    Ok(())
}

// ── Ad-hoc AI ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_generate(app: AppHandle, prompt: String, locale: String) -> Result<(), AppError> {
    if !gemma::is_loaded() {
        return Err(AppError::Ai("Model not loaded. Call models_prepare first.".into()));
    }
    let full_prompt = prompts::get_summary_prompt(&prompt, &locale);
    gemma::generate(&full_prompt, move |token| {
        let _ = app.emit(AI_TOKEN, token);
    }).await
}

#[tauri::command]
pub async fn ai_cancel() -> Result<(), AppError> {
    gemma::cancel().await
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn settings_set(key: String, value: String) -> Result<(), AppError> {
    storage::setting_set(&key, &value)
}

#[tauri::command]
pub fn settings_get(key: String) -> Result<Option<String>, AppError> {
    storage::setting_get(&key)
}
