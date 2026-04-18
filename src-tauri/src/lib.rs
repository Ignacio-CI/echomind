mod audio;
mod ai;
mod storage;
mod commands;
mod error;
pub mod events;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::audio_list_devices,
            commands::audio_start,
            commands::audio_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
