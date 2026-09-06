//! Menu bar / system tray icon: glyph, live count (macOS title), tooltip and
//! the right-click menu. Left click toggles the popover.
//!
//! Contract (implemented by the tray/windows agent):
//! - macOS: template glyph, filled variant while any session is busy, and
//!   the running count as the tray title when `show_count_in_tray`.
//! - Windows/Linux: no title support, so render the count into the icon at
//!   runtime (small badge) and use a light glyph on dark taskbars.
//! - Attention (any session needs the user) → orange dot overlay / badge.

use crate::model::Snapshot;
use crate::{state, windows};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "main";

#[cfg(target_os = "macos")]
const GLYPH: &[u8] = include_bytes!("../icons/tray/tray-black.png");
#[cfg(not(target_os = "macos"))]
const GLYPH: &[u8] = include_bytes!("../icons/tray/tray-white.png");

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open Heron").build(app)?;
    let new_session =
        MenuItemBuilder::with_id("new", "New Session…").accelerator("CmdOrCtrl+N").build(app)?;
    let panel = MenuItemBuilder::with_id("panel", "Show / Hide Panel").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings…").accelerator("CmdOrCtrl+,").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Heron").accelerator("CmdOrCtrl+Q").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &open,
            &new_session,
            &panel,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ])
        .build()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(GLYPH)?)
        .icon_as_template(true)
        .tooltip("Heron")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => windows::show_popover(app),
            "new" => windows::new_session_with_picker(app),
            "panel" => windows::toggle_panel(app),
            "settings" => windows::open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_popover(tray.app_handle());
            }
        })
        .build(app)?;

    let snap = app.state::<state::AppState>().snapshot();
    update(app, &snap);
    Ok(())
}

/// Reflect the latest snapshot in the tray (count, tooltip, icon variant).
pub fn update(app: &AppHandle, snap: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let running = snap.running_count();
    let attention = snap.attention_count();
    let title =
        if snap.settings.show_count_in_tray && running > 0 { Some(running.to_string()) } else { None };
    let _ = tray.set_title(title.as_deref());
    let tooltip = match (running, attention) {
        (0, _) => "Heron — no Claude sessions".to_string(),
        (n, 0) => format!("Heron — {n} running"),
        (n, a) => format!("Heron — {n} running, {a} need you"),
    };
    let _ = tray.set_tooltip(Some(tooltip));
}
