//! Window-behaviour commands owned by the tray/windows module, kept out of
//! `commands.rs` so the two IPC surfaces evolve independently. Registered in
//! `lib.rs`; thin wrappers over `windows`.

use crate::windows;
use tauri::AppHandle;

/// The popover page reports its content height (logical px) after measuring
/// itself; the window is resized between 120 and 560 px and stays attached
/// to the tray icon. Frontend: `invoke('popover_resized', { height })`.
#[tauri::command]
pub fn popover_resized(app: AppHandle, height: f64) {
    windows::popover_resized(&app, height);
}
