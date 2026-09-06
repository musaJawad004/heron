//! Heron: a menu bar / tray app that watches Claude Code CLI sessions.
//! Local only — there is no network code anywhere in this crate.

pub mod claude;
pub mod commands;
pub mod hooks;
pub mod launch;
pub mod model;
pub mod placement;
pub mod settings;
pub mod state;
pub mod tray;
pub mod window_commands;
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

            windows::configure_native(&handle);
            let show_panel = handle.state::<state::AppState>().settings().show_panel;
            windows::set_panel_visible(&handle, show_panel);
            windows::start_panel_thread(&handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match (window.label(), event) {
                (windows::POPOVER, WindowEvent::Focused(false)) => windows::popover_blurred(app),
                (windows::PANEL, WindowEvent::Moved(_)) => windows::panel_window_moved(),
                // Keep the app alive; windows are reopened from the tray.
                (windows::SETTINGS, WindowEvent::CloseRequested { api, .. }) => {
                    api.prevent_close();
                    windows::settings_close_requested(app);
                }
                (windows::POPOVER, WindowEvent::CloseRequested { api, .. }) => {
                    api.prevent_close();
                    windows::hide_popover(app);
                }
                _ => {}
            }
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
            window_commands::popover_resized,
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
