//! Native notifications for hook events (tauri-plugin-notification).
//!
//! - Title "<project> needs you" / "<project> finished". The project is the
//!   session's folder name, else the last component of the event's `cwd`,
//!   else "Claude".
//! - Body: `PermissionRequest` → "Permission: <tool>" (or "Claude is waiting
//!   for permission"); `permission_prompt` / `idle_prompt` → the CLI's own
//!   short system message when present, else the generic sentence; every
//!   other attention event → "Claude is waiting for your input"; completion
//!   → "Claude finished its turn". No other payload text is ever shown.
//! - Sound only when `settings.play_sound`, using the platform's default
//!   notification sound.
//! - `PermissionRequest` and `Notification(permission_prompt)` fire for the
//!   same prompt, so an identical (session, title) within 2 s is posted once.
//! - Failures (no permission, no notification server) are logged and
//!   swallowed; the app must outlive them.
//! - Click handling: the desktop backend offers no click callback on
//!   macOS/Linux, so clicking only brings the app forward.

use crate::model::{now_ms, HookEvent, HookEventKind, Session};
use crate::settings::Settings;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// The system's default notification sound, in the name each backend expects
/// (`mac-notification-sys`, `winrt-notification`, freedesktop sound theme).
const DEFAULT_SOUND: &str = if cfg!(target_os = "macos") {
    "NSUserNotificationDefaultSoundName"
} else if cfg!(windows) {
    "Default"
} else {
    "message-new-instant"
};

const DEDUPE_WINDOW_MS: u64 = 2_000;

/// (session id, title, posted at ms) of recent notifications.
static RECENT: Mutex<Vec<(String, String, u64)>> = Mutex::new(Vec::new());

struct Text {
    title: String,
    body: String,
}

pub fn post(app: &AppHandle, event: &HookEvent, session: Option<&Session>, settings: &Settings) {
    let Some(text) = compose(event, session) else { return };
    if is_duplicate(&event.session_id, &text.title, now_ms()) {
        log::debug!("suppressing duplicate notification for session {}", event.session_id);
        return;
    }
    // Prefer the clickable path; it is only available from a real bundle.
    #[cfg(target_os = "macos")]
    if post_clickable(app, event.session_id.clone(), &text, settings.play_sound) {
        return;
    }
    let mut builder = app.notification().builder().title(&text.title).body(&text.body);
    if settings.play_sound {
        builder = builder.sound(DEFAULT_SOUND);
    }
    if let Err(e) = builder.show() {
        log::warn!("could not show notification: {e}");
    }
}

/// Deliver the banner ourselves so a click can be answered by focusing the
/// session's terminal. `tauri-plugin-notification` throws the response away,
/// and in a dev build it also attributes banners to Terminal.app, so this path
/// is what gives them Heron's own icon and name.
///
/// Returns false when the notification could not be delivered this way (an
/// unbundled binary, or the notification centre refusing), so the caller can
/// fall back to the plugin.
#[cfg(target_os = "macos")]
fn post_clickable(app: &AppHandle, session_id: String, text: &Text, play_sound: bool) -> bool {
    use mac_notification_sys::{MainButton, Notification, NotificationResponse};
    use std::sync::OnceLock;

    // `set_application` needs a real bundle; remember whether it took.
    static READY: OnceLock<bool> = OnceLock::new();
    let ready = *READY.get_or_init(|| {
        let id = app.config().identifier.clone();
        match mac_notification_sys::set_application(&id) {
            Ok(()) => true,
            Err(e) => {
                log::info!("notifications fall back to the plugin ({id} is not a bundle here): {e}");
                false
            }
        }
    });
    if !ready {
        return false;
    }

    let (title, body) = (text.title.clone(), text.body.clone());
    let handle = app.clone();
    // `send` blocks until the banner is dismissed or clicked, so it gets a
    // thread of its own. A few concurrent banners is the realistic ceiling.
    let spawned = std::thread::Builder::new().name("heron-notify".into()).spawn(move || {
        let mut n = Notification::new();
        n.title(&title).message(&body).main_button(MainButton::SingleAction("Focus"));
        if play_sound {
            n.sound(DEFAULT_SOUND);
        }
        match n.send() {
            Ok(NotificationResponse::Click) | Ok(NotificationResponse::ActionButton(_)) => {
                let state = handle.state::<crate::state::AppState>();
                state.focus(&handle, &session_id);
            }
            Ok(_) => {}
            Err(e) => log::warn!("could not show notification: {e}"),
        }
    });
    spawned.is_ok()
}

/// A sample notification for the Settings → Notifications "Send test" button.
pub fn post_test(app: &AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("Heron")
        .body("Notifications are working. This never leaves your machine.")
        .show()
        .map_err(|e| e.to_string())
}

/// Title and body for an event, or `None` when it is not worth a notification.
fn compose(event: &HookEvent, session: Option<&Session>) -> Option<Text> {
    let project = session
        .map(Session::project_name)
        .or_else(|| event.cwd.as_deref().and_then(last_component))
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "Claude".to_string());
    let subtype = event.subtype.as_deref();
    if event.requires_attention() {
        let body = match (event.kind, subtype) {
            (HookEventKind::PermissionRequest, _) => match event.tool_name.as_deref() {
                Some(tool) => format!("Permission: {tool}"),
                None => "Claude is waiting for permission".to_string(),
            },
            // The CLI's own one-line system messages; never user or tool text.
            (_, Some("permission_prompt")) => {
                event.message.clone().unwrap_or_else(|| "Claude is waiting for permission".to_string())
            }
            _ => "Claude is waiting for your input".to_string(),
        };
        return Some(Text { title: format!("{project} needs you"), body });
    }
    if event.is_idle_prompt() {
        return Some(Text {
            title: format!("{project} is waiting"),
            // The CLI's own one-line system message; never user or tool text.
            body: event.message.clone().unwrap_or_else(|| "Claude is waiting for your input".to_string()),
        });
    }
    if event.is_completion() {
        return Some(Text {
            title: format!("{project} finished"),
            body: "Claude finished its turn".to_string(),
        });
    }
    None
}

fn last_component(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).map(String::from)
}

/// Record (session, title) at `now`; true when the same pair was recorded
/// less than `DEDUPE_WINDOW_MS` ago.
fn is_duplicate(session_id: &str, title: &str, now: u64) -> bool {
    let mut recent = RECENT.lock();
    recent.retain(|(_, _, at)| now.saturating_sub(*at) < DEDUPE_WINDOW_MS);
    if recent.iter().any(|(s, t, _)| s == session_id && t == title) {
        return true;
    }
    recent.push((session_id.to_string(), title.to_string(), now));
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: HookEventKind, subtype: Option<&str>) -> HookEvent {
        HookEvent {
            id: "1-0".into(),
            kind,
            session_id: "sess".into(),
            cwd: Some("/Users/me/my-app/".into()),
            transcript_path: None,
            subtype: subtype.map(String::from),
            message: None,
            tool_name: None,
            received_at: 1,
        }
    }

    fn text(ev: &HookEvent, session: Option<&Session>) -> (String, String) {
        let t = compose(ev, session).expect("notifiable");
        (t.title, t.body)
    }

    #[test]
    fn permission_request_names_the_tool() {
        let mut ev = event(HookEventKind::PermissionRequest, None);
        assert_eq!(text(&ev, None), ("my-app needs you".into(), "Claude is waiting for permission".into()));
        ev.tool_name = Some("Bash".into());
        assert_eq!(text(&ev, None).1, "Permission: Bash");
    }

    #[test]
    fn notification_subtypes() {
        let mut ev = event(HookEventKind::Notification, Some("permission_prompt"));
        assert_eq!(text(&ev, None).1, "Claude is waiting for permission");
        ev.message = Some("Claude needs your permission to use Bash".into());
        assert_eq!(text(&ev, None).1, "Claude needs your permission to use Bash");

        let mut idle = event(HookEventKind::Notification, Some("idle_prompt"));
        assert_eq!(text(&idle, None).1, "Claude is waiting for your input");
        idle.message = Some("Claude is waiting for your input".into());
        assert_eq!(text(&idle, None).1, "Claude is waiting for your input");

        for sub in ["elicitation_dialog", "elicitation_url_dialog", "agent_needs_input"] {
            let mut ev = event(HookEventKind::Notification, Some(sub));
            ev.message = Some("What is the database password?".into());
            assert_eq!(
                text(&ev, None).1,
                "Claude is waiting for your input",
                "{sub} never echoes the message"
            );
        }

        let done = event(HookEventKind::Notification, Some("agent_completed"));
        assert_eq!(text(&done, None), ("my-app finished".into(), "Claude finished its turn".into()));
    }

    #[test]
    fn stop_is_a_completion() {
        let ev = event(HookEventKind::Stop, None);
        assert_eq!(text(&ev, None), ("my-app finished".into(), "Claude finished its turn".into()));
    }

    #[test]
    fn project_name_sources() {
        let mut ev = event(HookEventKind::Stop, None);
        let session = Session { cwd: "C:\\Users\\me\\other".into(), ..Default::default() };
        assert_eq!(text(&ev, Some(&session)).0, "other finished", "session wins over cwd");
        ev.cwd = None;
        assert_eq!(text(&ev, None).0, "Claude finished");
        ev.cwd = Some("/".into());
        assert_eq!(text(&ev, None).0, "Claude finished");
    }

    #[test]
    fn silent_events_are_not_notified() {
        for (kind, sub) in [
            (HookEventKind::SessionStart, Some("startup")),
            (HookEventKind::SessionEnd, Some("user_exit")),
            (HookEventKind::UserPromptSubmit, None),
            (HookEventKind::Notification, Some("auth_success")),
            (HookEventKind::Notification, None),
            (HookEventKind::PreToolUse, None),
            (HookEventKind::Other, None),
        ] {
            assert!(compose(&event(kind, sub), None).is_none(), "{kind:?}/{sub:?}");
        }
    }

    #[test]
    fn duplicates_within_window_are_suppressed() {
        assert!(!is_duplicate("dedupe-a", "x needs you", 10_000));
        assert!(is_duplicate("dedupe-a", "x needs you", 10_500));
        assert!(!is_duplicate("dedupe-a", "x finished", 10_600), "different title");
        assert!(!is_duplicate("dedupe-b", "x needs you", 10_700), "different session");
        assert!(!is_duplicate("dedupe-a", "x needs you", 13_000), "window elapsed");
    }
}
