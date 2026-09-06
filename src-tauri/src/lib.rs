//! Heron: a menu bar / tray app that watches Claude Code CLI sessions.
//! Local only — there is no network code anywhere in this crate.

pub mod claude;
pub mod commands;
pub mod hooks;
pub mod launch;
pub mod model;
pub mod settings;
pub mod state;
pub mod tray;
pub mod windows;

use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just reveals the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_popover(app);
        }))
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            app.manage(state::AppState::new(&handle));
            tray::create(&handle)?;
            state::start(&handle);

            let show_panel = handle.state::<state::AppState>().settings().show_panel;
            windows::set_panel_visible(&handle, show_panel);
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) if window.label() == "popover" => {
                let _ = window.hide();
            }
            WindowEvent::CloseRequested { api, .. } if window.label() != "panel" => {
                // Keep the app alive; windows are reopened from the tray.
                let _ = window.hide();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_history,
            commands::clear_error,
            commands::new_session,
            commands::resume_session,
            commands::focus_session,
            commands::terminate_session,
            commands::reveal_session,
            commands::get_settings,
            commands::update_settings,
            commands::installed_terminals,
            commands::detect_claude,
            commands::pick_folder,
            commands::set_launch_at_login,
            commands::install_hooks,
            commands::uninstall_hooks,
            commands::test_notification,
            commands::reveal_claude_settings,
            commands::reveal_app_data,
            commands::hide_popover,
            commands::toggle_panel,
            commands::open_settings,
            commands::panel_resized,
            commands::panel_moved,
            commands::open_url,
            commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Heron")
        .run(|_app, event| {
            // With every window hidden Tauri would exit; stay resident in the tray.
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
