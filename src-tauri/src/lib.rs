mod audio;
mod filesystem;
mod sqlite;
mod api;

use std::sync::{Arc, Mutex};
use tauri::State;

struct AudioState {
    boombox: Mutex<audio::Boombox>,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[tauri::command]
fn add_folders(folders: Vec<String>) -> Result<(), String> {
    sqlite::add_folders(folders).map_err(|e| e.to_string())?;
    Ok(())
}

// get folder list
#[tauri::command]
fn get_folders() -> Result<Vec<String>, String> {
    sqlite::get_folders().map_err(|e| e.to_string())?;
    Ok(sqlite::get_folders()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|s| s.to_string())
        .collect())
}

#[tauri::command]
fn get_files_in_folder(folder: String) -> Result<Vec<String>, String> {
    filesystem::get_files_in_path(folder).map_err(|e| e.to_string())
}

#[tauri::command]
fn play_audio(state: State<AudioState>, path: String) -> Result<(), String> {
    state.boombox.lock().unwrap().play_file(path)
}

#[tauri::command]
fn pause_audio(state: State<AudioState>) -> Result<(), String> {
    state.boombox.lock().unwrap().pause()
}

#[tauri::command]
fn resume_audio(state: State<AudioState>) -> Result<(), String> {
    state.boombox.lock().unwrap().resume()
}

#[tauri::command]
fn stop_audio(state: State<AudioState>) -> Result<(), String> {
    state.boombox.lock().unwrap().stop()
}

#[tauri::command]
fn set_volume(state: State<AudioState>, volume: f32) -> Result<(), String> {
    // volume should be between 0 and 1
    if volume < 0.0 || volume > 1.0 {
        return Err("Volume must be between 0 and 1".into());
    }
    state.boombox.lock().unwrap().set_volume(volume)
}

#[tauri::command]
fn list_audio_devices(state: State<AudioState>) -> Result<Vec<String>, String> {
    state.boombox.lock().unwrap().list_devices()
}

#[tauri::command]
fn set_audio_device(state: State<AudioState>, device_name: String) -> Result<(), String> {
    state.boombox.lock().unwrap().set_device(device_name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let boombox = audio::Boombox::new().expect("Failed to initialize audio system");
    let api_boombox = boombox.clone(); // Assuming you've implemented Clone for Boombox

    // Start API server in a separate thread
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        runtime.block_on(async {
            api::start_api_server(api_boombox).await.expect("Failed to start API server");
        });
    });

    tauri::Builder::default()
        .manage(AudioState {
            boombox: Mutex::new(boombox),
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            add_folders,
            get_folders,
            get_files_in_folder,
            play_audio,
            pause_audio,
            resume_audio,
            stop_audio,
            set_volume,
            list_audio_devices,
            set_audio_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
