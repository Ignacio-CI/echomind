mod audio;
mod ai;
mod storage;
mod commands;
mod error;
pub mod events;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Try project-root .env first, then fall back to CWD search
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join("../.env"))
        .or_else(|_| dotenvy::dotenv().map(|_| ()));
    
    tauri::Builder::default()
        .setup(|app| {
            storage::init(app)?;
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::models_prepare,
            commands::models_status,
            commands::audio_list_devices,
            commands::audio_start,
            commands::audio_stop,
            commands::ai_generate,
            commands::ai_cancel,
            commands::meeting_summarize,
            commands::meetings_list,
            commands::meeting_create,
            commands::meeting_get_transcript,
            commands::meeting_get_summary,
            commands::settings_set,
            commands::settings_get,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
