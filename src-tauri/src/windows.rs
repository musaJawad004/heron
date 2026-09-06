//! The three windows: `popover` (under the tray icon), `panel` (the floating
//! vigil at the screen edge) and `settings`.
//!
//! Contract (implemented by the tray/windows agent):
//! - Popover: positioned relative to the tray icon (positioner plugin),
//!   hidden on blur (see lib.rs), never shown in the Dock/taskbar.
//! - Panel: non-activating, always on top, on all Spaces; placed at the saved
//!   origin if still on some monitor, else pinned to `panel_edge` of the
//!   monitor under the cursor (or the primary), 8 px in, top at work-area
//!   top + 8 px. Follows the active monitor when enabled. Snaps to an edge
//!   after a drag and persists the origin.
//! - Settings: a normal window; closing hides it so it reopens instantly.

use crate::settings::PanelEdge;
use crate::state::AppState;
use tauri::{AppHandle, Manager, PhysicalPosition, Position};
use tauri_plugin_positioner::WindowExt;

pub fn toggle_popover(app: &AppHandle) {
    let Some(w) = app.get_webview_window("popover") else { return };
    if w.is_visible().unwrap_or(false) {
        hide_popover(app);
    } else {
        show_popover(app);
    }
}

pub fn show_popover(app: &AppHandle) {
    let Some(w) = app.get_webview_window("popover") else { return };
    #[cfg(target_os = "macos")]
    let _ = w.move_window(tauri_plugin_positioner::Position::TrayCenter);
    #[cfg(not(target_os = "macos"))]
    let _ = w.move_window(tauri_plugin_positioner::Position::TrayBottomCenter);
    let _ = w.show();
    let _ = w.set_focus();
}

pub fn hide_popover(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("popover") {
        let _ = w.hide();
    }
}

pub fn open_settings(app: &AppHandle) {
    hide_popover(app);
    if let Some(w) = app.get_webview_window("settings") {
        #[cfg(target_os = "macos")]
        let _ = app.show();
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn toggle_panel(app: &AppHandle) {
    let state = app.state::<AppState>();
    let visible = !state.settings().show_panel;
    let _ = state.update_settings(app, serde_json::json!({ "showPanel": visible }));
}

pub fn set_panel_visible(app: &AppHandle, visible: bool) {
    let Some(w) = app.get_webview_window("panel") else { return };
    if visible {
        place_panel(app);
        let _ = w.show();
    } else {
        let _ = w.hide();
    }
}

/// Put the panel at its saved origin or pinned to the configured edge.
pub fn place_panel(app: &AppHandle) {
    // STUB — the tray/windows agent replaces this with monitor-aware placement.
    let Some(w) = app.get_webview_window("panel") else { return };
    let settings = app.state::<AppState>().settings();
    if let Some(o) = &settings.panel_origin {
        let _ = w.set_position(Position::Physical(PhysicalPosition::new(o.x, o.y)));
        return;
    }
    let Ok(Some(monitor)) = app.primary_monitor() else { return };
    let scale = monitor.scale_factor();
    let size = w.outer_size().unwrap_or(tauri::PhysicalSize::new(200, 44));
    let margin = (8.0 * scale) as i32;
    let top = monitor.position().y + (32.0 * scale) as i32;
    let x = match settings.panel_edge {
        PanelEdge::Left => monitor.position().x + margin,
        PanelEdge::Right => monitor.position().x + monitor.size().width as i32 - size.width as i32 - margin,
    };
    let _ = w.set_position(Position::Physical(PhysicalPosition::new(x, top)));
}

/// Called by the panel page after a drag ends: persist and snap.
pub fn panel_moved(app: &AppHandle) {
    // STUB — snap to the nearest edge when within 24 px, then persist.
    let Some(w) = app.get_webview_window("panel") else { return };
    if let Ok(p) = w.outer_position() {
        let state = app.state::<AppState>();
        let _ = state.update_settings(
            app,
            serde_json::json!({ "panelOrigin": { "x": p.x, "y": p.y, "monitor": null } }),
        );
    }
}

/// Resize the panel window to the content size reported by the page.
pub fn panel_resized(app: &AppHandle, width: f64, height: f64) {
    let Some(w) = app.get_webview_window("panel") else { return };
    let _ = w.set_size(tauri::Size::Logical(tauri::LogicalSize::new(width.max(120.0), height.max(32.0))));
}

/// Open the folder picker, then start a session there.
pub fn new_session_with_picker(app: &AppHandle) {
    use tauri_plugin_dialog::DialogExt;
    hide_popover(app);
    let state = app.state::<AppState>();
    let base = state.settings().default_projects_folder;
    let mut dialog = app.dialog().file().set_title("Start Claude Code in…");
    if !base.is_empty() {
        dialog = dialog.set_directory(base);
    }
    let handle = app.clone();
    dialog.pick_folder(move |picked| {
        if let Some(path) = picked.and_then(|p| p.into_path().ok()) {
            let state = handle.state::<AppState>();
            state.new_session(&handle, &path);
        }
    });
}
