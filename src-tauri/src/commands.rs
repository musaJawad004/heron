//! Tauri commands: the whole IPC surface the Svelte side can call.
//! Keep these thin; logic lives in `state`, `launch`, `hooks`.

use crate::launch;
use crate::model::{Snapshot, TerminalChoice};
use crate::settings::Settings;
use crate::state::{self, AppState};
use crate::{hooks, windows};
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snapshot()
}

#[tauri::command]
pub fn refresh_history(app: AppHandle) {
    state::refresh_history(&app);
}

#[tauri::command]
pub fn clear_error(app: AppHandle, state: State<'_, AppState>) {
    state.clear_error(&app);
}

#[tauri::command]
pub fn new_session(app: AppHandle, state: State<'_, AppState>, folder: Option<String>) {
    match folder {
        Some(f) if !f.is_empty() => state.new_session(&app, &PathBuf::from(f)),
        _ => windows::new_session_with_picker(&app),
    }
}

#[tauri::command]
pub fn resume_session(app: AppHandle, state: State<'_, AppState>, id: String) {
    state.resume(&app, &id);
}

#[tauri::command]
pub fn focus_session(app: AppHandle, state: State<'_, AppState>, id: String) {
    windows::hide_popover(&app);
    state.focus(&app, &id);
}

#[tauri::command]
pub fn terminate_session(app: AppHandle, state: State<'_, AppState>, id: String) {
    state.terminate(&app, &id);
}

#[tauri::command]
pub fn reveal_session(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let session = state.session(&id).ok_or("unknown session")?;
    app.opener().reveal_item_in_dir(PathBuf::from(&session.cwd)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<Settings, String> {
    state.update_settings(&app, patch)
}

#[tauri::command]
pub fn installed_terminals() -> Vec<TerminalChoice> {
    launch::installed_terminals()
}

#[tauri::command]
pub fn detect_claude() -> Option<String> {
    launch::find_claude().map(|p| p.display().to_string())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |p| {
        let _ = tx.send(p);
    });
    tauri::async_runtime::spawn_blocking(move || rx.recv().ok().flatten())
        .await
        .ok()
        .flatten()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let r = if enabled { launcher.enable() } else { launcher.disable() };
    r.map_err(|e| e.to_string())?;
    state.update_settings(&app, serde_json::json!({ "launchAtLogin": enabled })).map(|_| ())
}

#[tauri::command]
pub fn install_hooks(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.install_hooks(&app)
}

#[tauri::command]
pub fn uninstall_hooks(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.uninstall_hooks(&app)
}

#[tauri::command]
pub fn test_notification(app: AppHandle) -> Result<(), String> {
    hooks::notify::post_test(&app)
}

#[tauri::command]
pub fn reveal_claude_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(state.claude_paths.settings_file()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_app_data(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(state.app_paths.data_dir.clone()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    windows::hide_popover(&app);
}

#[tauri::command]
pub fn toggle_panel(app: AppHandle) {
    windows::toggle_panel(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
}

#[tauri::command]
pub fn panel_resized(app: AppHandle, width: f64, height: f64) {
    windows::panel_resized(&app, width, height);
}

#[tauri::command]
pub fn panel_moved(app: AppHandle) {
    windows::panel_moved(&app);
}

/// Opens an https:// link in the default browser (About page). Heron itself
/// never talks to the network; this hands the URL to the OS.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !url.starts_with("https://") {
        return Err("only https links are opened".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
