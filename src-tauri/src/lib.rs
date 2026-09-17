use gk2_save_core::{Request, Response, Service, Summary};
use std::sync::Mutex;

type SaveState = Mutex<Service>;

#[tauri::command]
fn save_open(
    request: tauri::ipc::Request<'_>,
    state: tauri::State<'_, SaveState>,
) -> Result<Summary, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("Expected binary save bytes".into());
    };
    state
        .lock()
        .map_err(|e| e.to_string())?
        .open(bytes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_request(request: Request, state: tauri::State<'_, SaveState>) -> Result<Response, String> {
    state
        .lock()
        .map_err(|e| e.to_string())?
        .request(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_export(state: tauri::State<'_, SaveState>) -> Result<tauri::ipc::Response, String> {
    let bytes = state
        .lock()
        .map_err(|e| e.to_string())?
        .export()
        .map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SaveState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            save_open,
            save_request,
            save_export
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
