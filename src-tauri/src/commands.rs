use crate::ai::whisper;
use crate::audio;
use crate::error::AppError;
use crate::events::{TRANSCRIPT_FINAL, TRANSCRIPT_PARTIAL};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub fn ping() -> Result<String, AppError> {
    Ok("pong".into())
}

#[tauri::command]
pub fn audio_list_devices() -> Result<Vec<audio::AudioDevice>, AppError> {
    audio::list_devices()
}

#[tauri::command]
pub async fn audio_start(app: AppHandle) -> Result<(), AppError> {
    let chunk_rx = audio::start(app.clone())?;

    // Load Whisper model (cached after first load)
    whisper::load().await?;

    // Spawn a task to consume audio chunks and emit transcript events
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        for chunk in chunk_rx {
            let _ = app_clone.emit(TRANSCRIPT_PARTIAL, "…");
            let app2 = app_clone.clone();
            rt.spawn(async move {
                match whisper::transcribe(chunk).await {
                    Ok(text) if !text.is_empty() => {
                        let _ = app2.emit(TRANSCRIPT_FINAL, text);
                    }
                    _ => {}
                }
            });
        }
    });

    Ok(())
}

#[tauri::command]
pub fn audio_stop() -> Result<(), AppError> {
    audio::stop()
}
