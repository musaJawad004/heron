//! Native notifications for hook events (tauri-plugin-notification).
//!
//! Contract (implemented by the hooks agent):
//! - Title "<project> needs you" / "<project> finished"; body from the event
//!   message or a generic sentence ("Claude is waiting for permission",
//!   "Permission: Bash", "Claude is waiting for your input", "Claude finished
//!   its turn"). Never include prompt or tool input text.
//! - Sound only when `settings.play_sound`.
//! - Clicking a notification should focus the session's terminal
//!   (`launch::focus`) when the platform supports click actions.

use crate::model::{HookEvent, Session};
use crate::settings::Settings;
use tauri::AppHandle;

pub fn post(_app: &AppHandle, _event: &HookEvent, _session: Option<&Session>, _settings: &Settings) {
    // STUB
}

/// A sample notification for the Settings → Notifications "Send test" button.
pub fn post_test(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title("Heron")
        .body("Notifications are working. This never leaves your machine.")
        .show()
        .map_err(|e| e.to_string())
}
