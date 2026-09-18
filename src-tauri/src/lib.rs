use gk2_save_core::{Command, DocumentSummary};
use gk2_save_desktop::{DesktopWorkspace, Opened, Settings};
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
struct State {
    desktop: DesktopWorkspace,
    settings: Settings,
    config: PathBuf,
}
type SaveState = Mutex<State>;
#[tauri::command]
fn save_open(
    request: tauri::ipc::Request<'_>,
    state: tauri::State<'_, SaveState>,
) -> Result<DocumentSummary, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("Expected binary save bytes".into());
    };
    state
        .lock()
        .map_err(|e| e.to_string())?
        .desktop
        .workspace
        .open(bytes)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_request(
    document: u32,
    request: Command,
    state: tauri::State<'_, SaveState>,
) -> Result<serde_json::Value, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let close = matches!(request, Command::Close);
    let result = s
        .desktop
        .workspace
        .request(document, request)
        .map_err(|e| e.to_string())?;
    if close {
        s.desktop.close(document)
    }
    Ok(result)
}
#[tauri::command]
fn save_export(
    document: u32,
    state: tauri::State<'_, SaveState>,
) -> Result<tauri::ipc::Response, String> {
    Ok(tauri::ipc::Response::new(
        state
            .lock()
            .map_err(|e| e.to_string())?
            .desktop
            .workspace
            .export(document)
            .map_err(|e| e.to_string())?,
    ))
}
fn directories(s: &State) -> Vec<PathBuf> {
    gk2_save_desktop::discover(
        std::env::consts::OS,
        &std::env::vars().collect(),
        &s.settings.custom_directories,
    )
}
#[tauri::command]
fn save_discover(
    include_backups: bool,
    state: tauri::State<'_, SaveState>,
) -> Result<serde_json::Value, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let dirs = directories(&s);
    Ok(
        serde_json::json!({"directories":dirs,"saves":gk2_save_desktop::list(&dirs,include_backups)}),
    )
}
#[tauri::command]
fn settings_get(state: tauri::State<'_, SaveState>) -> Result<Settings, String> {
    Ok(state.lock().map_err(|e| e.to_string())?.settings.clone())
}
#[tauri::command]
fn settings_set(settings: Settings, state: tauri::State<'_, SaveState>) -> Result<(), String> {
    settings.validate()?;
    let mut s = state.lock().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(s.config.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(
        &s.config,
        serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    s.settings = settings;
    Ok(())
}
#[tauri::command]
async fn save_open_path(
    path: PathBuf,
    state: tauri::State<'_, SaveState>,
) -> Result<Opened, String> {
    state
        .lock()
        .map_err(|e| e.to_string())?
        .desktop
        .open_path(&path)
}
#[tauri::command]
async fn save_dialog(
    app: tauri::AppHandle,
    kind: String,
    document: Option<u32>,
) -> Result<Option<PathBuf>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<SaveState>();
        let s = state.lock().map_err(|e| e.to_string())?;
        let dir = s
            .desktop
            .recent_directory
            .clone()
            .filter(|p| p.is_dir())
            .or_else(|| directories(&s).into_iter().next());
        let name = document
            .and_then(|id| s.desktop.path(id))
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
        drop(s);
        let mut dialog = app.dialog().file();
        if let Some(dir) = dir {
            dialog = dialog.set_directory(dir)
        }
        if let Some(name) = name {
            dialog = dialog.set_file_name(name)
        }
        let selected = match kind.as_str() {
            "folder" => dialog.blocking_pick_folder(),
            "save" => dialog
                .add_filter("Save file", &["dat"])
                .blocking_save_file(),
            _ => dialog
                .add_filter("Save file", &["dat"])
                .blocking_pick_file(),
        };
        selected
            .map(|p| p.into_path().map_err(|e| e.to_string()))
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn save_write(
    document: u32,
    revision: u32,
    destination: Option<PathBuf>,
    state: tauri::State<'_, SaveState>,
) -> Result<Opened, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let retention = s.settings.backup_retention;
    s.desktop.save(document, revision, destination, retention)
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = app.path().app_config_dir()?.join("settings.json");
            let settings = std::fs::read(&config)
                .ok()
                .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
                .filter(|s| s.validate().is_ok())
                .unwrap_or_default();
            app.manage(Mutex::new(State {
                desktop: DesktopWorkspace::default(),
                settings,
                config,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_open,
            save_request,
            save_export,
            save_discover,
            save_open_path,
            save_dialog,
            save_write,
            settings_get,
            settings_set
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
