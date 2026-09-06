//! Shared domain types. This file is the contract between every Rust module
//! and the Svelte frontend (`src/lib/types.ts` mirrors it field for field).
//! Keep it free of Tauri types so the core can be unit-tested without a GUI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unix time in milliseconds (Claude Code's own convention).
pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Live status of a running session, as written by the CLI to
/// `~/.claude/sessions/<pid>.json` (`status` field).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Claude is generating / running tools.
    Busy,
    /// Waiting for the user to type the next prompt.
    Idle,
    /// Blocked on the human (permission prompt, question, plan approval).
    NeedsInput,
    /// Reserved by the CLI; treated like `NeedsInput` for display.
    Waiting,
    /// Registry did not report a status we understand, or the session is historical.
    #[default]
    Unknown,
}

impl SessionStatus {
    pub fn from_raw(raw: Option<&str>) -> Self {
        match raw {
            Some("busy") => Self::Busy,
            Some("idle") => Self::Idle,
            Some("needs_input") => Self::NeedsInput,
            Some("waiting") => Self::Waiting,
            _ => Self::Unknown,
        }
    }

    /// True when the session is blocked on the human.
    pub fn needs_attention(self) -> bool {
        matches!(self, Self::NeedsInput | Self::Waiting)
    }
}

/// One Claude Code session: running (has a `pid`) or historical (resumable).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// Claude Code session id (UUID string). Also the transcript file stem.
    pub id: String,
    /// Display name from the live registry (`name`), e.g. "civl-mobile-app-1b".
    pub name: Option<String>,
    /// First user prompt, whitespace-collapsed, max 80 chars.
    pub title: Option<String>,
    /// Working directory the session was started in.
    pub cwd: String,
    /// Process id when running.
    pub pid: Option<u32>,
    pub status: SessionStatus,
    /// Registry `startedAt` (ms).
    pub started_at: Option<u64>,
    /// Registry `updatedAt`/`statusUpdatedAt` or transcript mtime (ms).
    pub last_active_at: u64,
    pub version: Option<String>,
    pub git_branch: Option<String>,
    pub transcript_path: Option<String>,
    /// Controlling terminal of the process (e.g. "ttys003"), Unix only.
    pub tty: Option<String>,
    /// Hook event that most recently asked for the user's attention.
    pub attention: Option<HookEvent>,
}

impl Session {
    pub fn is_running(&self) -> bool {
        self.pid.is_some()
    }

    /// Last path component of `cwd`.
    pub fn project_name(&self) -> String {
        let trimmed = self.cwd.trim_end_matches(['/', '\\']);
        trimmed.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).unwrap_or(&self.cwd).to_string()
    }

    /// Name, then title, then project folder.
    pub fn display_name(&self) -> String {
        if let Some(n) = self.name.as_deref().filter(|s| !s.is_empty()) {
            return n.to_string();
        }
        if let Some(t) = self.title.as_deref().filter(|s| !s.is_empty()) {
            return t.to_string();
        }
        self.project_name()
    }
}

/// Which Claude Code hook produced an event (`hook_event_name`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HookEventKind {
    SessionStart,
    SessionEnd,
    Stop,
    SubagentStop,
    Notification,
    PermissionRequest,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Other,
}

impl HookEventKind {
    pub fn from_name(name: &str) -> Self {
        match name {
            "SessionStart" => Self::SessionStart,
            "SessionEnd" => Self::SessionEnd,
            "Stop" => Self::Stop,
            "SubagentStop" => Self::SubagentStop,
            "Notification" => Self::Notification,
            "PermissionRequest" => Self::PermissionRequest,
            "UserPromptSubmit" => Self::UserPromptSubmit,
            "PreToolUse" => Self::PreToolUse,
            "PostToolUse" => Self::PostToolUse,
            _ => Self::Other,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::Stop => "Stop",
            Self::SubagentStop => "SubagentStop",
            Self::Notification => "Notification",
            Self::PermissionRequest => "PermissionRequest",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::Other => "Other",
        }
    }
}

/// A hook event received through the local spool directory. Only the fields
/// listed here are ever kept; `tool_input` and `user_input` are dropped at
/// parse time (see docs/HOOKS.md).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    pub id: String,
    pub kind: HookEventKind,
    pub session_id: String,
    pub cwd: Option<String>,
    pub transcript_path: Option<String>,
    /// `notification_type` | `reason` (SessionEnd) | `source` (SessionStart) | `result` (SubagentStop).
    pub subtype: Option<String>,
    /// `message` for notifications, truncated to 200 chars.
    pub message: Option<String>,
    pub tool_name: Option<String>,
    /// When Heron read the event (ms).
    pub received_at: u64,
}

impl HookEvent {
    /// Events that mean "a human needs to look at this session".
    pub fn requires_attention(&self) -> bool {
        match self.kind {
            HookEventKind::PermissionRequest => true,
            HookEventKind::Notification => matches!(
                self.subtype.as_deref(),
                Some("permission_prompt")
                    | Some("idle_prompt")
                    | Some("elicitation_dialog")
                    | Some("elicitation_url_dialog")
                    | Some("agent_needs_input")
            ),
            _ => false,
        }
    }

    /// Events that mean "Claude finished its turn".
    pub fn is_completion(&self) -> bool {
        self.kind == HookEventKind::Stop
            || (self.kind == HookEventKind::Notification
                && self.subtype.as_deref() == Some("agent_completed"))
    }
}

/// Terminal emulators Heron can open a session in. Availability is per OS;
/// see `launch::installed_terminals`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalApp {
    // macOS
    Terminal,
    Iterm,
    Ghostty,
    Warp,
    Kitty,
    Alacritty,
    Wezterm,
    // Windows
    WindowsTerminal,
    Powershell,
    Cmd,
    // Linux
    GnomeTerminal,
    Konsole,
    Xterm,
}

impl TerminalApp {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Terminal => "Terminal",
            Self::Iterm => "iTerm2",
            Self::Ghostty => "Ghostty",
            Self::Warp => "Warp",
            Self::Kitty => "kitty",
            Self::Alacritty => "Alacritty",
            Self::Wezterm => "WezTerm",
            Self::WindowsTerminal => "Windows Terminal",
            Self::Powershell => "PowerShell",
            Self::Cmd => "Command Prompt",
            Self::GnomeTerminal => "GNOME Terminal",
            Self::Konsole => "Konsole",
            Self::Xterm => "xterm",
        }
    }
}

/// Whether Heron's hooks are registered in `~/.claude/settings.json`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HookStatus {
    Installed,
    #[default]
    NotInstalled,
    /// Some events registered, or the script/command is stale.
    Partial,
}

/// A terminal choice offered to the user.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TerminalChoice {
    pub id: TerminalApp,
    pub name: &'static str,
}

/// Everything the UI needs, pushed on every change via the `snapshot` event.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub running: Vec<Session>,
    pub recent: Vec<Session>,
    pub attention: HashMap<String, HookEvent>,
    pub event_log: Vec<HookEvent>,
    pub hook_status: HookStatus,
    pub claude_path: Option<String>,
    pub last_error: Option<String>,
    pub is_refreshing_history: bool,
    pub settings: crate::settings::Settings,
}

impl Snapshot {
    pub fn running_count(&self) -> usize {
        self.running.len()
    }
    pub fn attention_count(&self) -> usize {
        self.running
            .iter()
            .filter(|s| s.status.needs_attention() || self.attention.contains_key(&s.id))
            .count()
    }
    pub fn busy_count(&self) -> usize {
        self.running.iter().filter(|s| s.status == SessionStatus::Busy).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parsing() {
        assert_eq!(SessionStatus::from_raw(Some("busy")), SessionStatus::Busy);
        assert_eq!(SessionStatus::from_raw(Some("needs_input")), SessionStatus::NeedsInput);
        assert_eq!(SessionStatus::from_raw(None), SessionStatus::Unknown);
        assert!(SessionStatus::NeedsInput.needs_attention());
    }

    #[test]
    fn project_name_handles_both_separators() {
        let s = Session { cwd: "/Users/me/proj".into(), ..Default::default() };
        assert_eq!(s.project_name(), "proj");
        let w = Session { cwd: "C:\\Users\\me\\proj\\".into(), ..Default::default() };
        assert_eq!(w.project_name(), "proj");
    }
}
